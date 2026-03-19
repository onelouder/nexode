import type {
  AgentPresence,
  AgentSlot,
  AgentStateName,
  FullStateSnapshot,
  Project,
  TaskNode,
  TaskStatusName,
} from './state';

export interface SlotCardModel {
  project: Project;
  slot: AgentSlot;
  task?: TaskNode;
  status: TaskStatusName;
  agentState: AgentStateName;
}

export interface KanbanCardModel {
  project?: Project;
  slot?: AgentSlot;
  task: TaskNode;
  agentId: string;
  agentState: AgentStateName;
}

export function buildSlotCardModels(
  snapshot: FullStateSnapshot,
  agents: readonly AgentPresence[],
): SlotCardModel[] {
  const taskMap = new Map(snapshot.taskDag.map((task) => [task.id, task]));
  const agentMap = new Map(agents.map((agent) => [agent.agentId, agent]));

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
      };
    }),
  );
}

export function buildKanbanCardModels(
  snapshot: FullStateSnapshot,
  agents: readonly AgentPresence[],
  projectFilter = 'all',
): KanbanCardModel[] {
  const projectById = new Map(snapshot.projects.map((project) => [project.id, project]));
  const slotByTaskId = new Map<string, { project: Project; slot: AgentSlot }>();
  for (const project of snapshot.projects) {
    for (const slot of project.slots) {
      slotByTaskId.set(slot.id, { project, slot });
    }
  }

  const agentMap = new Map(agents.map((agent) => [agent.agentId, agent]));

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
      };
    });
}
