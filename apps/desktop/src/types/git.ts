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
  /** Changes in the project folder, paths relative to the repository root. */
  files: FileChange[]
  /** Repository top-level folder as Git prints it (forward slashes). */
  root: string | null
}

export type ChangeKind = 'changed' | 'renamed' | 'conflicted' | 'untracked'

export interface FileChange {
  path: string
  origPath: string | null
  kind: ChangeKind
  /** Git's status letter in the index; '.' when unchanged there, '?' when untracked. */
  index: string
  /** The same for the work tree. */
  worktree: string
}

export interface CommitSummary {
  id: string
  author: string
  /** ISO 8601 with offset. */
  date: string
  subject: string
}

export interface GitInit {
  status: GitStatus
  /** False when the folder already had a .gitignore, which is kept. */
  gitignoreCreated: boolean
}
