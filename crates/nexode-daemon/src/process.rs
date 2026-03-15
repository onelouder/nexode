use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use nexode_proto::SlotAgentSwapped;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{self, Instant};

use crate::harness::AgentHarness;

static AGENT_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct AgentProcessSpec {
    pub slot_id: String,
    pub agent_id_prefix: Option<String>,
    pub worktree_path: PathBuf,
    pub command: AgentCommand,
    pub harness: Arc<dyn AgentHarness>,
    pub watchdog_timeout: Duration,
    pub watchdog_poll_interval: Duration,
    pub respawn_on_failure: bool,
    pub max_restarts: usize,
}

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub env: BTreeMap<String, String>,
    pub setup_files: Vec<SetupFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupFile {
    pub relative_path: PathBuf,
    pub content: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentProcessEvent {
    Spawned {
        slot_id: String,
        agent_id: String,
        pid: Option<u32>,
    },
    Output {
        slot_id: String,
        agent_id: String,
        stream: OutputStream,
        line: String,
        telemetry: Option<ParsedTelemetry>,
    },
    Exited {
        slot_id: String,
        agent_id: String,
        status_code: Option<i32>,
        success: bool,
    },
    TimedOut {
        slot_id: String,
        agent_id: String,
        timeout: Duration,
    },
    SlotAgentSwapped(SlotAgentSwapped),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTelemetry {
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    pub cost_usd: Option<f64>,
}

#[derive(Debug)]
pub struct SlotSupervisor {
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

#[derive(Debug)]
pub struct AgentProcessManager;

#[derive(Debug, Error)]
pub enum AgentProcessError {
    #[error("invalid process spec for slot `{slot_id}`: {message}")]
    InvalidSpec { slot_id: String, message: String },
    #[error("failed to spawn agent for slot `{slot_id}`: {source}")]
    Spawn {
        slot_id: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to prepare setup file `{path}` for slot `{slot_id}`: {source}")]
    SetupFile {
        slot_id: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to await slot supervisor: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl AgentCommand {
    pub fn new(
        program: impl Into<OsString>,
        args: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            env: BTreeMap::new(),
            setup_files: Vec::new(),
        }
    }

    pub fn shell(script: impl AsRef<str>) -> Self {
        Self::new(
            "sh",
            vec![OsString::from("-lc"), OsString::from(script.as_ref())],
        )
    }
}

impl AgentProcessManager {
    pub fn new() -> Self {
        Self
    }

    pub fn spawn_slot(
        &self,
        spec: AgentProcessSpec,
        event_tx: mpsc::UnboundedSender<AgentProcessEvent>,
    ) -> Result<SlotSupervisor, AgentProcessError> {
        validate_spec(&spec)?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            supervise_slot(spec, event_tx, shutdown_rx).await;
        });

        Ok(SlotSupervisor {
            shutdown_tx: Some(shutdown_tx),
            task,
        })
    }
}

impl SlotSupervisor {
    pub async fn wait(self) -> Result<(), AgentProcessError> {
        self.task.await?;
        Ok(())
    }

    pub async fn shutdown(mut self) -> Result<(), AgentProcessError> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.task.await?;
        Ok(())
    }
}

async fn supervise_slot(
    spec: AgentProcessSpec,
    event_tx: mpsc::UnboundedSender<AgentProcessEvent>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let mut restart_count = 0usize;
    let mut pending_swap: Option<(String, &'static str)> = None;

    loop {
        let agent_id = next_agent_id(spec.agent_id_prefix.as_deref(), &spec.slot_id);
        if let Some((old_agent_id, reason)) = pending_swap.take() {
            let _ = event_tx.send(AgentProcessEvent::SlotAgentSwapped(SlotAgentSwapped {
                slot_id: spec.slot_id.clone(),
                old_agent_id,
                new_agent_id: agent_id.clone(),
                reason: reason.to_string(),
            }));
        }

        let outcome = match run_single_agent(&spec, &agent_id, &event_tx, &mut shutdown_rx).await {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = event_tx.send(AgentProcessEvent::Output {
                    slot_id: spec.slot_id.clone(),
                    agent_id: agent_id.clone(),
                    stream: OutputStream::Stderr,
                    line: format!("spawn error: {error}"),
                    telemetry: None,
                });
                break;
            }
        };

        match outcome {
            AgentOutcome::Completed {
                status_code,
                success,
            } => {
                let _ = event_tx.send(AgentProcessEvent::Exited {
                    slot_id: spec.slot_id.clone(),
                    agent_id: agent_id.clone(),
                    status_code,
                    success,
                });

                if success {
                    break;
                }

                if spec.respawn_on_failure && restart_count < spec.max_restarts {
                    restart_count += 1;
                    pending_swap = Some((agent_id, "crash_recovery"));
                    continue;
                }

                break;
            }
            AgentOutcome::TimedOut => {
                let _ = event_tx.send(AgentProcessEvent::TimedOut {
                    slot_id: spec.slot_id.clone(),
                    agent_id: agent_id.clone(),
                    timeout: spec.watchdog_timeout,
                });
                let _ = event_tx.send(AgentProcessEvent::Exited {
                    slot_id: spec.slot_id.clone(),
                    agent_id: agent_id.clone(),
                    status_code: None,
                    success: false,
                });

                if spec.respawn_on_failure && restart_count < spec.max_restarts {
                    restart_count += 1;
                    pending_swap = Some((agent_id, "crash_recovery"));
                    continue;
                }

                break;
            }
            AgentOutcome::Shutdown => break,
        }
    }
}

async fn run_single_agent(
    spec: &AgentProcessSpec,
    agent_id: &str,
    event_tx: &mpsc::UnboundedSender<AgentProcessEvent>,
    shutdown_rx: &mut oneshot::Receiver<()>,
) -> Result<AgentOutcome, AgentProcessError> {
    write_setup_files(&spec.slot_id, &spec.worktree_path, &spec.command.setup_files)?;

    let mut command = Command::new(&spec.command.program);
    command
        .args(&spec.command.args)
        .envs(&spec.command.env)
        .current_dir(&spec.worktree_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|source| AgentProcessError::Spawn {
        slot_id: spec.slot_id.clone(),
        source,
    })?;

    let _ = event_tx.send(AgentProcessEvent::Spawned {
        slot_id: spec.slot_id.clone(),
        agent_id: agent_id.to_string(),
        pid: child.id(),
    });

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (line_tx, mut line_rx) = mpsc::unbounded_channel();

    if let Some(stdout) = stdout {
        tokio::spawn(read_lines(stdout, OutputStream::Stdout, line_tx.clone()));
    }
    if let Some(stderr) = stderr {
        tokio::spawn(read_lines(stderr, OutputStream::Stderr, line_tx));
    }

    let mut last_output = Instant::now();
    let mut completion_detected = false;
    let mut tick = time::interval(spec.watchdog_poll_interval);
    loop {
        tokio::select! {
            _ = &mut *shutdown_rx => {
                let _ = child.kill().await;
                return Ok(AgentOutcome::Shutdown);
            }
            Some(line_event) = line_rx.recv() => {
                last_output = Instant::now();
                completion_detected |= spec.harness.detect_completion(&line_event.line);
                let _ = event_tx.send(AgentProcessEvent::Output {
                    slot_id: spec.slot_id.clone(),
                    agent_id: agent_id.to_string(),
                    stream: line_event.stream,
                    telemetry: spec.harness.parse_telemetry(&line_event.line),
                    line: line_event.line,
                });
            }
            _ = tick.tick() => {
                if let Some(status) = child.try_wait().map_err(|source| AgentProcessError::Spawn {
                    slot_id: spec.slot_id.clone(),
                    source,
                })? {
                    return Ok(AgentOutcome::Completed {
                        status_code: status.code(),
                        success: status.success() || completion_detected,
                    });
                }
                if last_output.elapsed() >= spec.watchdog_timeout {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    return Ok(AgentOutcome::TimedOut);
                }
            }
        }
    }
}

async fn read_lines<R>(reader: R, stream: OutputStream, line_tx: mpsc::UnboundedSender<LineEvent>)
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let _ = line_tx.send(LineEvent { stream, line });
    }
}

