import { invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type { BridgedInterface, ForwardedPorts, HostPort } from '@/types/vagrantfile'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const networkApi = {
  /** Per port: something on this computer already accepts TCP connections on it. */
  checkHostPorts: (ports: HostPort[]) => call<boolean[]>('check_host_ports', { ports }),
  /** Adapters a VirtualBox public network can bridge to; empty without VirtualBox. */
  listBridgedInterfaces: () => call<BridgedInterface[]>('list_bridged_interfaces'),
  /** Forwarded ports of every registered environment, from their Vagrantfiles. */
  listForwardedPorts: () => call<ForwardedPorts>('list_forwarded_ports'),
}
