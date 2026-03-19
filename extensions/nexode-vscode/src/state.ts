export type AgentModeName =
  | 'AGENT_MODE_UNSPECIFIED'
  | 'AGENT_MODE_NORMAL'
  | 'AGENT_MODE_PLAN'
  | 'AGENT_MODE_FULL_AUTO';

export type AgentStateName =
  | 'AGENT_STATE_UNSPECIFIED'
  | 'AGENT_STATE_INIT'
  | 'AGENT_STATE_IDLE'
  | 'AGENT_STATE_PLANNING'
  | 'AGENT_STATE_EXECUTING'
  | 'AGENT_STATE_REVIEW'
  | 'AGENT_STATE_BLOCKED'
  | 'AGENT_STATE_TERMINATED';

export type TaskStatusName =
  | 'TASK_STATUS_UNSPECIFIED'
  | 'TASK_STATUS_PENDING'
  | 'TASK_STATUS_WORKING'
  | 'TASK_STATUS_REVIEW'
  | 'TASK_STATUS_MERGE_QUEUE'
  | 'TASK_STATUS_RESOLVING'
  | 'TASK_STATUS_DONE'
  | 'TASK_STATUS_PAUSED'
  | 'TASK_STATUS_ARCHIVED';

export interface AgentSlot {
  id: string;
  projectId: string;
  task: string;
  mode: AgentModeName;
  branch: string;
  currentAgentId: string;
  worktreeId: string;
  totalTokens: number;
  totalCostUsd: number;
}

export interface Project {
  id: string;
  displayName: string;
  repoPath: string;
  color: string;
  tags: string[];
  budgetMaxUsd: number;
  budgetWarnUsd: number;
  currentCostUsd: number;
  slots: AgentSlot[];
}

export interface TaskNode {
  id: string;
  title: string;
  description: string;
  status: TaskStatusName;
  assignedAgentId: string;
  projectId: string;
  dependencyIds: string[];
}

export interface FullStateSnapshot {
  projects: Project[];
  taskDag: TaskNode[];
  totalSessionCost: number;
  sessionBudgetMaxUsd: number;
  lastEventSequence: number;
}

export interface AgentStateChangedEvent {
  agentId: string;
  newState: AgentStateName;
  slotId: string;
}

export interface AgentTelemetryUpdatedEvent {
  agentId: string;
  incrTokens: number;
  tps: number;
}

export interface TaskStatusChangedEvent {
  taskId: string;
  newStatus: TaskStatusName;
  agentId: string;
}

export interface ProjectBudgetAlertEvent {
  projectId: string;
  currentUsd: number;
  limitUsd: number;
  hardKill: boolean;
}

export interface SlotAgentSwappedEvent {
  slotId: string;
  oldAgentId: string;
  newAgentId: string;
  reason: string;
}

export interface UncertaintyFlagTriggeredEvent {
  agentId: string;
  taskId: string;
  reason: string;
}

export interface WorktreeStatusChangedEvent {
  worktreeId: string;
  newRisk: number;
}

export type ObserverInterventionName =
  | 'OBSERVER_INTERVENTION_UNSPECIFIED'
  | 'OBSERVER_INTERVENTION_ALERT'
  | 'OBSERVER_INTERVENTION_KILL'
  | 'OBSERVER_INTERVENTION_PAUSE';

export type FindingKindName =
  | 'FINDING_KIND_UNSPECIFIED'
  | 'FINDING_KIND_LOOP_DETECTED'
  | 'FINDING_KIND_STUCK'
  | 'FINDING_KIND_BUDGET_VELOCITY';

export interface LoopDetected {
  reason: string;
  intervention: ObserverInterventionName;
  findingKind: FindingKindName;
}

export interface SandboxViolation {
  path: string;
  reason: string;
}

export interface UncertaintySignal {
  reason: string;
}

export interface ObserverAlertEvent {
  slotId: string;
  agentId: string;
  loopDetected?: LoopDetected;
  sandboxViolation?: SandboxViolation;
  uncertaintySignal?: UncertaintySignal;
}

export interface RecentObserverAlert extends ObserverAlertEvent {
  eventId: string;
  timestampMs: number;
  eventSequence: number;
}

