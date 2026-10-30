import { invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type { GitStatus } from '@/types/git'

export const gitApi = {
  /** Read-only `git status` of the project folder. */
  status: (projectId: string): Promise<GitStatus> =>
    hasBackend()
      ? invoke<GitStatus>('get_git_status', { projectId })
      : Promise.reject(new BackendUnavailableError()),
}
