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

/** Mirrors `LifecycleAction` in crates/vagrant-adapter. */
export type LifecycleAction =
  'up' | 'halt' | 'reload' | 'suspend' | 'resume' | 'provision' | 'destroy'

/** One-time token for a destructive action (crates/operation-engine `Confirmation`). */
export interface Confirmation {
  token: string
  /** What the user has to type. */
  phrase: string
  expiresAt: string
}

export interface ConfirmationProof {
  token: string
  typed: string
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
    message?: string
    operation?: Operation
    status?: EnvironmentStatus
    /** `operation.provisioning` */
    machines?: MachineProvisioning[]
  }
}

/** Mirrors `Provisioning` in crates/vagrant-adapter. */
export type Provisioning = { kind: 'default' } | { kind: 'all' } | { kind: 'only'; names: string[] }

/** Mirrors `ProvisionerRun` in crates/vagrant-adapter (`provision` module). */
export interface ProvisionerRun {
  machine: string
  /** `null` for unnamed provisioners, which Vagrant labels by type. */
  name: string | null
  kind: string
  status: 'succeeded' | 'failed' | 'unfinished'
  /** The last lines Vagrant printed for it, without the machine prefix. */
  output: string[]
  truncated: boolean
}

/** Mirrors `MachineProvisioning` in crates/operation-engine. */
export interface MachineProvisioning {
  operationId: string
  machine: string
  finishedAt: string
  runs: ProvisionerRun[]
}

/** Mirrors `commands::snapshots::MachineSnapshots`. */
export interface MachineSnapshots {
  machine: string
  names: string[]
}

export type SnapshotChange = 'restore' | 'delete'

export type FindingLevel = 'ok' | 'warning' | 'error'

export interface Finding {
  level: FindingLevel
  message: string
  fix: string | null
}

/** Mirrors `vagrant_studio_provider_registry::Capability`. */
export type Capability =
  | 'lifecycle.basic'
  | 'lifecycle.suspend'
  | 'provision.shell'
  | 'network.forwarded_port'
  | 'network.private'
  | 'network.public'
  | 'synced_folder.default'
  | 'config.cpu_memory'
  | 'ssh'
  | 'snapshot'
  | 'export'

/** Mirrors `vagrant_studio_provider_registry::ProviderReport`. */
export interface ProviderCheck {
  id: string
  name: string
  kind: 'vm' | 'container'
  state: 'ready' | 'attention' | 'unavailable'
  /** `verified` only after the full test plan passed on this host OS. */
  support: 'verified' | 'detected' | 'unavailable'
  version: string | null
  /** Empty unless detection found the provider. */
  capabilities: Capability[]
  findings: Finding[]
}

export interface HostReadiness {
  vagrant: { path: string | null; version: string | null; findings: Finding[] }
  plugins: { name: string; version: string | null; scope: string | null }[]
  providers: ProviderCheck[]
}