export interface HypervisorEvent {
  eventId: string;
  timestampMs: number;
  barrierId: string;
  eventSequence: number;
  agentStateChanged?: AgentStateChangedEvent;
  agentTelemetryUpdated?: AgentTelemetryUpdatedEvent;
  taskStatusChanged?: TaskStatusChangedEvent;
  uncertaintyFlag?: UncertaintyFlagTriggeredEvent;
  worktreeStatusChanged?: WorktreeStatusChangedEvent;
  projectBudgetAlert?: ProjectBudgetAlertEvent;
  slotAgentSwapped?: SlotAgentSwappedEvent;
  observerAlert?: ObserverAlertEvent;
  payload?: string;
}

export interface CommandResponse {
  success: boolean;
  errorMessage: string;
  commandId: string;
  outcome: string;
}

export interface ConnectionStatus {
  state: 'connected' | 'disconnected' | 'reconnecting';
  attempt?: number;
  detail?: string;
  nextRetryAt?: number;
}

export interface SlotSummary {
  project: Project;
  slot: AgentSlot;
  task?: TaskNode;
  status: TaskStatusName;
}

export interface AgentPresence {
  agentId: string;
  slotId: string;
  state: AgentStateName;
}

export interface AggregateMetrics {
  agentCount: number;
  totalTokens: number;
  totalSessionCost: number;
}

export interface DisposableLike {
  dispose(): void;
}

export type Event<T> = (listener: (event: T) => void) => DisposableLike;

const DEFAULT_CONNECTION_STATUS: ConnectionStatus = {
  state: 'disconnected',
};
const MAX_RECENT_ALERTS = 20;

const TASK_STATUSES: TaskStatusName[] = [
  'TASK_STATUS_UNSPECIFIED',
  'TASK_STATUS_PENDING',
  'TASK_STATUS_WORKING',
  'TASK_STATUS_REVIEW',
  'TASK_STATUS_MERGE_QUEUE',
  'TASK_STATUS_RESOLVING',
  'TASK_STATUS_DONE',
  'TASK_STATUS_PAUSED',
  'TASK_STATUS_ARCHIVED',
];

const AGENT_STATES: AgentStateName[] = [
  'AGENT_STATE_UNSPECIFIED',
  'AGENT_STATE_INIT',
  'AGENT_STATE_IDLE',
  'AGENT_STATE_PLANNING',
  'AGENT_STATE_EXECUTING',
  'AGENT_STATE_REVIEW',
  'AGENT_STATE_BLOCKED',
  'AGENT_STATE_TERMINATED',
];

const AGENT_MODES: AgentModeName[] = [
  'AGENT_MODE_UNSPECIFIED',
  'AGENT_MODE_NORMAL',
  'AGENT_MODE_PLAN',
  'AGENT_MODE_FULL_AUTO',
];

const OBSERVER_INTERVENTIONS: ObserverInterventionName[] = [
  'OBSERVER_INTERVENTION_UNSPECIFIED',
  'OBSERVER_INTERVENTION_ALERT',
  'OBSERVER_INTERVENTION_KILL',
  'OBSERVER_INTERVENTION_PAUSE',
];

const FINDING_KINDS: FindingKindName[] = [
  'FINDING_KIND_UNSPECIFIED',
  'FINDING_KIND_LOOP_DETECTED',
  'FINDING_KIND_STUCK',
  'FINDING_KIND_BUDGET_VELOCITY',
];

class Emitter<T> implements DisposableLike {
  private readonly listeners = new Set<(event: T) => void>();

  public readonly event: Event<T> = (listener) => {
    this.listeners.add(listener);
    return {
      dispose: () => {
        this.listeners.delete(listener);
      },
    };
  };

  public fire(event: T): void {
    for (const listener of [...this.listeners]) {
      listener(event);
    }
  }

  public dispose(): void {
    this.listeners.clear();
  }
}

export class StateCache {
  private readonly changeEmitter = new Emitter<void>();
  private projects: Project[] = [];
  private taskDag: TaskNode[] = [];
  private agents = new Map<string, AgentPresence>();
  private alerts: RecentObserverAlert[] = [];
  private totalSessionCost = 0;
  private sessionBudgetMaxUsd = 0;
  private lastEventSequence = 0;
  private connectionStatus: ConnectionStatus = DEFAULT_CONNECTION_STATUS;

