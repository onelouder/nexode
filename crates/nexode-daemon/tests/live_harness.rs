#![cfg(feature = "live-test")]

use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use nexode_daemon::engine::{DaemonConfig, run_daemon_with_listener};
use nexode_proto::hypervisor_client::HypervisorClient;
use nexode_proto::operator_command;
use nexode_proto::{CommandOutcome, MoveTask, OperatorCommand, StateRequest, TaskStatus};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{self, timeout};

#[tokio::test(flavor = "multi_thread")]
async fn live_claude_code_hello_world() {
    let Some(harness) = LiveHarness::claude_if_available() else {
        eprintln!("skipping live_claude_code_hello_world: missing `claude` or ANTHROPIC_API_KEY");
        return;
    };

    let result = run_live_task(harness, false).await;
    assert!(result.worktree_file_found);
    assert!(result.file_contents.contains("hello"));
    assert!(result.total_tokens > 0, "expected non-zero telemetry");
}

#[tokio::test(flavor = "multi_thread")]
async fn live_codex_cli_hello_world() {
    let Some(harness) = LiveHarness::codex_if_available() else {
        eprintln!("skipping live_codex_cli_hello_world: missing `codex` or OPENAI_API_KEY");
        return;
    };

    let result = run_live_task(harness, false).await;
    assert!(result.worktree_file_found);
    assert!(result.file_contents.contains("hello"));
    assert!(result.total_tokens > 0, "expected non-zero telemetry");
}

#[tokio::test(flavor = "multi_thread")]
async fn live_full_lifecycle() {
    let Some(harness) = LiveHarness::any_available() else {
        eprintln!("skipping live_full_lifecycle: no live harness prerequisites available");
        return;
    };

    let result = run_live_task(harness, true).await;
    assert!(result.repo_file_found);
    assert!(result.file_contents.contains("hello"));
}

struct LiveRunResult {
    worktree_file_found: bool,
    repo_file_found: bool,
    file_contents: String,
    total_tokens: u64,
}

#[derive(Clone, Copy)]
enum LiveHarness {
    ClaudeCode,
    CodexCli,
}

impl LiveHarness {
    fn claude_if_available() -> Option<Self> {
        (command_exists("claude") && has_non_empty_env("ANTHROPIC_API_KEY"))
            .then_some(Self::ClaudeCode)
    }

    fn codex_if_available() -> Option<Self> {
        (command_exists("codex") && has_non_empty_env("OPENAI_API_KEY")).then_some(Self::CodexCli)
    }

    fn any_available() -> Option<Self> {
        Self::claude_if_available().or_else(Self::codex_if_available)
    }

    fn model(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-sonnet-4-5",
            Self::CodexCli => "gpt-4.1",
        }
    }

    fn harness_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::CodexCli => "codex-cli",
        }
    }
}

