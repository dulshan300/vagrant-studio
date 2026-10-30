/** Mirrors `vagrant_studio_git::GitStatus` (serde tag `state`). */
export type GitStatus =
  | { state: 'unavailable' }
  | { state: 'notRepository' }
  | ({ state: 'repository' } & RepoStatus)
  | { state: 'error'; message: string }

export interface RepoStatus {
  /** null when HEAD is detached. */
  branch: string | null
  /** Abbreviated commit id; null before the first commit. */
  commit: string | null
  upstream: string | null
  ahead: number | null
  behind: number | null
  staged: number
  modified: number
  untracked: number
  conflicted: number
}
