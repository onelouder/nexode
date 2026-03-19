import type {
  AgentPresence,
  AggregateMetrics,
  ConnectionStatus,
  FullStateSnapshot,
  RecentObserverAlert,
  TaskStatusName,
} from '../../src/state';

export type SurfaceKind = 'synapse-grid' | 'synapse-sidebar' | 'macro-kanban';

export interface StateEnvelope {
  surface: SurfaceKind;
  snapshot: FullStateSnapshot;
  agents: AgentPresence[];
  alerts: RecentObserverAlert[];
  metrics: AggregateMetrics;
  connection: ConnectionStatus;
  hasSnapshot: boolean;
}

export interface HostStateMessage {
  type: 'state';
  payload: StateEnvelope;
}

export interface ReadyMessage {
  type: 'ready';
  surface: SurfaceKind;
}

export interface MoveTaskMessage {
  type: 'moveTask';
  taskId: string;
  target: TaskStatusName;
}

export type HostToWebviewMessage = HostStateMessage;
export type WebviewToHostMessage = ReadyMessage | MoveTaskMessage;