async fn run_live_task(harness: LiveHarness, full_lifecycle: bool) -> LiveRunResult {
    let fixture = LiveFixture::new();
    let session_path = fixture.write_session(harness);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let config = fixture.config(session_path.clone());
    let server = tokio::spawn(async move {
        run_daemon_with_listener(config, listener, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let mut client = fixture.client(addr).await;
    let review_snapshot = wait_for_status(&mut client, "slot-a", TaskStatus::Review).await;
    let slot = review_snapshot.projects[0]
        .slots
        .iter()
        .find(|slot| slot.id == "slot-a")
        .expect("slot-a in snapshot");
    let worktree_root = PathBuf::from(&slot.worktree_id);
    let worktree_file = locate_generated_file(&worktree_root).expect("generated file in worktree");
    let worktree_file_found = true;
    let file_contents = fs::read_to_string(&worktree_file).expect("read generated file");

    if full_lifecycle {
        let response = client
            .dispatch_command(tonic::Request::new(OperatorCommand {
                command_id: "live-move".to_string(),
                action: Some(operator_command::Action::MoveTask(MoveTask {
                    task_id: "slot-a".to_string(),
                    target: TaskStatus::MergeQueue as i32,
                })),
            }))
            .await
            .expect("dispatch move task")
            .into_inner();
        assert!(response.success, "expected command execution");
        assert_eq!(response.outcome, CommandOutcome::Executed as i32);

        let done_snapshot = wait_for_status(&mut client, "slot-a", TaskStatus::Done).await;
        let repo_file_found = locate_generated_file(&fixture.repo).is_some();
        shutdown_tx.send(()).expect("signal shutdown");
        server
            .await
            .expect("join daemon task")
            .expect("daemon exits cleanly");

        return LiveRunResult {
            worktree_file_found,
            repo_file_found,
            file_contents,
            total_tokens: done_snapshot.projects[0].slots[0].total_tokens,
        };
    }

    shutdown_tx.send(()).expect("signal shutdown");
    server
        .await
        .expect("join daemon task")
        .expect("daemon exits cleanly");

    LiveRunResult {
        worktree_file_found,
        repo_file_found: false,
        file_contents,
        total_tokens: review_snapshot.projects[0].slots[0].total_tokens,
    }
}

struct LiveFixture {
    _tempdir: TempDir,
    root: PathBuf,
    repo: PathBuf,
    session_dir: PathBuf,
}

impl LiveFixture {
    fn new() -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let root = tempdir.path().to_path_buf();
        let repo = root.join("repo");
        fs::create_dir_all(&repo).expect("create repo");
        run_git(
            &root,
            ["init", "-b", "main", repo.to_str().expect("repo path")],
        );
        run_git(&repo, ["config", "user.email", "test@example.com"]);
        run_git(&repo, ["config", "user.name", "Test User"]);
        fs::write(repo.join("README.md"), "# Live Harness Test\n").expect("write readme");
        run_git(&repo, ["add", "."]);
        run_git(&repo, ["commit", "-m", "initial"]);

        let session_dir = root.join("session");
        fs::create_dir_all(&session_dir).expect("create session dir");

        Self {
            _tempdir: tempdir,
            root,
            repo,
            session_dir,
        }
    }

    fn write_session(&self, harness: LiveHarness) -> PathBuf {
        let session_path = self.session_dir.join("session.yaml");
        let repo_relative = self
            .repo
            .strip_prefix(&self.session_dir)
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "../repo".to_string());

        let contents = format!(
            r#"version: "2.0"
session:
  name: "live-harness"
defaults:
  model: "{model}"
  mode: "plan"
  timeout_minutes: 2
projects:
  - id: "project-1"
    repo: "{repo_relative}"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "{harness_name}"
        task: "Add a hello() function to hello.rs that returns the string 'Hello from Nexode'. Keep the change minimal and commit it."
"#,
            model = harness.model(),
            repo_relative = repo_relative,
            harness_name = harness.harness_name(),
        );
        fs::write(&session_path, contents).expect("write session file");
        session_path
    }

    fn config(&self, session_path: PathBuf) -> DaemonConfig {
        let mut config = DaemonConfig::new(session_path);
        config.tick_interval = Duration::from_millis(100);
        config.verification_timeout = Duration::from_secs(120);
        config.accounting_db_path = self.root.join("token-accounting.sqlite3");
        config
    }

    async fn client(&self, addr: SocketAddr) -> HypervisorClient<tonic::transport::Channel> {
        timeout(Duration::from_secs(10), async {
            loop {
                match HypervisorClient::connect(format!("http://{addr}")).await {
                    Ok(client) => return client,
                    Err(_) => time::sleep(Duration::from_millis(50)).await,
                }
            }
        })
        .await
        .expect("connect to daemon")
    }
}

async fn wait_for_status(
    client: &mut HypervisorClient<tonic::transport::Channel>,
    task_id: &str,
    expected: TaskStatus,
) -> nexode_proto::FullStateSnapshot {
    timeout(Duration::from_secs(120), async {
        loop {
            let snapshot = client
                .get_full_state(tonic::Request::new(StateRequest {}))
                .await
                .expect("get full state")
                .into_inner();
            if snapshot
                .task_dag
                .iter()
                .any(|task| task.id == task_id && task.status == expected as i32)
            {
                return snapshot;
            }
            time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .expect("task reaches expected state")
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn has_non_empty_env(name: &str) -> bool {
    matches!(env::var(name), Ok(value) if !value.trim().is_empty())
}

fn locate_generated_file(root: &Path) -> Option<PathBuf> {
    [root.join("hello.rs"), root.join("src/hello.rs")]
        .into_iter()
        .find(|path| path.exists())
}

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run git");
    if !output.status.success() {
        panic!(
            "git failed in {}:\nstdout:\n{}\nstderr:\n{}",
            cwd.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
