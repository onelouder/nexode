use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use nexode_proto::hypervisor_server::{Hypervisor, HypervisorServer};
use nexode_proto::{
    CommandOutcome, CommandResponse, FullStateSnapshot, HypervisorEvent, OperatorCommand,
    StateRequest, SubscribeRequest,
};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::{BroadcastStream, TcpListenerStream};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[cfg(test)]
const COMMAND_RESPONSE_TIMEOUT: Duration = Duration::from_millis(50);
#[cfg(not(test))]
const COMMAND_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

pub type CommandEnvelope = (OperatorCommand, oneshot::Sender<CommandResponse>);
pub type CommandReceiver = mpsc::UnboundedReceiver<CommandEnvelope>;

#[derive(Debug)]
pub struct GrpcBridge {
    service: HypervisorService,
    command_rx: CommandReceiver,
}

#[derive(Debug, Clone)]
pub struct HypervisorService {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    event_tx: broadcast::Sender<HypervisorEvent>,
    command_tx: mpsc::UnboundedSender<CommandEnvelope>,
    state: RwLock<FullStateSnapshot>,
}

type EventStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<HypervisorEvent, Status>> + Send + 'static>>;

impl GrpcBridge {
    pub fn new(initial_state: FullStateSnapshot) -> Self {
        Self::with_event_buffer(initial_state, 256)
    }

    pub fn with_event_buffer(initial_state: FullStateSnapshot, event_buffer: usize) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, _) = broadcast::channel(event_buffer);

        let service = HypervisorService {
            inner: Arc::new(Inner {
                event_tx,
                command_tx,
                state: RwLock::new(initial_state),
            }),
        };

        Self {
            service,
            command_rx,
        }
    }

    pub fn service(&self) -> HypervisorService {
        self.service.clone()
    }

    pub fn into_parts(self) -> (HypervisorService, CommandReceiver) {
        (self.service, self.command_rx)
    }
}

impl HypervisorService {
    pub async fn set_full_state(&self, snapshot: FullStateSnapshot) {
        *self.inner.state.write().await = snapshot;
    }

    pub async fn full_state(&self) -> FullStateSnapshot {
        self.inner.state.read().await.clone()
    }

    pub fn publish_event(&self, event: HypervisorEvent) -> usize {
        self.inner.event_tx.send(event).unwrap_or_default()
    }

    pub async fn serve_tcp(
        self,
        listener: TcpListener,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<(), tonic::transport::Error> {
        Server::builder()
            .add_service(HypervisorServer::new(self))
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
            .await
    }
}

#[tonic::async_trait]
impl Hypervisor for HypervisorService {
    type SubscribeEventsStream = EventStream;

    async fn subscribe_events(
        &self,
        _request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let stream =
            BroadcastStream::new(self.inner.event_tx.subscribe()).filter_map(
                |result| match result {
                    Ok(event) => Some(Ok(event)),
                    Err(BroadcastStreamRecvError::Lagged(skipped)) => Some(Err(Status::data_loss(
                        format!("event stream lagged by {skipped} messages"),
                    ))),
                },
            );

        Ok(Response::new(Box::pin(stream)))
    }

    async fn dispatch_command(
        &self,
        request: Request<OperatorCommand>,
    ) -> Result<Response<CommandResponse>, Status> {
        let command = request.into_inner();
        let command_id = command.command_id.clone();
        let (response_tx, response_rx) = oneshot::channel();
        if self.inner.command_tx.send((command, response_tx)).is_err() {
            return Ok(Response::new(command_error_response(
                &command_id,
                "Engine command channel is closed",
                CommandOutcome::Unspecified,
            )));
        }

        let response = match tokio::time::timeout(COMMAND_RESPONSE_TIMEOUT, response_rx).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => command_error_response(
                &command_id,
                "Engine dropped command response channel",
                CommandOutcome::Unspecified,
            ),
            Err(_) => command_error_response(
                &command_id,
                "Engine did not respond within timeout",
                CommandOutcome::Unspecified,
            ),
        };

