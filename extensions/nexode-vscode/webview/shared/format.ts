import type { AgentStateName, RecentObserverAlert, TaskStatusName } from '../../src/state';

const CURRENCY_FORMATTER = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const COUNT_FORMATTER = new Intl.NumberFormat('en-US');

export function formatCurrency(value: number): string {
  return CURRENCY_FORMATTER.format(value);
}

export function formatCount(value: number): string {
  return COUNT_FORMATTER.format(value);
}

export function toTitleWords(value: string): string {
  return value
    .split('_')
    .filter(Boolean)
    .map((segment) => segment.charAt(0) + segment.slice(1).toLowerCase())
    .join(' ');
}

export function formatStatus(status: string): string {
  return toTitleWords(status.replace(/^TASK_STATUS_/, ''));
}

export function formatAgentState(state: string): string {
  return toTitleWords(state.replace(/^AGENT_STATE_/, ''));
}

export function formatMode(mode: string): string {
  return toTitleWords(mode.replace(/^AGENT_MODE_/, ''));
}

export function statusTone(status: TaskStatusName | string): string {
  switch (status) {
    case 'TASK_STATUS_WORKING':
      return 'info';
    case 'TASK_STATUS_REVIEW':
    case 'TASK_STATUS_MERGE_QUEUE':
    case 'TASK_STATUS_RESOLVING':
      return 'warn';
    case 'TASK_STATUS_DONE':
      return 'success';
    case 'TASK_STATUS_PAUSED':
    case 'TASK_STATUS_ARCHIVED':
      return 'muted';
    default:
      return 'neutral';
  }
}

export function agentTone(state: AgentStateName | string): string {
  switch (state) {
    case 'AGENT_STATE_EXECUTING':
      return 'info';
    case 'AGENT_STATE_REVIEW':
    case 'AGENT_STATE_PLANNING':
      return 'warn';
    case 'AGENT_STATE_IDLE':
      return 'success';
    case 'AGENT_STATE_BLOCKED':
    case 'AGENT_STATE_TERMINATED':
      return 'muted';
    default:
      return 'neutral';
  }
}

export function alertTone(alert: Pick<RecentObserverAlert, 'loopDetected' | 'sandboxViolation' | 'uncertaintySignal'>): string {
  if (alert.sandboxViolation) {
    return 'danger';
  }

  if (alert.loopDetected) {
    return 'warn';
  }

  if (alert.uncertaintySignal) {
    return 'info';
  }

  return 'neutral';
}

export function formatAlertKind(
  alert: Pick<RecentObserverAlert, 'loopDetected' | 'sandboxViolation' | 'uncertaintySignal'>,
): string {
  if (alert.sandboxViolation) {
    return 'Sandbox';
  }

  if (alert.loopDetected?.findingKind) {
    return toTitleWords(alert.loopDetected.findingKind.replace(/^FINDING_KIND_/, ''));
  }

  if (alert.uncertaintySignal) {
    return 'Uncertainty';
  }

  return 'Observer';
}

export function formatAlertMessage(
  alert: Pick<RecentObserverAlert, 'loopDetected' | 'sandboxViolation' | 'uncertaintySignal'>,
): string {
  if (alert.loopDetected?.reason) {
    return alert.loopDetected.reason;
  }

  if (alert.sandboxViolation) {
    const path = alert.sandboxViolation.path ? ` (${alert.sandboxViolation.path})` : '';
    return `${alert.sandboxViolation.reason}${path}`;
  }

  if (alert.uncertaintySignal?.reason) {
    return alert.uncertaintySignal.reason;
  }

  return 'Observer alert';
}

export function formatAlertTime(timestampMs: number): string {
  if (!timestampMs) {
    return 'Pending';
  }

  return new Intl.DateTimeFormat('en-US', {
    hour: 'numeric',
    minute: '2-digit',
  }).format(timestampMs);
}
