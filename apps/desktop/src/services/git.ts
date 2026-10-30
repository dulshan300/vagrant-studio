import { invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type { CommitSummary, GitInit, GitStatus } from '@/types/git'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const gitApi = {
  installed: () => call<boolean>('is_git_installed'),
  /** `git status` of the project folder; never changes the repository. */
  status: (projectId: string) => call<GitStatus>('get_git_status', { projectId }),
  log: (projectId: string) => call<CommitSummary[]>('get_git_log', { projectId }),
  /** `git init` plus a .gitignore for Vagrant (kept if one exists). Nothing is committed. */
  init: (projectId: string) => call<GitInit>('init_git_repository', { projectId }),
  /** Paths as listed in the status (relative to the repository root). */
  stage: (projectId: string, paths: string[]) => call<GitStatus>('git_stage', { projectId, paths }),
  unstage: (projectId: string, paths: string[]) =>
    call<GitStatus>('git_unstage', { projectId, paths }),
  commit: (projectId: string, message: string) =>
    call<GitStatus>('git_commit', { projectId, message }),
}
