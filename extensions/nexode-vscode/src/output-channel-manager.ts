import * as vscode from 'vscode';
import type { AgentOutputLine, StateCache, TaskStatusName } from './state';

const TERMINAL_STATUSES: Set<TaskStatusName> = new Set([
  'TASK_STATUS_DONE',
  'TASK_STATUS_ARCHIVED',
]);

export class OutputChannelManager implements vscode.Disposable {
  private readonly channels: Map<string, vscode.OutputChannel> = new Map();
  private readonly stateSubscription: vscode.Disposable;

  constructor(private readonly state: StateCache) {
    this.stateSubscription = this.state.onDidChange(() => this.cleanupChannels());
  }

  appendLine(output: AgentOutputLine): void {
    let channel = this.channels.get(output.slotId);
    if (!channel) {
      channel = vscode.window.createOutputChannel(`Nexode: ${output.slotId}`);
      this.channels.set(output.slotId, channel);
    }
    const prefix = output.stream === 'stderr' ? '[stderr] ' : '';
    channel.appendLine(`${prefix}${output.line}`);
  }

  showSlotOutput(slotId: string): void {
    const channel = this.channels.get(slotId);
    if (channel) {
      channel.show(true); // preserveFocus = true
    }
  }

  private cleanupChannels(): void {
    for (const project of this.state.getProjects()) {
      for (const slot of project.slots) {
        const status = this.state.getTaskStatusForSlot(slot.id);
        if (TERMINAL_STATUSES.has(status)) {
          const channel = this.channels.get(slot.id);
          if (channel) {
            channel.dispose();
            this.channels.delete(slot.id);
          }
        }
      }
    }
  }

  dispose(): void {
    this.stateSubscription.dispose();
    for (const channel of this.channels.values()) {
      channel.dispose();
    }
    this.channels.clear();
  }
}
