import type { Capability, ProviderCheck } from '@/types/operations'

/** Display names for capability flags (docs/provider-capabilities.md). */
export const CAPABILITY_LABEL: Record<Capability, string> = {
  'lifecycle.basic': 'Start, stop, reload, destroy',
  'lifecycle.suspend': 'Suspend and resume',
  'provision.shell': 'Shell provisioning',
  'network.forwarded_port': 'Forwarded ports',
  'network.private': 'Private networks',
  'network.public': 'Public networks',
  'synced_folder.default': 'Synced folders',
  'config.cpu_memory': 'CPU and memory settings',
  ssh: 'SSH',
  snapshot: 'Snapshots',
  export: 'Export to a box file',
}

export const SUPPORT_LABEL: Record<ProviderCheck['support'], string> = {
  verified: 'Verified',
  detected: 'Detected, not yet verified',
  unavailable: 'Not available',
}