fn validate_spec(spec: &AgentProcessSpec) -> Result<(), AgentProcessError> {
    if spec.watchdog_timeout.is_zero() {
        return Err(AgentProcessError::InvalidSpec {
            slot_id: spec.slot_id.clone(),
            message: "watchdog_timeout must be greater than zero".to_string(),
        });
    }
    if spec.watchdog_poll_interval.is_zero() {
        return Err(AgentProcessError::InvalidSpec {
            slot_id: spec.slot_id.clone(),
            message: "watchdog_poll_interval must be greater than zero".to_string(),
        });
    }
    if !spec.worktree_path.exists() {
        return Err(AgentProcessError::InvalidSpec {
            slot_id: spec.slot_id.clone(),
            message: format!(
                "worktree path `{}` does not exist",
                spec.worktree_path.display()
            ),
        });
    }
    Ok(())
}

#[derive(Debug)]
struct LineEvent {
    stream: OutputStream,
    line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AgentOutcome {
    Completed {
        status_code: Option<i32>,
        success: bool,
    },
    TimedOut,
    Shutdown,
}

impl ParsedTelemetry {
    pub fn parse(line: &str) -> Option<Self> {
        if let Some(line) = line.strip_prefix("TOKENS ") {
            return parse_space_delimited(line);
        }
        if let Some(line) = line.strip_prefix("NEXODE_TELEMETRY:") {
            return parse_csv(line);
        }
        None
    }
}

fn parse_space_delimited(line: &str) -> Option<ParsedTelemetry> {
    let mut telemetry = ParsedTelemetry {
        tokens_in: None,
        tokens_out: None,
        cost_usd: None,
    };

    for part in line.split_whitespace() {
        let (key, value) = part.split_once('=')?;
        match key {
            "in" => telemetry.tokens_in = value.parse().ok(),
            "out" => telemetry.tokens_out = value.parse().ok(),
            "cost" => telemetry.cost_usd = value.parse().ok(),
            _ => {}
        }
    }

    Some(telemetry)
}

fn parse_csv(line: &str) -> Option<ParsedTelemetry> {
    let mut telemetry = ParsedTelemetry {
        tokens_in: None,
        tokens_out: None,
        cost_usd: None,
    };

    for part in line.split(',') {
        let (key, value) = part.split_once('=')?;
        match key {
            "tokens_in" => telemetry.tokens_in = value.parse().ok(),
            "tokens_out" => telemetry.tokens_out = value.parse().ok(),
            "cost_usd" => telemetry.cost_usd = value.parse().ok(),
            _ => {}
        }
    }

    Some(telemetry)
}

fn write_setup_files(
    slot_id: &str,
    worktree_path: &PathBuf,
    setup_files: &[SetupFile],
) -> Result<(), AgentProcessError> {
    for setup_file in setup_files {
        let path = worktree_path.join(&setup_file.relative_path);
        if path.exists() && !setup_file.overwrite {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| AgentProcessError::SetupFile {
                slot_id: slot_id.to_string(),
                path: path.clone(),
                source,
            })?;
        }
        fs::write(&path, &setup_file.content).map_err(|source| AgentProcessError::SetupFile {
            slot_id: slot_id.to_string(),
            path,
            source,
        })?;
    }
    Ok(())
}

