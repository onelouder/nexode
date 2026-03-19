import * as vscode from 'vscode';

import { DaemonClient } from './daemon-client';
import {
  CommandResponse,
  StateCache,
  TaskStatusName,
  formatCommandOutcome,
  formatTaskStatus,
  formatTokenCount,
} from './state';

interface SlotQuickPickItem extends vscode.QuickPickItem {
  slotId: string;
}

interface StatusQuickPickItem extends vscode.QuickPickItem {
  status: TaskStatusName;
}

const MOVE_TARGETS: readonly TaskStatusName[] = [
  'TASK_STATUS_WORKING',
  'TASK_STATUS_REVIEW',
  'TASK_STATUS_MERGE_QUEUE',
  'TASK_STATUS_PAUSED',
  'TASK_STATUS_ARCHIVED',
];

export function registerCommands(
  client: DaemonClient,
  state: StateCache,
  focusSlotsView: () => Thenable<void> | Promise<void>,
  openSynapseGrid: () => Thenable<void> | Promise<void>,
  openKanban: () => Thenable<void> | Promise<void>,
): vscode.Disposable[] {
  return [
    vscode.commands.registerCommand('nexode.pauseSlot', async () => {
      const selected = await selectSlot(
        state,
        ['TASK_STATUS_WORKING', 'TASK_STATUS_REVIEW'],
        'Pause Nexode Slot',
      );
      if (!selected) {
        return;
      }

      await dispatchWithFeedback(client, {
        commandId: commandId(),
        moveTask: {
          taskId: selected.slotId,
          target: 'TASK_STATUS_PAUSED',
        },
      });
    }),
    vscode.commands.registerCommand('nexode.resumeSlot', async () => {
      const selected = await selectSlot(state, ['TASK_STATUS_PAUSED'], 'Resume Nexode Slot');
      if (!selected) {
        return;
      }

      await dispatchWithFeedback(client, {
        commandId: commandId(),
        resumeSlot: {
          slotId: selected.slotId,
          instruction: '',
        },
      });
    }),
    vscode.commands.registerCommand('nexode.moveTask', async () => {
      const selectedSlot = await selectSlot(state, undefined, 'Move Nexode Task');
      if (!selectedSlot) {
        return;
      }

      const selectedStatus = await selectTargetStatus();
      if (!selectedStatus) {
        return;
      }

      await dispatchWithFeedback(client, {
        commandId: commandId(),
        moveTask: {
          taskId: selectedSlot.slotId,
          target: selectedStatus.status,
        },
      });
    }),
    vscode.commands.registerCommand('nexode.focusSlots', async () => {
      await focusSlotsView();
    }),
    vscode.commands.registerCommand('nexode.openSynapseGrid', async () => {
      await openSynapseGrid();
    }),
    vscode.commands.registerCommand('nexode.openKanban', async () => {
      await openKanban();
    }),
  ];
}

async function selectSlot(
  state: StateCache,
  statuses: readonly TaskStatusName[] | undefined,
  title: string,
): Promise<SlotQuickPickItem | undefined> {
  const slotSummaries = statuses ? state.getSlotsByStatuses(statuses) : state.getAllSlots();
  if (slotSummaries.length === 0) {
    void vscode.window.showInformationMessage('No matching Nexode slots are available.');
    return undefined;
  }

  const items: SlotQuickPickItem[] = slotSummaries.map((summary) => ({
    label: summary.slot.id,
    description: `${summary.project.displayName} · ${formatTaskStatus(summary.status)}`,
    detail: `${summary.slot.currentAgentId || '-'} · ${formatTokenCount(summary.slot.totalTokens)} tok`,
    slotId: summary.slot.id,
  }));

  return vscode.window.showQuickPick(items, {
    placeHolder: 'Select a slot',
    title,
    matchOnDescription: true,
    matchOnDetail: true,
  });
}

async function selectTargetStatus(): Promise<StatusQuickPickItem | undefined> {
  const items: StatusQuickPickItem[] = MOVE_TARGETS.map((status) => ({
    label: formatTaskStatus(status),
    status,
  }));

  return vscode.window.showQuickPick(items, {
    placeHolder: 'Select a target status',
    title: 'Move Nexode Task',
  });
}

async function dispatchWithFeedback(
  client: DaemonClient,
  command: Record<string, unknown>,
): Promise<void> {
  try {
    const response = await client.dispatchCommand(command);
    showCommandResponse(response);
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Command dispatch failed';
    void vscode.window.showErrorMessage(message);
  }
}

function showCommandResponse(response: CommandResponse): void {
  const messageParts = [formatCommandOutcome(response.outcome)];

  if (response.commandId) {
    messageParts.push(`(${response.commandId})`);
  }

  if (response.errorMessage) {
    messageParts.push(response.errorMessage);
  }

  const message = messageParts.join(' ');
  if (response.success) {
    void vscode.window.showInformationMessage(message);
    return;
  }

  void vscode.window.showErrorMessage(message);
}

function commandId(): string {
  return `vscode-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}
