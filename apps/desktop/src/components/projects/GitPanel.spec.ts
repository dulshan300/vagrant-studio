import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import GitPanel from './GitPanel.vue'
import { gitApi } from '@/services/git'
import type { GitStatus, RepoStatus } from '@/types/git'

vi.mock('@/services/git', () => ({
  gitApi: {
    log: vi.fn(),
    init: vi.fn(),
    stage: vi.fn(),
    unstage: vi.fn(),
    commit: vi.fn(),
  },
}))

const repo = (overrides: Partial<RepoStatus> = {}): GitStatus => ({
  state: 'repository',
  branch: 'main',
  commit: '1a2b3c4',
  upstream: null,
  ahead: null,
  behind: null,
  staged: 1,
  modified: 1,
  untracked: 1,
  conflicted: 0,
  root: 'D:/Dev/my-test',
  files: [
    { path: 'Vagrantfile', origPath: null, kind: 'changed', index: 'M', worktree: '.' },
    { path: 'notes.md', origPath: null, kind: 'changed', index: '.', worktree: 'M' },
    { path: 'scripts/', origPath: null, kind: 'untracked', index: '?', worktree: '?' },
  ],
  ...overrides,
})

function mountPanel(status: GitStatus | null) {
  return mount(GitPanel, {
    props: { projectId: 'p1', projectRoot: 'D:\\Dev\\my-test', status, loading: false },
  })
}

describe('GitPanel', () => {
  beforeEach(() => {
    vi.mocked(gitApi.log).mockResolvedValue([
      { id: '1a2b3c4', author: 'Dev', date: '2026-10-30T10:00:00Z', subject: 'Add Vagrantfile' },
    ])
  })
  afterEach(() => vi.clearAllMocks())

  it('lists changes and history', async () => {
    const wrapper = mountPanel(repo())
    await flushPromises()
    expect(wrapper.text()).toContain('Changes (3)')
    expect(wrapper.text()).toContain('scripts/')
    expect(wrapper.text()).toContain('Add Vagrantfile')
    const box = wrapper.get('[aria-label="Unstage Vagrantfile"]').element as HTMLInputElement
    expect(box.checked).toBe(true)
  })

  it('stages, unstages and commits through the backend', async () => {
    const after = repo({ staged: 2 })
    vi.mocked(gitApi.stage).mockResolvedValue(after)
    vi.mocked(gitApi.unstage).mockResolvedValue(after)
    vi.mocked(gitApi.commit).mockResolvedValue(repo({ staged: 0, files: [], commit: '5d6e7f8' }))
    const wrapper = mountPanel(repo())
    await flushPromises()

    await wrapper.get('[aria-label="Stage notes.md"]').trigger('change')
    await flushPromises()
    expect(gitApi.stage).toHaveBeenCalledWith('p1', ['notes.md'])
    expect(wrapper.emitted('update')![0]).toEqual([after])

    await wrapper.get('[aria-label="Unstage Vagrantfile"]').trigger('change')
    await flushPromises()
    expect(gitApi.unstage).toHaveBeenCalledWith('p1', ['Vagrantfile'])

    const commitButton = () => wrapper.findAll('button').find((b) => b.text().startsWith('Commit'))!
    expect(commitButton().attributes('disabled')).toBeDefined()
    await wrapper.get('#commit-message').setValue('Add notes')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(gitApi.commit).toHaveBeenCalledWith('p1', 'Add notes')
    expect((wrapper.get('#commit-message').element as HTMLTextAreaElement).value).toBe('')
  })

  it('offers to initialize a folder that is not a repository', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(true)
    vi.mocked(gitApi.init).mockResolvedValue({ status: repo(), gitignoreCreated: true })
    const wrapper = mountPanel({ state: 'notRepository' })
    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Initialize Git'))!
      .trigger('click')
    await flushPromises()
    expect(gitApi.init).toHaveBeenCalledWith('p1')
    expect(wrapper.text()).toContain('Repository created with a .gitignore for Vagrant.')
  })

  it('shows Git errors and missing Git', () => {
    expect(mountPanel({ state: 'unavailable' }).text()).toContain("Git isn't installed")
    expect(mountPanel({ state: 'error', message: 'dubious ownership' }).text()).toContain(
      'dubious ownership',
    )
  })
})
