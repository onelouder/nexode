use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nexode_proto::ObserverAlert;
use nexode_proto::hypervisor_client::HypervisorClient;
use nexode_proto::hypervisor_server::Hypervisor;
use tempfile::TempDir;
use tokio::time::timeout;
use tokio_stream::StreamExt;

use super::config::DEFAULT_TARGET_BRANCH;
use super::*;

pub(super) async fn wait_for_all_tasks_done(
    client: &mut HypervisorClient<tonic::transport::Channel>,
) -> FullStateSnapshot {
    let deadline = time::Instant::now() + Duration::from_secs(10);

    loop {
        let snapshot = client
            .get_full_state(tonic::Request::new(nexode_proto::StateRequest {}))
            .await
            .expect("get full state")
            .into_inner();
        let done = !snapshot.task_dag.is_empty()
            && snapshot
                .task_dag
                .iter()
                .all(|task| task.status == TaskStatus::Done as i32);
        if done {
            return snapshot;
        }
        if time::Instant::now() >= deadline {
            panic!("tasks did not reach done state: {:?}", snapshot);
        }
        time::sleep(Duration::from_millis(100)).await;
    }
}

pub(super) async fn wait_for_status(
    client: &mut HypervisorClient<tonic::transport::Channel>,
    task_id: &str,
    expected: TaskStatus,
) -> FullStateSnapshot {
    timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = client
                .get_full_state(tonic::Request::new(nexode_proto::StateRequest {}))
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
            time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("task reaches expected state")
}

pub(super) async fn subscribe_events(
    service: &HypervisorService,
) -> <HypervisorService as Hypervisor>::SubscribeEventsStream {
    Hypervisor::subscribe_events(
        service,
        tonic::Request::new(nexode_proto::SubscribeRequest {
            client_version: "test-client".to_string(),
        }),
    )
    .await
    .expect("subscribe events")
    .into_inner()
}

pub(super) async fn next_observer_alert(
    stream: &mut <HypervisorService as Hypervisor>::SubscribeEventsStream,
) -> ObserverAlert {
    timeout(Duration::from_secs(2), async {
        loop {
            let event = stream.next().await.expect("stream item").expect("event");
            if let Some(hypervisor_event::Payload::ObserverAlert(alert)) = event.payload {
                return alert;
            }
        }
    })
    .await
    .expect("observer alert before timeout")
}

pub(super) async fn drive_engine_until<F>(engine: &mut DaemonEngine, mut done: F)
where
    F: FnMut(&DaemonEngine) -> bool,
{
    let deadline = time::Instant::now() + Duration::from_secs(3);
    while !done(engine) {
        if let Ok(Some(event)) = timeout(Duration::from_millis(50), engine.process_rx.recv()).await
        {
            engine
                .handle_process_event(event)
                .await
                .expect("handle process event");
        }
        engine.run_observer_tick().await.expect("observer tick");
        if time::Instant::now() >= deadline {
            panic!("engine did not reach expected state");
        }
    }
}

pub(super) struct DaemonFixture {
    _tempdir: TempDir,
    pub(super) root: PathBuf,
    pub(super) repo: PathBuf,
    session_dir: PathBuf,
}

impl DaemonFixture {
    pub(super) fn new() -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let root = tempdir.path().to_path_buf();
        let repo = root.join("repo");
        fs::create_dir_all(&repo).expect("create repo directory");
        run_git(
            &root,
            ["init", "-b", DEFAULT_TARGET_BRANCH, repo.to_str().unwrap()],
        );
        run_git(&repo, ["config", "user.email", "test@example.com"]);
        run_git(&repo, ["config", "user.name", "Test User"]);
        fs::write(repo.join("README.md"), "base\n").expect("write base file");
        run_git(&repo, ["add", "."]);
        run_git(&repo, ["commit", "-m", "initial"]);

        let session_dir = root.join("session");
        fs::create_dir_all(&session_dir).expect("create session directory");

        Self {
            _tempdir: tempdir,
            root,
            repo,
            session_dir,
        }
    }

    pub(super) fn write_session(&self, contents: &str) -> PathBuf {
        let session_path = self.session_dir.join("session.yaml");
        let repo_relative = self
            .repo
            .strip_prefix(&self.session_dir)
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "../repo".to_string());
        let contents = contents.replace("./repo", &repo_relative);
        fs::write(&session_path, contents).expect("write session config");
        session_path
    }

    pub(super) fn config(&self, session_path: PathBuf) -> DaemonConfig {
        let mut config = DaemonConfig::new(session_path);
        config.tick_interval = Duration::from_millis(50);
        config.verification_timeout = Duration::from_secs(5);
        config.accounting_db_path = self.root.join("token-accounting.sqlite3");
        config
    }

    pub(super) async fn engine(
        &self,
        session_path: PathBuf,
        config: DaemonConfig,
    ) -> (DaemonEngine, HypervisorService) {
        let session = load_session_config(&session_path).expect("load session");
        let state = RuntimeState::from_session(session, config.verification_timeout)
            .expect("create runtime state");
        let bridge = GrpcBridge::new(state.snapshot());
        let (service, command_rx) = bridge.into_parts();
        let accounting = TokenAccountingHandle::start(self.root.join("engine-test.sqlite3"))
            .expect("accounting");
        let wal = Wal::open(self.root.join(".nexode/engine-test.wal")).expect("open wal");
        let (process_tx, process_rx) = mpsc::unbounded_channel();

        (
            DaemonEngine {
                loop_detector: LoopDetector::new(config.observer.loop_detection.clone()),
                sandbox_guard: SandboxGuard::new(config.observer.sandbox_enforcement),
                config,
                service: service.clone(),
                command_rx,
                process_rx,
                process_tx,
                process_manager: AgentProcessManager::new(),
                accounting,
                wal,
                daemon_instance_id: "test-daemon".to_string(),
                state,
            },
            service,
        )
    }

    pub(super) async fn client(
        &self,
        addr: SocketAddr,
    ) -> HypervisorClient<tonic::transport::Channel> {
        timeout(Duration::from_secs(5), async {
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

pub(super) fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = std::process::Command::new("git")
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
