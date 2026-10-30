import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ProjectTasks from './ProjectTasks.vue'
import { tasksApi } from '@/services/workspace'
import type { ProjectTask } from '@/types/workspace'

vi.mock('@/services/app', () => ({ hasBackend: () => true }))
vi.mock('@/services/workspace', () => ({
  tasksApi: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    remove: vi.fn(),
    reorder: vi.fn(),
  },
}))

function task(id: string, title: string, done = false, sortOrder = 0): ProjectTask {
  return {
    id,
    projectId: 'p1',
    title,
    done,
    sortOrder,
    createdAt: '2026-10-30T10:00:00Z',
    updatedAt: '2026-10-30T10:00:00Z',
    completedAt: done ? '2026-10-30T11:00:00Z' : null,
  }
}

const label = (wrapper: ReturnType<typeof mount>, text: string) =>
  wrapper.get(`[aria-label="${text}"]`)

describe('ProjectTasks', () => {
  beforeEach(() => {
    vi.mocked(tasksApi.list).mockResolvedValue([
      task('a', 'Install box', false, 0),
      task('b', 'Open port', false, 1),
      task('c', 'Read docs', true, 2),
    ])
  })
  afterEach(() => vi.clearAllMocks())

  it('lists open tasks and folds done ones away', async () => {
    const wrapper = mount(ProjectTasks, { props: { projectId: 'p1' } })
    await flushPromises()
    expect(wrapper.text()).toContain('2 open · 1 done')
    expect(wrapper.get('details summary').text()).toBe('Done (1)')
  })

  it('adds, completes and reorders tasks', async () => {
    vi.mocked(tasksApi.create).mockResolvedValue(task('d', 'Seed data', false, 3))
    vi.mocked(tasksApi.update).mockResolvedValue(task('a', 'Install box', true, 0))
    vi.mocked(tasksApi.reorder).mockResolvedValue([
      task('b', 'Open port', false, 0),
      task('a', 'Install box', false, 1),
      task('d', 'Seed data', false, 2),
      task('c', 'Read docs', true, 3),
    ])
    const wrapper = mount(ProjectTasks, { props: { projectId: 'p1' } })
    await flushPromises()

    await wrapper.get('#new-task').setValue('Seed data')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(tasksApi.create).toHaveBeenCalledWith('p1', 'Seed data')
    expect(wrapper.text()).toContain('Seed data')

    await label(wrapper, 'Move “Open port” up').trigger('click')
    await flushPromises()
    // Open tasks in their new order, then the done ones.
    expect(tasksApi.reorder).toHaveBeenCalledWith('p1', ['b', 'a', 'd', 'c'])

    await label(wrapper, 'Mark “Install box” done').trigger('change')
    await flushPromises()
    expect(tasksApi.update).toHaveBeenCalledWith('p1', 'a', { done: true })
  })

  it('renames a task in place', async () => {
    vi.mocked(tasksApi.update).mockResolvedValue(task('a', 'Install the box'))
    const wrapper = mount(ProjectTasks, { props: { projectId: 'p1' }, attachTo: document.body })
    await flushPromises()
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Install box')!
      .trigger('click')
    const input = label(wrapper, 'Rename “Install box”')
    await input.setValue('Install the box')
    await input.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(tasksApi.update).toHaveBeenCalledTimes(1)
    expect(tasksApi.update).toHaveBeenCalledWith('p1', 'a', { title: 'Install the box' })
    expect(wrapper.text()).toContain('Install the box')
    wrapper.unmount()
  })
})
