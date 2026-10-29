// Mirrors crates/config-model (model, Analysis) and the Vagrantfile DTOs in
// crates/project-service/src/vagrantfile.rs.

export type ProviderKind = 'virtualbox' | 'vmware_desktop'
export type Protocol = 'tcp' | 'udp'
export type ProvisionRun = 'once' | 'always' | 'never'
export type LineEnding = 'lf' | 'crlf'

export interface VagrantConfig {
  /** Settings on `config`; they apply to every machine. */
  defaults: MachineSettings
  /** `config.vm.define` blocks. Empty: Vagrant runs one machine named "default". */
  machines: MachineDefinition[]
}

export interface MachineDefinition {
  name: string
  primary: boolean
  autostart: boolean
  settings: MachineSettings
}

export interface MachineSettings {
  box: string | null
  hostname: string | null
  privateNetworks: PrivateNetwork[]
  publicNetworks: PublicNetwork[]
  forwardedPorts: ForwardedPort[]
  syncedFolders: SyncedFolder[]
  providers: ProviderSettings[]
  provisioners: ShellProvisioner[]
}

/** A static IP, or DHCP when `ip` is null. */
export interface PrivateNetwork {
  ip: string | null
}

/**
 * A bridged network on the host's LAN: a static IP, or DHCP when `ip` is null.
 * `bridge` names the host adapter (VirtualBox only).
 */
export interface PublicNetwork {
  ip: string | null
  bridge: string | null
}

export interface ForwardedPort {
  guest: number
  host: number
  hostIp: string | null
  protocol: Protocol | null
}

export interface SyncedFolder {
  hostPath: string
  guestPath: string
  disabled: boolean
}

export interface ProviderSettings {
  provider: ProviderKind
  cpus: number | null
  memoryMb: number | null
}

export type ShellSource = { kind: 'inline'; script: string } | { kind: 'path'; path: string }

export interface ShellProvisioner {
  /** Named provisioners (`vm.provision "name", type: "shell"`) can be run on their own. */
  name: string | null
  source: ShellSource
  run: ProvisionRun | null
  privileged: boolean | null
}

export interface Unsupported {
  line: number
  message: string
}

export interface FieldError {
  /** camelCase path such as `machines.0.settings.forwardedPorts.1.host`. */
  path: string
  message: string
}

export interface Analysis {
  model: VagrantConfig | null
  unsupported: Unsupported | null
  /** Written by the visual editor (has its header). */
  managed: boolean
  /** Managed, but changed since the visual editor wrote it. */
  modifiedOutside: boolean
  warnings: string[]
  errors: FieldError[]
  lineEnding: LineEnding
}

export interface VagrantfileDocument {
  environmentId: string
  path: string
  exists: boolean
  content: string
  /** SHA-256 of the file as read; null when it doesn't exist. */
  revision: string | null
  analysis: Analysis
}

export interface SaveVagrantfileRequest {
  content: string
  baseRevision: string | null
  replaceUnmanaged: boolean
}

export interface VagrantfileBackup {
  name: string
  path: string
  size: number
  createdAt: string
}

export interface SaveVagrantfileOutcome {
  document: VagrantfileDocument
  backup: VagrantfileBackup | null
  changed: boolean
}

export interface GeneratedVagrantfile {
  content: string | null
  errors: FieldError[]
}

/** Mirrors `vagrant_studio_provider_registry::HostPort`. */
export interface HostPort {
  hostIp: string | null
  port: number
}

/** Mirrors `vagrant_studio_provider_registry::BridgedInterface`. */
export interface BridgedInterface {
  name: string
  wireless: boolean
}

/** Mirrors `MachineProvisioners` in crates/config-model: shared ones first, then the machine's. */
export interface MachineProvisioners {
  machine: string
  provisioners: (ShellProvisioner & { shared: boolean })[]
}

/** Mirrors `ProvisionerList` in src-tauri/src/commands/provisioning.rs. */
export interface ProvisionerList {
  machines: MachineProvisioners[]
  /** Why provisioners can't be listed; `machines` is then empty. */
  unavailable: string | null
}