  public readonly onDidChange = this.changeEmitter.event;

  public dispose(): void {
    this.changeEmitter.dispose();
  }

  public applySnapshot(snapshot: FullStateSnapshot): void {
    this.projects = snapshot.projects.map(cloneProject);
    this.taskDag = snapshot.taskDag.map(cloneTaskNode);
    this.agents = seedAgents(this.projects, this.agents);
    this.totalSessionCost = snapshot.totalSessionCost;
    this.sessionBudgetMaxUsd = snapshot.sessionBudgetMaxUsd;
    this.lastEventSequence = snapshot.lastEventSequence;
    this.changeEmitter.fire();
  }

  public applyEvent(event: HypervisorEvent): void {
    this.lastEventSequence = Math.max(this.lastEventSequence, event.eventSequence);

    if (event.taskStatusChanged) {
      const task = this.taskDag.find((entry) => entry.id === event.taskStatusChanged?.taskId);
      if (task) {
        task.status = event.taskStatusChanged.newStatus;
        if (event.taskStatusChanged.agentId) {
          task.assignedAgentId = event.taskStatusChanged.agentId;
        }
      }
    }

    if (event.agentTelemetryUpdated) {
      const slot = this.projects
        .flatMap((project) => project.slots)
        .find((entry) => entry.currentAgentId === event.agentTelemetryUpdated?.agentId);
      if (slot) {
        slot.totalTokens += event.agentTelemetryUpdated.incrTokens;
        if (!this.agents.has(event.agentTelemetryUpdated.agentId)) {
          this.agents.set(event.agentTelemetryUpdated.agentId, {
            agentId: event.agentTelemetryUpdated.agentId,
            slotId: slot.id,
            state: 'AGENT_STATE_UNSPECIFIED',
          });
        }
      }
    }

    if (event.projectBudgetAlert) {
      const project = this.projects.find((entry) => entry.id === event.projectBudgetAlert?.projectId);
      if (project) {
        project.currentCostUsd = event.projectBudgetAlert.currentUsd;
      }
    }

    if (event.slotAgentSwapped) {
      const slot = this.projects
        .flatMap((project) => project.slots)
        .find((entry) => entry.id === event.slotAgentSwapped?.slotId);
      if (slot) {
        const preservedState = event.slotAgentSwapped.newAgentId
          ? this.agents.get(event.slotAgentSwapped.newAgentId)?.state
          : undefined;
        if (event.slotAgentSwapped.oldAgentId) {
          this.agents.delete(event.slotAgentSwapped.oldAgentId);
        }
        slot.currentAgentId = event.slotAgentSwapped.newAgentId;
        if (event.slotAgentSwapped.newAgentId) {
          this.agents.set(event.slotAgentSwapped.newAgentId, {
            agentId: event.slotAgentSwapped.newAgentId,
            slotId: slot.id,
            state: preservedState ?? 'AGENT_STATE_UNSPECIFIED',
          });
        }
      }
    }

    if (event.agentStateChanged?.slotId && event.agentStateChanged.agentId) {
      const slot = this.projects
        .flatMap((project) => project.slots)
        .find((entry) => entry.id === event.agentStateChanged?.slotId);
      const previousAgentId = slot?.currentAgentId;
      if (slot) {
        slot.currentAgentId = event.agentStateChanged.agentId;
      }

      this.agents.set(event.agentStateChanged.agentId, {
        agentId: event.agentStateChanged.agentId,
        slotId: event.agentStateChanged.slotId,
        state: event.agentStateChanged.newState,
      });
      if (previousAgentId && previousAgentId !== event.agentStateChanged.agentId) {
        this.agents.delete(previousAgentId);
      }
    }

    if (event.observerAlert) {
      this.pushAlert({
        eventId: event.eventId,
        timestampMs: event.timestampMs,
        eventSequence: event.eventSequence,
        ...event.observerAlert,
      });
    } else if (event.uncertaintyFlag) {
      this.pushAlert({
        eventId: event.eventId,
        timestampMs: event.timestampMs,
        eventSequence: event.eventSequence,
        slotId: event.uncertaintyFlag.taskId,
        agentId: event.uncertaintyFlag.agentId,
        uncertaintySignal: {
          reason: event.uncertaintyFlag.reason,
        },
      });
    }

    this.changeEmitter.fire();
  }

