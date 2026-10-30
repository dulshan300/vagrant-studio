import { describe, expect, it } from 'vitest'
import {
  branchLabel,
  changeCount,
  changeLabel,
  changeSummary,
  displayPath,
  folderInRepo,
  stageState,
} from './git'
import type { FileChange, RepoStatus } from '@/types/git'

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
  files: [],
  root: null,
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

describe('file changes', () => {
  const file = (index: string, worktree: string, kind: FileChange['kind'] = 'changed') => ({
    path: 'env/Vagrantfile',
    origPath: null,
    kind,
    index,
    worktree,
  })

  it('knows how much of a change is staged', () => {
    expect(stageState(file('M', '.'))).toBe('staged')
    expect(stageState(file('M', 'M'))).toBe('partly')
    expect(stageState(file('.', 'M'))).toBe('unstaged')
    expect(stageState(file('?', '?', 'untracked'))).toBe('unstaged')
  })

  it('labels changes', () => {
    expect(changeLabel(file('A', '.')).text).toBe('Added')
    expect(changeLabel(file('.', 'D')).letter).toBe('D')
    expect(changeLabel(file('?', '?', 'untracked')).letter).toBe('U')
    expect(changeLabel(file('U', 'U', 'conflicted')).text).toBe('Merge conflict')
  })

  it('shows paths relative to the project folder', () => {
    expect(folderInRepo('D:/Dev/repo', 'D:\\Dev\\repo')).toBe('')
    expect(folderInRepo('d:/dev/repo', 'D:\\Dev\\Repo\\my-test')).toBe('my-test')
    expect(folderInRepo('D:/Other', 'D:\\Dev\\repo')).toBeNull()
    expect(folderInRepo(null, 'D:\\Dev\\repo')).toBeNull()
    expect(displayPath('my-test/Vagrantfile', 'my-test')).toBe('Vagrantfile')
    expect(displayPath('Vagrantfile', '')).toBe('Vagrantfile')
  })
})
