use std::pin::Pin;
use std::sync::Arc;

use nexode_proto::hypervisor_server::{Hypervisor, HypervisorServer};
use nexode_proto::{
    CommandResponse, FullStateSnapshot, HypervisorEvent, OperatorCommand, StateRequest,
    SubscribeRequest,
};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::{BroadcastStream, TcpListenerStream};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub type CommandReceiver = mpsc::UnboundedReceiver<OperatorCommand>;

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
    command_tx: mpsc::UnboundedSender<OperatorCommand>,
    state: RwLock<FullStateSnapshot>,
}

type EventStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<HypervisorEvent, Status>> + Send + 'static>>;

impl GrpcBridge {
    pub fn new(initial_state: FullStateSnapshot) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, _) = broadcast::channel(256);

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
        let stream = BroadcastStream::new(self.inner.event_tx.subscribe()).filter_map(|result| {
            match result {
                Ok(event) => Some(Ok(event)),
                // For the skeleton, dropping lagged events is acceptable; later phases can add
                // replay/state catch-up if the UI needs stronger guarantees.
                Err(_) => None,
            }
        });

        Ok(Response::new(Box::pin(stream)))
    }

    async fn dispatch_command(
        &self,
        request: Request<OperatorCommand>,
    ) -> Result<Response<CommandResponse>, Status> {
        self.inner
            .command_tx
            .send(request.into_inner())
            .map_err(|_| Status::unavailable("command channel closed"))?;

        Ok(Response::new(CommandResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn get_full_state(
        &self,
        _request: Request<StateRequest>,
    ) -> Result<Response<FullStateSnapshot>, Status> {
        Ok(Response::new(self.full_state().await))
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

        let response = client
            .dispatch_command(Request::new(command.clone()))
            .await
            .expect("dispatch command")
            .into_inner();
        assert!(response.success);

        let received = timeout(Duration::from_secs(2), harness.recv_command())
            .await
            .expect("receive command before timeout")
            .expect("command");
        assert_eq!(received.command_id, command.command_id);

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
            payload: Some(hypervisor_event::Payload::AgentStateChanged(
                AgentStateChanged {
                    agent_id: "agent-1".to_string(),
                    new_state: AgentState::Executing as i32,
                },
            )),
        });

        let event = timeout(Duration::from_secs(2), stream.message())
            .await
            .expect("receive event before timeout")
            .expect("stream response")
            .expect("event payload");

        assert_eq!(event.event_id, "event-1");
        assert!(matches!(
            event.payload,
            Some(hypervisor_event::Payload::AgentStateChanged(payload))
                if payload.agent_id == "agent-1" && payload.new_state == AgentState::Executing as i32
        ));

        drop(stream);
        drop(client);
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
            let bridge = GrpcBridge::new(initial_state);
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

        async fn recv_command(&self) -> Option<OperatorCommand> {
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