  public setConnectionStatus(status: ConnectionStatus): void {
    this.connectionStatus = {
      ...status,
      nextRetryAt: status.nextRetryAt,
    };
    this.changeEmitter.fire();
  }

  public getConnectionStatus(): ConnectionStatus {
    return this.connectionStatus;
  }

  public getSnapshot(): FullStateSnapshot {
    return {
      projects: this.projects.map(cloneProject),
      taskDag: this.taskDag.map(cloneTaskNode),
      totalSessionCost: this.totalSessionCost,
      sessionBudgetMaxUsd: this.sessionBudgetMaxUsd,
      lastEventSequence: this.lastEventSequence,
    };
  }

  public getProjects(): readonly Project[] {
    return this.projects;
  }

  public getAgentStates(): AgentPresence[] {
    return [...this.agents.values()].map(cloneAgentPresence);
  }

  public getAlerts(): RecentObserverAlert[] {
    return this.alerts.map(cloneRecentObserverAlert);
  }

  public getAgentState(agentId: string): AgentStateName | undefined {
    return this.agents.get(agentId)?.state;
  }

  public getAgentsBySlot(slotId: string): AgentPresence[] {
    return [...this.agents.values()]
      .filter((agent) => agent.slotId === slotId)
      .map(cloneAgentPresence);
  }

  public getTaskDag(): readonly TaskNode[] {
    return this.taskDag;
  }

  public getTaskById(taskId: string): TaskNode | undefined {
    return this.taskDag.find((task) => task.id === taskId);
  }

  public getTaskStatusForSlot(slotId: string): TaskStatusName {
    return this.getTaskById(slotId)?.status ?? 'TASK_STATUS_UNSPECIFIED';
  }

  public getAllSlots(): SlotSummary[] {
    return this.projects.flatMap((project) =>
      project.slots.map((slot) => ({
        project,
        slot,
        task: this.getTaskById(slot.id),
        status: this.getTaskStatusForSlot(slot.id),
      })),
    );
  }

  public getSlotsByStatuses(statuses: readonly TaskStatusName[]): SlotSummary[] {
    const allowed = new Set(statuses);
    return this.getAllSlots()
      .filter((summary) => allowed.has(summary.status))
      .sort((left, right) => {
        const projectOrder = left.project.displayName.localeCompare(right.project.displayName);
        return projectOrder !== 0 ? projectOrder : left.slot.id.localeCompare(right.slot.id);
      });
  }

  public getAggregateMetrics(): AggregateMetrics {
    let totalTokens = 0;

    for (const project of this.projects) {
      for (const slot of project.slots) {
        totalTokens += slot.totalTokens;
      }
    }

    return {
      agentCount: this.agents.size,
      totalTokens,
      totalSessionCost: this.totalSessionCost,
    };
  }

  public hasSnapshot(): boolean {
    return this.projects.length > 0 || this.taskDag.length > 0 || this.lastEventSequence > 0;
  }

  public getSessionBudgetMaxUsd(): number {
    return this.sessionBudgetMaxUsd;
  }

  private pushAlert(alert: RecentObserverAlert): void {
    this.alerts = [cloneRecentObserverAlert(alert), ...this.alerts].slice(0, MAX_RECENT_ALERTS);
  }
}

export function normalizeSnapshot(raw: Record<string, unknown> | undefined): FullStateSnapshot {
  return {
    projects: normalizeProjectList(raw?.projects),
    taskDag: normalizeTaskDag(raw?.taskDag),
    totalSessionCost: coerceNumber(raw?.totalSessionCost),
    sessionBudgetMaxUsd: coerceNumber(raw?.sessionBudgetMaxUsd),
    lastEventSequence: coerceNumber(raw?.lastEventSequence),
  };
}

