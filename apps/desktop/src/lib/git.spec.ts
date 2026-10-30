import { describe, expect, it } from 'vitest'
import { branchLabel, changeCount, changeSummary } from './git'
import type { RepoStatus } from '@/types/git'

const base: RepoStatus = {
  branch: 'main',
  commit: '1a2b3c4',
  upstream: null,
  ahead: null,
  behind: null,
  staged: 0,
  modified: 0,
  untracked: 0,
  conflicted: 0,
}

describe('git labels', () => {
  it('names the branch, a detached head or an empty repository', () => {
    expect(branchLabel(base)).toBe('main')
    expect(branchLabel({ ...base, branch: null })).toBe('detached at 1a2b3c4')
    expect(branchLabel({ ...base, branch: null, commit: null })).toBe('no commits yet')
  })

  it('summarizes changes', () => {
    expect(changeSummary(base)).toBe('No uncommitted changes')
    const s = { ...base, staged: 1, modified: 2, untracked: 3, conflicted: 1 }
    expect(changeCount(s)).toBe(7)
    expect(changeSummary(s)).toBe('1 conflicted, 1 staged, 2 modified, 3 untracked')
  })
})
