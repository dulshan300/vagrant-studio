import { Channel, invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

/** Terminal size in character cells. Mirrors `Size` in crates/terminal. */
export interface TerminalSize {
  cols: number
  rows: number
}

/** Mirrors `TerminalEvent` in src-tauri/src/commands/terminal.rs. */
export type TerminalEvent =
  | { kind: 'output'; data: string }
  /** `code` is null when the session was closed from the app. */
  | { kind: 'exit'; code: number | null }

/** Mirrors `TerminalSession` in src-tauri/src/commands/terminal.rs. */
export interface TerminalSession {
  id: string
  title: string
}

export const terminalApi = {
  /** Opens `vagrant ssh` for a running machine in a new terminal window; resolves to its name. */
  openSsh: (environmentId: string, machine: string) =>
    call<string>('open_ssh_terminal', { environmentId, machine }),
  /** Starts `vagrant ssh` in the app's terminal panel; output arrives through `onEvent`. */
  startSsh: (
    environmentId: string,
    machine: string,
    size: TerminalSize,
    onEvent: (event: TerminalEvent) => void,
  ) => {
    if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
    const channel = new Channel<TerminalEvent>()
    channel.onmessage = onEvent
    return invoke<TerminalSession>('start_ssh_session', {
      environmentId,
      machine,
      size,
      onEvent: channel,
    })
  },
  write: (id: string, data: string) => call<void>('terminal_write', { id, data }),
  resize: (id: string, size: TerminalSize) => call<void>('terminal_resize', { id, size }),
  close: (id: string) => call<void>('terminal_close', { id }),
}
