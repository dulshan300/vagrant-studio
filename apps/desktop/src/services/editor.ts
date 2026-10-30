import { invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type { DetectedEditor } from '@/types/app'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const editorApi = {
  /** Known code editors installed on this computer, in order of preference. */
  list: () => call<DetectedEditor[]>('list_editors'),
  /** Opens the project folder, or an environment's folder, in the code editor; resolves to its name. */
  open: (projectId: string, environmentId?: string) =>
    call<string>('open_in_editor', { projectId, environmentId: environmentId ?? null }),
}