fn next_agent_id(prefix: Option<&str>, slot_id: &str) -> String {
    let id = AGENT_COUNTER.fetch_add(1, Ordering::Relaxed);
    match prefix {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}-{slot_id}-agent-{id}"),
        _ => format!("{slot_id}-agent-{id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::harness::MockHarness;
    use tempfile::TempDir;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    #[tokio::test(flavor = "multi_thread")]
    async fn streams_output_and_parses_mock_telemetry() {
        let fixture = ProcessFixture::new();
        let manager = AgentProcessManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let script = r#"
echo "boot"
echo "TOKENS in=12 out=3 cost=0.75"
echo "done" >&2
"#;

        let supervisor = manager
            .spawn_slot(fixture.spec(AgentCommand::shell(script), false, 0), tx)
            .expect("spawn slot");

        let events = collect_until_success_exit(&mut rx).await;
        supervisor.wait().await.expect("wait for supervisor");

        assert!(matches!(events[0], AgentProcessEvent::Spawned { .. }));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentProcessEvent::Output { line, telemetry, .. }
                if line == "TOKENS in=12 out=3 cost=0.75"
                    && telemetry.as_ref() == Some(&ParsedTelemetry {
                        tokens_in: Some(12),
                        tokens_out: Some(3),
                        cost_usd: Some(0.75),
                    })
        )));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, AgentProcessEvent::Exited { success: true, .. }))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn respawns_after_crash_into_same_slot() {
        let fixture = ProcessFixture::new();
        let manager = AgentProcessManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let script = r#"
if [ ! -f .crash-once ]; then
  touch .crash-once
  echo "first run crash" >&2
  exit 1
fi
echo "recovered"
"#;

        let supervisor = manager
            .spawn_slot(fixture.spec(AgentCommand::shell(script), true, 1), tx)
            .expect("spawn slot");

        let events = collect_until_success_exit(&mut rx).await;
        supervisor.wait().await.expect("wait for supervisor");

        assert!(events.iter().any(|event| matches!(
            event,
            AgentProcessEvent::SlotAgentSwapped(swapped)
                if swapped.slot_id == fixture.slot_id
                    && swapped.reason == "crash_recovery"
        )));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, AgentProcessEvent::Exited { success: false, .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, AgentProcessEvent::Exited { success: true, .. }))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn watchdog_kills_quiet_process_and_respawns() {
        let fixture = ProcessFixture::new();
        let manager = AgentProcessManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let script = r#"
if [ ! -f .timeout-once ]; then
  touch .timeout-once
  sleep 2
  exit 0
fi
echo "after-timeout"
"#;

        let supervisor = manager
            .spawn_slot(
                fixture.spec_with_timeout(
                    AgentCommand::shell(script),
                    true,
                    1,
                    Duration::from_millis(150),
                ),
                tx,
            )
            .expect("spawn slot");

        let events = collect_until_success_exit(&mut rx).await;
        supervisor.wait().await.expect("wait for supervisor");

        assert!(
            events
                .iter()
                .any(|event| matches!(event, AgentProcessEvent::TimedOut { .. }))
        );
        assert!(events.iter().any(|event| matches!(
            event,
            AgentProcessEvent::SlotAgentSwapped(swapped) if swapped.reason == "crash_recovery"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentProcessEvent::Output { line, .. } if line == "after-timeout"
        )));
    }

    async fn collect_until_success_exit(
        rx: &mut mpsc::UnboundedReceiver<AgentProcessEvent>,
    ) -> Vec<AgentProcessEvent> {
        let mut events = Vec::new();
        loop {
            let event = timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("receive event before timeout")
                .expect("event");
            let done = matches!(event, AgentProcessEvent::Exited { success: true, .. });
            events.push(event);
            if done {
                return events;
            }
        }
    }

    struct ProcessFixture {
        _tempdir: TempDir,
        worktree: PathBuf,
        slot_id: String,
    }

    impl ProcessFixture {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("tempdir");
            let worktree = tempdir.path().join("worktree");
            fs::create_dir_all(&worktree).expect("create worktree");

            Self {
                _tempdir: tempdir,
                worktree,
                slot_id: "slot-a".to_string(),
            }
        }

        fn spec(
            &self,
            command: AgentCommand,
            respawn_on_failure: bool,
            max_restarts: usize,
        ) -> AgentProcessSpec {
            self.spec_with_timeout(
                command,
                respawn_on_failure,
                max_restarts,
                Duration::from_secs(5),
            )
        }

        fn spec_with_timeout(
            &self,
            command: AgentCommand,
            respawn_on_failure: bool,
            max_restarts: usize,
            watchdog_timeout: Duration,
        ) -> AgentProcessSpec {
            AgentProcessSpec {
                slot_id: self.slot_id.clone(),
                agent_id_prefix: None,
                worktree_path: self.worktree.clone(),
                command,
                harness: Arc::new(MockHarness),
                watchdog_timeout,
                watchdog_poll_interval: Duration::from_millis(25),
                respawn_on_failure,
                max_restarts,
            }
        }
    }
}
