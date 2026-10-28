export type MachineState =
  'running' | 'stopped' | 'starting' | 'stopping' | 'provisioning' | 'failed' | 'unknown'

interface StateStyle {
  label: string
  /** Tailwind background class for the indicator dot. */
  dot: string
  /** Transitional states pulse so they read as "in progress" without relying on color alone. */
  pulse: boolean
}

export const MACHINE_STATE_STYLES: Record<MachineState, StateStyle> = {
  running: { label: 'Running', dot: 'bg-status-running', pulse: false },
  stopped: { label: 'Stopped', dot: 'bg-status-stopped', pulse: false },
  starting: { label: 'Starting', dot: 'bg-status-transition', pulse: true },
  stopping: { label: 'Stopping', dot: 'bg-status-transition', pulse: true },
  provisioning: { label: 'Provisioning', dot: 'bg-status-provisioning', pulse: true },
  failed: { label: 'Failed', dot: 'bg-status-failed', pulse: false },
  unknown: { label: 'Unknown', dot: 'bg-status-unknown', pulse: false },
}

export const MACHINE_STATES = Object.keys(MACHINE_STATE_STYLES) as MachineState[]
