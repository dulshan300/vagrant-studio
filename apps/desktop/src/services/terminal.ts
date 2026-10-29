import { invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const terminalApi = {
  /** Opens `vagrant ssh` for a running machine in a new terminal window; resolves to its name. */
  openSsh: (environmentId: string, machine: string) =>
    call<string>('open_ssh_terminal', { environmentId, machine }),
}
