// Mirrors crates/domain (Operation), crates/operation-engine and crates/vagrant-adapter DTOs.

export type OperationStatus =
  'queued' | 'running' | 'succeeded' | 'failed' | 'cancel_requested' | 'cancelled' | 'interrupted'

export interface Operation {
  id: string
  projectId: string | null
  environmentId: string | null
  machineName: string | null
  targetLabel: string
  operationType: string
  status: OperationStatus
  commandLine: string
  workingDir: string | null
  startedAt: string | null
  finishedAt: string | null
  exitCode: number | null
  logPath: string | null
  errorSummary: string | null
  createdAt: string
}

export interface LogLine {
  kind: 'out' | 'err' | 'meta'
  text: string
}

export interface MachineStatus {
  name: string
  provider: string | null
  /** Vagrant's state id, e.g. running, poweroff, not_created. */
  state: string | null
  stateShort: string | null
  stateLong: string | null
}

export interface EnvironmentStatus {
  environmentId: string
  operationId: string | null
  observedAt: string
  machines: MachineStatus[]
  error: { class: string | null; message: string } | null
}

export interface OperationEvent {
  type: string
  timestamp: string
  operationId: string | null
  environmentId: string | null
  machineName: string | null
  payload: {
    line?: string
    operation?: Operation
    status?: EnvironmentStatus
  }
}

export type FindingLevel = 'ok' | 'warning' | 'error'

export interface Finding {
  level: FindingLevel
  message: string
  fix: string | null
}

export interface ProviderCheck {
  id: string
  name: string
  state: 'ready' | 'attention' | 'unavailable'
  version: string | null
  findings: Finding[]
}

export interface HostReadiness {
  vagrant: { path: string | null; version: string | null; findings: Finding[] }
  plugins: { name: string; version: string | null; scope: string | null }[]
  providers: ProviderCheck[]
}
