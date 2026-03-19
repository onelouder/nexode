import type {
  AgentPresence,
  AgentSlot,
  AgentStateName,
  FullStateSnapshot,
  Project,
  RecentObserverAlert,
  TaskNode,
  TaskStatusName,
} from './state';

export interface SlotCardModel {
  project: Project;
  slot: AgentSlot;
  task?: TaskNode;
  status: TaskStatusName;
  agentState: AgentStateName;
  alerts: RecentObserverAlert[];
}

export interface KanbanCardModel {
  project?: Project;
  slot?: AgentSlot;
  task: TaskNode;
  agentId: string;
  agentState: AgentStateName;
  alerts: RecentObserverAlert[];
}

const FLAT_VIEW_STATUS_ORDER: readonly TaskStatusName[] = [
  'TASK_STATUS_WORKING',
  'TASK_STATUS_REVIEW',
  'TASK_STATUS_MERGE_QUEUE',
  'TASK_STATUS_RESOLVING',
  'TASK_STATUS_PENDING',
  'TASK_STATUS_PAUSED',
  'TASK_STATUS_ARCHIVED',
  'TASK_STATUS_DONE',
  'TASK_STATUS_UNSPECIFIED',
];

export function buildSlotCardModels(
  snapshot: FullStateSnapshot,
  agents: readonly AgentPresence[],
  alerts: readonly RecentObserverAlert[] = [],
): SlotCardModel[] {
  const taskMap = new Map(snapshot.taskDag.map((task) => [task.id, task]));
  const agentMap = new Map(agents.map((agent) => [agent.agentId, agent]));
  const alertsBySlotId = groupAlertsBySlot(alerts);

  return snapshot.projects.flatMap((project) =>
    project.slots.map((slot) => {
      const task = taskMap.get(slot.id);
      return {
        project,
        slot,
        task,
        status: task?.status ?? 'TASK_STATUS_UNSPECIFIED',
        agentState: slot.currentAgentId
          ? agentMap.get(slot.currentAgentId)?.state ?? 'AGENT_STATE_UNSPECIFIED'
          : 'AGENT_STATE_UNSPECIFIED',
        alerts: alertsBySlotId.get(slot.id) ?? [],
      };
    }),
  );
}

export function buildKanbanCardModels(
  snapshot: FullStateSnapshot,
  agents: readonly AgentPresence[],
  projectFilter = 'all',
  alerts: readonly RecentObserverAlert[] = [],
): KanbanCardModel[] {
  const projectById = new Map(snapshot.projects.map((project) => [project.id, project]));
  const slotByTaskId = new Map<string, { project: Project; slot: AgentSlot }>();
  for (const project of snapshot.projects) {
    for (const slot of project.slots) {
      slotByTaskId.set(slot.id, { project, slot });
    }
  }

  const agentMap = new Map(agents.map((agent) => [agent.agentId, agent]));
  const alertsBySlotId = groupAlertsBySlot(alerts);

  return snapshot.taskDag
    .filter((task) => projectFilter === 'all' || task.projectId === projectFilter)
    .map((task) => {
      const resolved = slotByTaskId.get(task.id);
      const agentId = resolved?.slot.currentAgentId || task.assignedAgentId;
      return {
        project: resolved?.project ?? projectById.get(task.projectId),
        slot: resolved?.slot,
        task,
        agentId,
        agentState: agentId ? agentMap.get(agentId)?.state ?? 'AGENT_STATE_UNSPECIFIED' : 'AGENT_STATE_UNSPECIFIED',
        alerts: alertsBySlotId.get(task.id) ?? [],
      };
    });
}

export function sortSlotCardModelsForFlatView(cards: readonly SlotCardModel[]): SlotCardModel[] {
  const rank = new Map(FLAT_VIEW_STATUS_ORDER.map((status, index) => [status, index]));

  return [...cards].sort((left, right) => {
    const statusOrder = (rank.get(left.status) ?? FLAT_VIEW_STATUS_ORDER.length) - (rank.get(right.status) ?? FLAT_VIEW_STATUS_ORDER.length);
    if (statusOrder !== 0) {
      return statusOrder;
    }

    const alertOrder = right.alerts.length - left.alerts.length;
    if (alertOrder !== 0) {
      return alertOrder;
    }

    const tokenOrder = right.slot.totalTokens - left.slot.totalTokens;
    if (tokenOrder !== 0) {
      return tokenOrder;
    }

    const projectOrder = left.project.displayName.localeCompare(right.project.displayName);
    if (projectOrder !== 0) {
      return projectOrder;
    }

    return left.slot.id.localeCompare(right.slot.id);
  });
}

function groupAlertsBySlot(alerts: readonly RecentObserverAlert[]): Map<string, RecentObserverAlert[]> {
  const grouped = new Map<string, RecentObserverAlert[]>();

  for (const alert of alerts) {
    const slotAlerts = grouped.get(alert.slotId);
    if (slotAlerts) {
      slotAlerts.push(alert);
      continue;
    }

    grouped.set(alert.slotId, [alert]);
  }

  return grouped;
}
