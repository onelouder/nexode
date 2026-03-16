use std::time::Duration;

use nexode_proto::hypervisor_server::Hypervisor;
use nexode_proto::observer_alert;
use nexode_proto::{MoveTask, ResumeSlot, SlotDispatch};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tokio_stream::StreamExt;

use crate::observer::{LoopAction, LoopDetectionConfig};

use super::test_support::{
    DaemonFixture, drive_engine_until, next_observer_alert, subscribe_events,
    wait_for_all_tasks_done, wait_for_status,
};
use super::*;

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn full_auto_slots_merge_through_fifo_queue() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "phase-0"
defaults:
  model: "codex"
  mode: "full_auto"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    verify:
      build: "test -d .nexode-mock"
      test: "find .nexode-mock -maxdepth 1 -type f | grep -q ."
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
      - id: "slot-b"
        harness: "mock"
        task: "Implement slot b"
      - id: "slot-c"
        harness: "mock"
        task: "Implement slot c"
"#,
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let config = fixture.config(session_path);
    let server = tokio::spawn(async move {
        run_daemon_with_listener(config, listener, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let mut client = fixture.client(addr).await;
    let snapshot = wait_for_all_tasks_done(&mut client).await;
    shutdown_tx.send(()).expect("signal shutdown");
    server
        .await
        .expect("join daemon task")
        .expect("daemon exits cleanly");

    assert_eq!(snapshot.task_dag.len(), 3);
    assert!(
        snapshot
            .task_dag
            .iter()
            .all(|task| task.status == TaskStatus::Done as i32)
    );
    assert!(fixture.repo.join(".nexode-mock/slot-a.txt").exists());
    assert!(fixture.repo.join(".nexode-mock/slot-b.txt").exists());
    assert!(fixture.repo.join(".nexode-mock/slot-c.txt").exists());
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn hard_budget_alert_archives_project_slots() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "budget"
defaults:
  model: "codex"
  mode: "full_auto"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    budget:
      max_usd: 0.25
      warn_usd: 0.1
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Run over budget"
"#,
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let config = fixture.config(session_path);
    let server = tokio::spawn(async move {
        run_daemon_with_listener(config, listener, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let mut client = fixture.client(addr).await;
    let snapshot = wait_for_status(&mut client, "slot-a", TaskStatus::Archived).await;
    shutdown_tx.send(()).expect("signal shutdown");
    server
        .await
        .expect("join daemon task")
        .expect("daemon exits cleanly");

    assert_eq!(snapshot.task_dag[0].status, TaskStatus::Archived as i32);
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn recovers_review_state_without_restarting_finished_slot() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "recovery"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
"#,
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let config = fixture.config(session_path.clone());
    let server = tokio::spawn(async move {
        run_daemon_with_listener(config, listener, std::future::pending::<()>()).await
    });

    let mut client = fixture.client(addr).await;
    let initial = wait_for_status(&mut client, "slot-a", TaskStatus::Review).await;
    let initial_agent_id = initial.task_dag[0].assigned_agent_id.clone();
    let initial_cost = initial.total_session_cost;

    server.abort();
    let _ = server.await;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let config = fixture.config(session_path);
    let server = tokio::spawn(async move {
        run_daemon_with_listener(config, listener, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let mut client = fixture.client(addr).await;
    let recovered = wait_for_status(&mut client, "slot-a", TaskStatus::Review).await;
    shutdown_tx.send(()).expect("signal shutdown");
    server
        .await
        .expect("join daemon task")
        .expect("daemon exits cleanly");

    assert_eq!(recovered.task_dag[0].assigned_agent_id, initial_agent_id);
    assert!((recovered.total_session_cost - initial_cost).abs() < f64::EPSILON);
}

#[tokio::test(flavor = "multi_thread")]
#[serial_test::serial]
async fn dispatch_command_returns_validated_outcomes() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "command-ack"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
"#,
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let config = fixture.config(session_path);
    let server = tokio::spawn(async move {
        run_daemon_with_listener(config, listener, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let mut client = fixture.client(addr).await;
    let snapshot = wait_for_status(&mut client, "slot-a", TaskStatus::Review).await;
    assert_eq!(snapshot.task_dag[0].status, TaskStatus::Review as i32);

    let invalid = client
        .dispatch_command(tonic::Request::new(OperatorCommand {
            command_id: "cmd-invalid".to_string(),
            action: Some(operator_command::Action::MoveTask(MoveTask {
                task_id: "slot-a".to_string(),
                target: TaskStatus::Done as i32,
            })),
        }))
        .await
        .expect("dispatch invalid transition")
        .into_inner();
    assert!(!invalid.success);
    assert_eq!(invalid.command_id, "cmd-invalid");
    assert_eq!(invalid.outcome, CommandOutcome::InvalidTransition as i32);

    let missing = client
        .dispatch_command(tonic::Request::new(OperatorCommand {
            command_id: "cmd-missing".to_string(),
            action: Some(operator_command::Action::SlotDispatch(SlotDispatch {
                slot_id: "slot-xyz".to_string(),
                raw_nl: "do the thing".to_string(),
            })),
        }))
        .await
        .expect("dispatch missing slot")
        .into_inner();
    assert!(!missing.success);
    assert_eq!(missing.command_id, "cmd-missing");
    assert_eq!(missing.outcome, CommandOutcome::SlotNotFound as i32);

    let executed = client
        .dispatch_command(tonic::Request::new(OperatorCommand {
            command_id: "cmd-executed".to_string(),
            action: Some(operator_command::Action::MoveTask(MoveTask {
                task_id: "slot-a".to_string(),
                target: TaskStatus::MergeQueue as i32,
            })),
        }))
        .await
        .expect("dispatch valid move")
        .into_inner();
    assert!(executed.success);
    assert_eq!(executed.command_id, "cmd-executed");
    assert_eq!(executed.outcome, CommandOutcome::Executed as i32);

    let done = wait_for_status(&mut client, "slot-a", TaskStatus::Done).await;
    shutdown_tx.send(()).expect("signal shutdown");
    server
        .await
        .expect("join daemon task")
        .expect("daemon exits cleanly");

    assert_eq!(done.task_dag[0].status, TaskStatus::Done as i32);
}

#[tokio::test(flavor = "multi_thread")]
async fn slot_agent_swapped_emits_executing_event() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "swap-events"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
"#,
    );

    let session = load_session_config(&session_path).expect("load session");
    let state =
        RuntimeState::from_session(session, Duration::from_secs(5)).expect("create runtime state");
    let bridge = GrpcBridge::new(state.snapshot());
    let (service, command_rx) = bridge.into_parts();
    let accounting =
        TokenAccountingHandle::start(fixture.root.join("command-ack.sqlite3")).expect("accounting");
    let wal = Wal::open(fixture.root.join(".nexode/wal.binlog")).expect("open wal");
    let (process_tx, process_rx) = mpsc::unbounded_channel();

    let mut engine = DaemonEngine {
        config: fixture.config(session_path),
        service: service.clone(),
        command_rx,
        process_rx,
        process_tx,
        process_manager: AgentProcessManager::new(),
        accounting,
        wal,
        daemon_instance_id: "test-daemon".to_string(),
        state,
        loop_detector: LoopDetector::new(LoopDetectionConfig::default()),
        sandbox_guard: SandboxGuard::new(true),
    };
    if let Some(slot) = engine.slot_mut("slot-a") {
        slot.task_status = TaskStatus::Working;
        slot.current_agent_id = Some("old-agent".to_string());
    }

    let mut stream = Hypervisor::subscribe_events(
        &service,
        tonic::Request::new(nexode_proto::SubscribeRequest {
            client_version: "test-client".to_string(),
        }),
    )
    .await
    .expect("subscribe events")
    .into_inner();

    engine
        .handle_process_event(AgentProcessEvent::SlotAgentSwapped(SlotAgentSwapped {
            slot_id: "slot-a".to_string(),
            old_agent_id: "old-agent".to_string(),
            new_agent_id: "new-agent".to_string(),
            reason: "crash_recovery".to_string(),
        }))
        .await
        .expect("handle swap event");

    let mut saw_swap = false;
    let mut saw_executing = false;
    for _ in 0..2 {
        let event = timeout(Duration::from_secs(2), stream.next())
            .await
            .expect("event before timeout")
            .expect("event payload")
            .expect("stream response");
        match event.payload {
            Some(hypervisor_event::Payload::SlotAgentSwapped(payload)) => {
                saw_swap = payload.slot_id == "slot-a" && payload.new_agent_id == "new-agent";
            }
            Some(hypervisor_event::Payload::AgentStateChanged(payload)) => {
                saw_executing = payload.agent_id == "new-agent"
                    && payload.new_state == AgentState::Executing as i32;
            }
            _ => {}
        }
    }

    assert!(saw_swap);
    assert!(saw_executing);
}

#[tokio::test(flavor = "multi_thread")]
async fn observer_loop_kill_stops_agent_and_pauses_slot() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "observer-loop"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "[[mock-loop]]"
"#,
    );

    let mut config = fixture.config(session_path.clone());
    config.observer.loop_detection.max_identical_outputs = 3;
    config.observer.loop_detection.on_loop = LoopAction::Kill;

    let (mut engine, service) = fixture.engine(session_path, config).await;
    let mut stream = subscribe_events(&service).await;

    engine.start_slot("slot-a").await.expect("start slot");
    drive_engine_until(&mut engine, |engine| {
        engine.current_task_status("slot-a") == Some(TaskStatus::Paused)
    })
    .await;

    assert_eq!(
        engine.current_task_status("slot-a"),
        Some(TaskStatus::Paused)
    );
    assert!(
        engine
            .slot_mut("slot-a")
            .expect("slot runtime")
            .supervisor
            .is_none()
    );

    let alert = next_observer_alert(&mut stream).await;
    match alert.detail.expect("alert detail") {
        observer_alert::Detail::LoopDetected(detail) => {
            assert_eq!(alert.slot_id, "slot-a");
            assert_eq!(detail.intervention, ObserverIntervention::Kill as i32);
        }
        other => panic!("expected loop alert, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn uncertainty_signal_pauses_slot_and_emits_alert() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "observer-uncertainty"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "[[mock-uncertain]]"
"#,
    );

    let config = fixture.config(session_path.clone());
    let (mut engine, service) = fixture.engine(session_path, config).await;
    let mut stream = subscribe_events(&service).await;

    engine.start_slot("slot-a").await.expect("start slot");
    drive_engine_until(&mut engine, |engine| {
        engine.current_task_status("slot-a") == Some(TaskStatus::Paused)
    })
    .await;

    assert_eq!(
        engine.current_task_status("slot-a"),
        Some(TaskStatus::Paused)
    );
    let alert = next_observer_alert(&mut stream).await;
    match alert.detail.expect("alert detail") {
        observer_alert::Detail::UncertaintySignal(detail) => {
            assert!(detail.reason.contains("DECISION:"));
        }
        other => panic!("expected uncertainty alert, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn observer_pause_can_resume_back_to_working() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "observer-resume"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "[[mock-loop]]"
"#,
    );

    let mut config = fixture.config(session_path.clone());
    config.observer.loop_detection.max_identical_outputs = 3;
    config.observer.loop_detection.on_loop = LoopAction::Pause;

    let (mut engine, _service) = fixture.engine(session_path, config).await;

    engine.start_slot("slot-a").await.expect("start slot");
    drive_engine_until(&mut engine, |engine| {
        engine.current_task_status("slot-a") == Some(TaskStatus::Paused)
    })
    .await;

    if let Some(slot) = engine.slot_mut("slot-a") {
        slot.task = "Implement slot a".to_string();
    }

    let response = engine
        .handle_command(OperatorCommand {
            command_id: "resume-slot".to_string(),
            action: Some(operator_command::Action::ResumeSlot(ResumeSlot {
                slot_id: "slot-a".to_string(),
                instruction: String::new(),
            })),
        })
        .await
        .expect("resume paused slot");
    assert!(response.success);
    assert_eq!(
        engine.current_task_status("slot-a"),
        Some(TaskStatus::Working)
    );

    drive_engine_until(&mut engine, |engine| {
        engine.current_task_status("slot-a") == Some(TaskStatus::Review)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sandbox_violation_pauses_slot_before_merge() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "observer-sandbox"
defaults:
  model: "mock"
  mode: "full_auto"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "[[mock-outside-write]]"
"#,
    );

    let config = fixture.config(session_path.clone());
    let (mut engine, service) = fixture.engine(session_path, config).await;
    let mut stream = subscribe_events(&service).await;

    engine.start_slot("slot-a").await.expect("start slot");
    drive_engine_until(&mut engine, |engine| {
        engine.current_task_status("slot-a") == Some(TaskStatus::Paused)
    })
    .await;

    assert_eq!(
        engine.current_task_status("slot-a"),
        Some(TaskStatus::Paused)
    );
    let alert = next_observer_alert(&mut stream).await;
    match alert.detail.expect("alert detail") {
        observer_alert::Detail::SandboxViolation(detail) => {
            assert_eq!(detail.path, "../../../etc/shadow");
        }
        other => panic!("expected sandbox alert, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn event_sequences_are_monotonic_and_snapshot_tracks_latest() {
    let fixture = DaemonFixture::new();
    let session_path = fixture.write_session(
        r#"
version: "2.0"
session:
  name: "event-sequence"
defaults:
  model: "mock"
  mode: "plan"
  timeout_minutes: 1
projects:
  - id: "project-1"
    repo: "./repo"
    display_name: "Project One"
    slots:
      - id: "slot-a"
        harness: "mock"
        task: "Implement slot a"
"#,
    );

    let config = fixture.config(session_path.clone());
    let (mut engine, service) = fixture.engine(session_path, config).await;
    let mut stream = subscribe_events(&service).await;

    engine.publish_event(
        hypervisor_event::Payload::AgentStateChanged(AgentStateChanged {
            agent_id: "agent-1".to_string(),
            new_state: AgentState::Executing as i32,
            slot_id: "slot-a".to_string(),
        }),
        None,
    );
    engine.publish_event(
        hypervisor_event::Payload::TaskStatusChanged(TaskStatusChanged {
            task_id: "slot-a".to_string(),
            new_status: TaskStatus::Working as i32,
            agent_id: "agent-1".to_string(),
        }),
        None,
    );
    engine.sync_snapshot().await;

    let first = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("first event before timeout")
        .expect("stream item")
        .expect("event");
    let second = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("second event before timeout")
        .expect("stream item")
        .expect("event");
    let snapshot = service.full_state().await;

    assert_eq!(first.event_sequence, 1);
    assert_eq!(second.event_sequence, 2);
    assert_eq!(snapshot.last_event_sequence, 2);
}
