import type { MachineState } from '@/components/status/machine-state'
import type {
  Capability,
  LifecycleAction,
  LogLine,
  Operation,
  OperationStatus,
} from '@/types/operations'

/**
 * Maps Vagrant provider states to the UI's status vocabulary. Unknown values stay unknown.
 * Keep in sync with `StateClass::of` in crates/vagrant-adapter.
 */
export function machineState(vagrantState: string | null | undefined): MachineState {
  switch (vagrantState) {
    case 'running':
      return 'running'
    case 'poweroff':
    case 'not_running':
    case 'shutoff':
    case 'stopped':
    case 'halted':
    case 'aborted':
      return 'stopped'
    case 'saved':
    case 'suspended':
    case 'paused':
      return 'suspended'
    case 'not_created':
      return 'notCreated'
    default:
      return 'unknown'
  }
}

export const LIFECYCLE_ACTIONS: readonly LifecycleAction[] = [
  'up',
  'halt',
  'reload',
  'suspend',
  'resume',
  'provision',
  'destroy',
]

const ACTIONS_BY_STATE: Partial<Record<MachineState, readonly LifecycleAction[]>> = {
  running: ['halt', 'reload', 'suspend', 'provision', 'destroy'],
  stopped: ['up', 'reload', 'destroy'],
  suspended: ['resume', 'up', 'halt', 'destroy'],
  notCreated: ['up'],
}

/**
 * Actions that make sense for a machine in this state on its provider. When
 * the state is unknown every action is offered and Vagrant decides. Provider
 * capabilities limit the list only when detection reported them (a non-empty
 * list); an unknown or undetected provider isn't limited.
 */
export function actionsFor(
  state: MachineState,
  capabilities: readonly Capability[] = [],
): readonly LifecycleAction[] {
  const actions = ACTIONS_BY_STATE[state] ?? LIFECYCLE_ACTIONS
  if (!capabilities.length || capabilities.includes('lifecycle.suspend')) return actions
  return actions.filter((a) => a !== 'suspend' && a !== 'resume')
}

/** The one-click action shown on a machine row, if any. */
export function primaryAction(state: MachineState): LifecycleAction | null {
  switch (state) {
    case 'running':
      return 'halt'
    case 'stopped':
    case 'notCreated':
      return 'up'
    case 'suspended':
      return 'resume'
    default:
      return null
  }
}

/**
 * Machines a lifecycle operation names, read from its command line
 * (`vagrant <verb> [options] --no-tty <names…>`). Option values such as
 * provisioner names (which may contain spaces) come before `--no-tty`;
 * machine names never contain spaces or start with `-`.
 */
export function targetMachines(op: Pick<Operation, 'operationType' | 'commandLine'>): string[] {
  if (!(LIFECYCLE_ACTIONS as readonly string[]).includes(op.operationType)) return []
  const tokens = op.commandLine.split(' ')
  const end = tokens.indexOf('--no-tty')
  return tokens.slice(end === -1 ? 2 : end + 1).filter((t) => t && !t.startsWith('-'))
}

export const ACTIVE_STATUSES: readonly OperationStatus[] = ['queued', 'running', 'cancel_requested']

export function isActive(op: Pick<Operation, 'status'>): boolean {
  return ACTIVE_STATUSES.includes(op.status)
}

const TYPE_LABELS: Record<string, string> = {
  status: 'Status',
  validate: 'Validate',
  box_list: 'Box list',
  box_add: 'Add box',
  box_update: 'Update box',
  box_remove: 'Remove box',
  up: 'Start',
  halt: 'Stop',
  reload: 'Reload',
  suspend: 'Suspend',
  resume: 'Resume',
  provision: 'Provision',
  destroy: 'Destroy',
}

export function operationLabel(type: string): string {
  return TYPE_LABELS[type] ?? type
}

export function duration(
  op: Pick<Operation, 'startedAt' | 'finishedAt'>,
  now = Date.now(),
): string {
  if (!op.startedAt) return '—'
  const ms = (op.finishedAt ? Date.parse(op.finishedAt) : now) - Date.parse(op.startedAt)
  const s = Math.max(0, Math.round(ms / 1000))
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  return m < 60 ? `${m}m ${s % 60}s` : `${Math.floor(m / 60)}h ${m % 60}m`
}

/**
 * Display text for a log line. Vagrant machine-readable lines
 * (`timestamp,target,type,data…`) are shown as their human message where
 * Vagrant included one (`ui` lines); other records are kept verbatim.
 */
export function displayLine(line: LogLine): string {
  const match = /^\d+,([^,]*),ui,[a-z]+,(.*)$/.exec(line.text)
  if (!match) return line.text
  const [, target, message] = match
  const text = (message ?? '').replaceAll('%!(VAGRANT_COMMA)', ',').replaceAll('\\n', '\n')
  return target ? `${target}: ${text}` : text
}
