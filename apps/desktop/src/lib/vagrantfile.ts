import type {
  FieldError,
  ForwardedPort,
  MachineDefinition,
  MachineSettings,
  ProviderKind,
  ShellProvisioner,
  VagrantConfig,
} from '@/types/vagrantfile'

export const PROVIDER_LABELS: Record<ProviderKind, string> = {
  virtualbox: 'VirtualBox',
  vmware_desktop: 'VMware Desktop',
}

export const DEFAULT_BOX = 'bento/ubuntu-24.04'

export function emptySettings(): MachineSettings {
  return {
    box: null,
    hostname: null,
    privateNetworks: [],
    forwardedPorts: [],
    syncedFolders: [],
    providers: [],
    provisioners: [],
  }
}

export function isProviderKind(provider: string | null): provider is ProviderKind {
  return provider === 'virtualbox' || provider === 'vmware_desktop'
}

/** Starting point for an environment that has no Vagrantfile yet. */
export function starterModel(provider: string | null): VagrantConfig {
  const settings = emptySettings()
  settings.box = DEFAULT_BOX
  if (isProviderKind(provider)) settings.providers.push({ provider, cpus: 2, memoryMb: 2048 })
  return { defaults: settings, machines: [] }
}

/** A deep copy that drops Vue's reactive proxies. */
export function cloneModel<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

export function sameModel(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

/** First message per field path. */
export function errorsByPath(errors: FieldError[]): Record<string, string> {
  const out: Record<string, string> = {}
  for (const e of errors) out[e.path] ??= e.message
  return out
}

/** A name not used by any machine yet: machine1, machine2, … */
export function nextMachineName(machines: MachineDefinition[]): string {
  const taken = new Set(machines.map((m) => m.name.toLowerCase()))
  let n = machines.length + 1
  while (taken.has(`machine${n}`)) n++
  return `machine${n}`
}

export function newMachine(machines: MachineDefinition[]): MachineDefinition {
  return {
    name: nextMachineName(machines),
    primary: false,
    autostart: true,
    settings: emptySettings(),
  }
}

/** A forwarded port whose host port isn't used anywhere in the file yet. */
export function newForwardedPort(config: VagrantConfig): ForwardedPort {
  const used = new Set(
    [config.defaults, ...config.machines.map((m) => m.settings)].flatMap((s) =>
      s.forwardedPorts.map((p) => p.host),
    ),
  )
  let host = 8080
  while (used.has(host)) host++
  return { guest: 80, host, hostIp: '127.0.0.1', protocol: null }
}

export function newProvisioner(): ShellProvisioner {
  return { source: { kind: 'inline', script: '' }, run: null, privileged: null }
}

/** Moves item `index` by `delta` places, if that stays inside the list. */
export function move<T>(list: T[], index: number, delta: -1 | 1): void {
  const to = index + delta
  if (to < 0 || to >= list.length) return
  const [item] = list.splice(index, 1)
  list.splice(to, 0, item!)
}

/** Empty input means "not set" for optional text settings. */
export function optionalText(value: string): string | null {
  return value.trim() === '' ? null : value
}

/** Whole numbers only; anything else means "not set". */
export function optionalNumber(value: string): number | null {
  const trimmed = value.trim()
  return /^\d+$/.test(trimmed) ? Number(trimmed) : null
}
