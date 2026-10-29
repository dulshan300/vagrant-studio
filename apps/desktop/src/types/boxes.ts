// Mirrors crates/vagrant-adapter (boxes, machine_readable::BoxInfo) and
// src-tauri/src/commands/boxes.rs.

/** One installed box as `vagrant box list` reports it. */
export interface BoxInfo {
  name: string
  provider: string | null
  version: string | null
  /** `n/a` for boxes without an architecture. */
  architecture: string | null
}

/** Exactly one installed box, as sent to update and remove (`BoxRef`). */
export interface BoxRef {
  name: string
  provider: string
  version: string
  architecture: string | null
}

export interface BoxUser {
  machine: string
  provider: string
  /** The state Vagrant last recorded, which may be out of date. */
  state: string | null
  folder: string
  /** Vagrant ignores index entries whose folder no longer exists. */
  folderExists: boolean
  projectId: string | null
  projectName: string | null
  environmentId: string | null
  environmentName: string | null
}

export interface InstalledBox extends BoxInfo {
  users: BoxUser[]
}

export interface BoxStore {
  boxes: InstalledBox[]
  usageError: string | null
}

export type BoxSource =
  | { kind: 'catalog'; value: string }
  | { kind: 'url'; value: string }
  | { kind: 'file'; value: string }

export interface BoxAddRequest {
  source: BoxSource
  provider: string | null
  version: string | null
  architecture: string | null
  /** Required for a file, optional for a URL. */
  name: string | null
}

export type OutdatedBox = { name: string; provider: string } & (
  | { state: 'upToDate'; version: string }
  | { state: 'outdated'; current: string; latest: string }
  | { state: 'noMetadata' }
  | { state: 'error'; message: string }
)