export function normalizeEvent(raw: Record<string, unknown> | undefined): HypervisorEvent {
  return {
    eventId: coerceString(raw?.eventId),
    timestampMs: coerceNumber(raw?.timestampMs),
    barrierId: coerceString(raw?.barrierId),
    eventSequence: coerceNumber(raw?.eventSequence),
    agentStateChanged: normalizeAgentStateChanged(raw?.agentStateChanged),
    agentTelemetryUpdated: normalizeAgentTelemetryUpdated(raw?.agentTelemetryUpdated),
    taskStatusChanged: normalizeTaskStatusChanged(raw?.taskStatusChanged),
    uncertaintyFlag: normalizeUncertaintyFlagTriggered(raw?.uncertaintyFlag),
    worktreeStatusChanged: normalizeWorktreeStatusChanged(raw?.worktreeStatusChanged),
    projectBudgetAlert: normalizeProjectBudgetAlert(raw?.projectBudgetAlert),
    slotAgentSwapped: normalizeSlotAgentSwapped(raw?.slotAgentSwapped),
    observerAlert: normalizeObserverAlert(raw?.observerAlert),
    payload: coerceString(raw?.payload),
  };
}

export function normalizeCommandResponse(raw: Record<string, unknown> | undefined): CommandResponse {
  return {
    success: Boolean(raw?.success),
    errorMessage: coerceString(raw?.errorMessage),
    commandId: coerceString(raw?.commandId),
    outcome: coerceEnum(raw?.outcome, [
      'COMMAND_OUTCOME_UNSPECIFIED',
      'COMMAND_OUTCOME_EXECUTED',
      'COMMAND_OUTCOME_REJECTED',
      'COMMAND_OUTCOME_SLOT_NOT_FOUND',
      'COMMAND_OUTCOME_INVALID_TRANSITION',
    ]),
  };
}

export function formatTaskStatus(status: TaskStatusName): string {
  return status.replace('TASK_STATUS_', '').replaceAll('_', ' ');
}

export function formatCommandOutcome(outcome: string): string {
  return outcome.replace('COMMAND_OUTCOME_', '').replaceAll('_', ' ');
}

export function formatTokenCount(totalTokens: number): string {
  return new Intl.NumberFormat('en-US').format(totalTokens);
}

function normalizeProjectList(raw: unknown): Project[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw.map((entry) => {
    const project = asRecord(entry);
    return {
      id: coerceString(project.id),
      displayName: coerceString(project.displayName),
      repoPath: coerceString(project.repoPath),
      color: coerceString(project.color),
      tags: Array.isArray(project.tags) ? project.tags.map((tag) => coerceString(tag)) : [],
      budgetMaxUsd: coerceNumber(project.budgetMaxUsd),
      budgetWarnUsd: coerceNumber(project.budgetWarnUsd),
      currentCostUsd: coerceNumber(project.currentCostUsd),
      slots: normalizeSlots(project.slots),
    };
  });
}

function normalizeSlots(raw: unknown): AgentSlot[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw.map((entry) => {
    const slot = asRecord(entry);
    return {
      id: coerceString(slot.id),
      projectId: coerceString(slot.projectId),
      task: coerceString(slot.task),
      mode: coerceEnum(slot.mode, AGENT_MODES),
      branch: coerceString(slot.branch),
      currentAgentId: coerceString(slot.currentAgentId),
      worktreeId: coerceString(slot.worktreeId),
      totalTokens: coerceNumber(slot.totalTokens),
      totalCostUsd: coerceNumber(slot.totalCostUsd),
    };
  });
}

function normalizeTaskDag(raw: unknown): TaskNode[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw.map((entry) => {
    const task = asRecord(entry);
    return {
      id: coerceString(task.id),
      title: coerceString(task.title),
      description: coerceString(task.description),
      status: coerceEnum(task.status, TASK_STATUSES),
      assignedAgentId: coerceString(task.assignedAgentId),
      projectId: coerceString(task.projectId),
      dependencyIds: Array.isArray(task.dependencyIds)
        ? task.dependencyIds.map((dependencyId) => coerceString(dependencyId))
        : [],
    };
  });
}

function normalizeAgentStateChanged(raw: unknown): AgentStateChangedEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    agentId: coerceString(payload.agentId),
    newState: coerceEnum(payload.newState, AGENT_STATES),
    slotId: coerceString(payload.slotId),
  };
}

function normalizeAgentTelemetryUpdated(raw: unknown): AgentTelemetryUpdatedEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    agentId: coerceString(payload.agentId),
    incrTokens: coerceNumber(payload.incrTokens),
    tps: coerceNumber(payload.tps),
  };
}

function normalizeTaskStatusChanged(raw: unknown): TaskStatusChangedEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    taskId: coerceString(payload.taskId),
    newStatus: coerceEnum(payload.newStatus, TASK_STATUSES),
    agentId: coerceString(payload.agentId),
  };
}