        Ok(Response::new(response))
    }

    async fn get_full_state(
        &self,
        _request: Request<StateRequest>,
    ) -> Result<Response<FullStateSnapshot>, Status> {
        Ok(Response::new(self.full_state().await))
    }
}

fn command_error_response(
    command_id: &str,
    error_message: impl Into<String>,
    outcome: CommandOutcome,
) -> CommandResponse {
    CommandResponse {
        success: false,
        error_message: error_message.into(),
        command_id: command_id.to_string(),
        outcome: outcome as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::time::Duration;

    use nexode_proto::hypervisor_client::HypervisorClient;
    use nexode_proto::hypervisor_event;
    use nexode_proto::{AgentState, AgentStateChanged, Project};
    use tokio::sync::oneshot;
    use tokio::time::timeout;

    #[tokio::test(flavor = "multi_thread")]
    async fn get_full_state_returns_the_current_snapshot() {
        let harness = GrpcHarness::new(FullStateSnapshot {
            projects: vec![Project {
                id: "project-1".to_string(),
                display_name: "Project One".to_string(),
                repo_path: "/tmp/project-1".to_string(),
                color: "#123456".to_string(),
                tags: vec!["phase-0".to_string()],
                budget_max_usd: 25.0,
                budget_warn_usd: 20.0,
                current_cost_usd: 1.5,
                slots: Vec::new(),
            }],
            task_dag: Vec::new(),
            total_session_cost: 1.5,
            session_budget_max_usd: 50.0,
            last_event_sequence: 0,
        })
        .await;

        let mut client = harness.client().await;
        let response = client
            .get_full_state(Request::new(StateRequest {}))
            .await
            .expect("get full state")
            .into_inner();

        assert_eq!(response.projects.len(), 1);
        assert_eq!(response.projects[0].id, "project-1");
        assert_eq!(response.total_session_cost, 1.5);

        harness.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatch_command_forwards_into_the_command_channel() {
        let harness = GrpcHarness::new(FullStateSnapshot::default()).await;
        let mut client = harness.client().await;

        let command = OperatorCommand {
            command_id: "cmd-1".to_string(),
            action: None,
        };
        let expected_command_id = command.command_id.clone();

        let command_task = tokio::spawn(async move {
            client
                .dispatch_command(Request::new(command.clone()))
                .await
                .expect("dispatch command")
                .into_inner()
        });

        let (received, response_tx) = timeout(Duration::from_secs(2), harness.recv_command())
            .await
            .expect("receive command before timeout")
            .expect("command");
        response_tx
            .send(CommandResponse {
                success: true,
                error_message: String::new(),
                command_id: received.command_id.clone(),
                outcome: CommandOutcome::Executed as i32,
            })
            .expect("send response");

        let response = command_task.await.expect("join dispatch task");
        assert!(response.success);
        assert_eq!(response.command_id, "cmd-1");
        assert_eq!(response.outcome, CommandOutcome::Executed as i32);
        assert_eq!(received.command_id, expected_command_id);

        harness.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatch_command_times_out_when_engine_does_not_respond() {
        let harness = GrpcHarness::new(FullStateSnapshot::default()).await;
        let mut client = harness.client().await;

        let response = client
            .dispatch_command(Request::new(OperatorCommand {
                command_id: "cmd-timeout".to_string(),
                action: None,
            }))
            .await
            .expect("dispatch command")
            .into_inner();

        assert!(!response.success);
        assert_eq!(response.command_id, "cmd-timeout");
        assert_eq!(response.outcome, CommandOutcome::Unspecified as i32);
        assert!(response.error_message.contains("did not respond"));

        harness.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn subscribe_events_streams_published_events() {
        let harness = GrpcHarness::new(FullStateSnapshot::default()).await;
        let mut client = harness.client().await;

        let mut stream = client
            .subscribe_events(Request::new(SubscribeRequest {
                client_version: "test-client".to_string(),
            }))
            .await
            .expect("subscribe events")
            .into_inner();

        harness.service.publish_event(HypervisorEvent {
            event_id: "event-1".to_string(),
            timestamp_ms: 1234,
            barrier_id: "barrier-1".to_string(),
            event_sequence: 1,
            payload: Some(hypervisor_event::Payload::AgentStateChanged(
                AgentStateChanged {
                    agent_id: "agent-1".to_string(),
                    new_state: AgentState::Executing as i32,
                    slot_id: "slot-1".to_string(),
                },
            )),
        });

        let event = timeout(Duration::from_secs(2), stream.message())
            .await
            .expect("receive event before timeout")
            .expect("stream response")
            .expect("event payload");

        assert_eq!(event.event_id, "event-1");
        assert_eq!(event.event_sequence, 1);
        assert!(matches!(
            event.payload,
            Some(hypervisor_event::Payload::AgentStateChanged(payload))
                if payload.agent_id == "agent-1"
                    && payload.slot_id == "slot-1"
                    && payload.new_state == AgentState::Executing as i32
        ));

        drop(stream);
        drop(client);
        harness.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn subscribe_events_reports_lagged_consumers() {
        let harness = GrpcHarness::with_event_buffer(FullStateSnapshot::default(), 4).await;
        let mut client = harness.client().await;

        let mut stream = client
            .subscribe_events(Request::new(SubscribeRequest {
                client_version: "test-client".to_string(),
            }))
            .await
            .expect("subscribe events")
            .into_inner();

        for sequence in 1..=8 {
            harness.service.publish_event(HypervisorEvent {
                event_id: format!("event-{sequence}"),
                timestamp_ms: sequence,
                barrier_id: String::new(),
                event_sequence: sequence,
                payload: None,
            });
        }

        let error = stream
            .message()
            .await
            .expect_err("lagged stream should surface data loss");
        assert_eq!(error.code(), tonic::Code::DataLoss);
        assert!(error.message().contains("lagged"));

        harness.shutdown().await;
    }

    struct GrpcHarness {
        service: HypervisorService,
        command_rx: tokio::sync::Mutex<CommandReceiver>,
        addr: SocketAddr,
        shutdown_tx: Option<oneshot::Sender<()>>,
        server_task: tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
    }

    impl GrpcHarness {
        async fn new(initial_state: FullStateSnapshot) -> Self {
            Self::with_event_buffer(initial_state, 256).await
        }

        async fn with_event_buffer(initial_state: FullStateSnapshot, event_buffer: usize) -> Self {
            let bridge = GrpcBridge::with_event_buffer(initial_state, event_buffer);
            let (service, command_rx) = bridge.into_parts();
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind tcp listener");
            let addr = listener.local_addr().expect("listener addr");
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let server_service = service.clone();
            let server_task = tokio::spawn(async move {
                server_service
                    .serve_tcp(listener, async move {
                        let _ = shutdown_rx.await;
                    })
                    .await
            });

            Self {
                service,
                command_rx: tokio::sync::Mutex::new(command_rx),
                addr,
                shutdown_tx: Some(shutdown_tx),
                server_task,
            }
        }

        async fn client(&self) -> HypervisorClient<tonic::transport::Channel> {
            HypervisorClient::connect(format!("http://{}", self.addr))
                .await
                .expect("connect client")
        }

        async fn recv_command(&self) -> Option<CommandEnvelope> {
            self.command_rx.lock().await.recv().await
        }

        async fn shutdown(mut self) {
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
            }
            match timeout(Duration::from_secs(2), &mut self.server_task).await {
                Ok(result) => result
                    .expect("join server task")
                    .expect("server shutdown cleanly"),
                Err(_) => {
                    self.server_task.abort();
                    let _ = self.server_task.await;
                }
            }
        }
    }
}
