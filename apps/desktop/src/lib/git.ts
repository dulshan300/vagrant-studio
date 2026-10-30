import type { RepoStatus } from '@/types/git'

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