function normalizeProjectBudgetAlert(raw: unknown): ProjectBudgetAlertEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    projectId: coerceString(payload.projectId),
    currentUsd: coerceNumber(payload.currentUsd),
    limitUsd: coerceNumber(payload.limitUsd),
    hardKill: Boolean(payload.hardKill),
  };
}

function normalizeSlotAgentSwapped(raw: unknown): SlotAgentSwappedEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    slotId: coerceString(payload.slotId),
    oldAgentId: coerceString(payload.oldAgentId),
    newAgentId: coerceString(payload.newAgentId),
    reason: coerceString(payload.reason),
  };
}

function normalizeUncertaintyFlagTriggered(raw: unknown): UncertaintyFlagTriggeredEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    agentId: coerceString(payload.agentId),
    taskId: coerceString(payload.taskId),
    reason: coerceString(payload.reason),
  };
}

function normalizeWorktreeStatusChanged(raw: unknown): WorktreeStatusChangedEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    worktreeId: coerceString(payload.worktreeId),
    newRisk: coerceNumber(payload.newRisk),
  };
}

function normalizeObserverAlert(raw: unknown): ObserverAlertEvent | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    slotId: coerceString(payload.slotId),
    agentId: coerceString(payload.agentId),
    loopDetected: normalizeLoopDetected(payload.loopDetected),
    sandboxViolation: normalizeSandboxViolation(payload.sandboxViolation),
    uncertaintySignal: normalizeUncertaintySignal(payload.uncertaintySignal),
  };
}

function normalizeLoopDetected(raw: unknown): LoopDetected | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    reason: coerceString(payload.reason),
    intervention: coerceEnum(payload.intervention, OBSERVER_INTERVENTIONS),
    findingKind: coerceEnum(payload.findingKind, FINDING_KINDS),
  };
}

function normalizeSandboxViolation(raw: unknown): SandboxViolation | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    path: coerceString(payload.path),
    reason: coerceString(payload.reason),
  };
}

function normalizeUncertaintySignal(raw: unknown): UncertaintySignal | undefined {
  if (!raw) {
    return undefined;
  }

  const payload = asRecord(raw);
  return {
    reason: coerceString(payload.reason),
  };
}

function cloneProject(project: Project): Project {
  return {
    ...project,
    tags: [...project.tags],
    slots: project.slots.map((slot) => ({ ...slot })),
  };
}

function cloneTaskNode(task: TaskNode): TaskNode {
  return {
    ...task,
    dependencyIds: [...task.dependencyIds],
  };
}

function cloneAgentPresence(agent: AgentPresence): AgentPresence {
  return { ...agent };
}

function cloneRecentObserverAlert(alert: RecentObserverAlert): RecentObserverAlert {
  const { loopDetected, sandboxViolation, uncertaintySignal, ...base } = alert;
  return {
    ...base,
    ...(loopDetected ? { loopDetected: { ...loopDetected } } : {}),
    ...(sandboxViolation ? { sandboxViolation: { ...sandboxViolation } } : {}),
    ...(uncertaintySignal ? { uncertaintySignal: { ...uncertaintySignal } } : {}),
  };
}

function seedAgents(
  projects: readonly Project[],
  previous: ReadonlyMap<string, AgentPresence>,
): Map<string, AgentPresence> {
  const next = new Map<string, AgentPresence>();

  for (const project of projects) {
    for (const slot of project.slots) {
      if (!slot.currentAgentId) {
        continue;
      }

      next.set(slot.currentAgentId, {
        agentId: slot.currentAgentId,
        slotId: slot.id,
        state: previous.get(slot.currentAgentId)?.state ?? 'AGENT_STATE_UNSPECIFIED',
      });
    }
  }

  return next;
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : {};
}

export function coerceString(value: unknown): string {
  return typeof value === 'string' ? value : '';
}

export function coerceNumber(value: unknown): number {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }

  if (typeof value === 'bigint') {
    return Number(value);
  }

  if (typeof value === 'string') {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  return 0;
}

export function coerceEnum<T extends string>(value: unknown, options: readonly T[]): T {
  return typeof value === 'string' && options.includes(value as T) ? (value as T) : options[0];
}
