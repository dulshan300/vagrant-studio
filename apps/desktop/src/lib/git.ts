import type { FileChange, RepoStatus } from '@/types/git'

/** "main", or "detached at 1a2b3c4" / "no commits yet". */
export function branchLabel(s: RepoStatus): string {
  if (s.branch) return s.branch
  return s.commit ? `detached at ${s.commit}` : 'no commits yet'
}

export function changeCount(s: RepoStatus): number {
  return s.staged + s.modified + s.untracked + s.conflicted
}

/** One line per non-zero count, for the tooltip. */
export function changeSummary(s: RepoStatus): string {
  const parts = [
    [s.conflicted, 'conflicted'],
    [s.staged, 'staged'],
    [s.modified, 'modified'],
    [s.untracked, 'untracked'],
  ] as const
  const lines = parts.filter(([n]) => n > 0).map(([n, what]) => `${n} ${what}`)
  return lines.length ? lines.join(', ') : 'No uncommitted changes'
}

export type StageState = 'staged' | 'partly' | 'unstaged'

/** Whether a file's changes are in the next commit: all, some, or none. */
export function stageState(f: FileChange): StageState {
  if (f.kind === 'untracked' || f.index === '.') return 'unstaged'
  return f.worktree === '.' ? 'staged' : 'partly'
}

const LETTERS: Record<string, string> = {
  M: 'Modified',
  T: 'Type changed',
  A: 'Added',
  D: 'Deleted',
  R: 'Renamed',
  C: 'Copied',
}

/** A short letter and its meaning for the file list. */
export function changeLabel(f: FileChange): { letter: string; text: string } {
  if (f.kind === 'untracked') return { letter: 'U', text: 'New file, not tracked yet' }
  if (f.kind === 'conflicted') return { letter: '!', text: 'Merge conflict' }
  const letter = f.index !== '.' ? f.index : f.worktree
  return { letter, text: LETTERS[letter] ?? 'Changed' }
}

/** Git paths always use '/'; Windows paths may use '\' and any letter case. */
function normalize(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()
}

/**
 * The project folder relative to the repository root ('' when they are the
 * same, null when the root is unknown or elsewhere), to shorten listed paths.
 */
export function folderInRepo(repoRoot: string | null, projectRoot: string): string | null {
  if (!repoRoot) return null
  const root = normalize(repoRoot)
  const folder = normalize(projectRoot)
  if (folder === root) return ''
  return folder.startsWith(`${root}/`)
    ? projectRoot.replace(/\\/g, '/').slice(root.length + 1)
    : null
}

/** A repository-relative path shown relative to the project folder. */
export function displayPath(path: string, prefix: string | null): string {
  if (!prefix) return path
  return path.toLowerCase().startsWith(`${prefix.toLowerCase()}/`)
    ? path.slice(prefix.length + 1)
    : path
}
