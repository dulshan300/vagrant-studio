import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ProjectNotes from './ProjectNotes.vue'
import { notesApi } from '@/services/workspace'
import type { ProjectNote } from '@/types/workspace'

vi.mock('@/services/app', () => ({ hasBackend: () => true }))
vi.mock('@/services/workspace', () => ({
  notesApi: { list: vi.fn(), create: vi.fn(), update: vi.fn(), remove: vi.fn() },
}))

function note(id: string, title: string, body: string): ProjectNote {
  return {
    id,
    projectId: 'p1',
    title,
    body,
    createdAt: '2026-10-30T10:00:00Z',
    updatedAt: '2026-10-30T10:00:00Z',
  }
}

const button = (wrapper: ReturnType<typeof mount>, text: string) =>
  wrapper.findAll('button').find((b) => b.text().includes(text))!

describe('ProjectNotes', () => {
  afterEach(() => vi.clearAllMocks())

  it('shows note text as text, never as HTML', async () => {
    vi.mocked(notesApi.list).mockResolvedValue([
      note('n1', 'Logins', '<img src=x onerror=alert(1)>'),
    ])
    const wrapper = mount(ProjectNotes, { props: { projectId: 'p1' } })
    await flushPromises()
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.text()).toContain('<img src=x onerror=alert(1)>')
  })

  it('creates and edits notes', async () => {
    vi.mocked(notesApi.list).mockResolvedValue([note('n1', 'Ports', 'web: 8080')])
    vi.mocked(notesApi.create).mockResolvedValue(note('n2', 'Logins', 'vagrant/vagrant'))
    vi.mocked(notesApi.update).mockResolvedValue(note('n1', 'Ports', 'web: 8081'))
    const wrapper = mount(ProjectNotes, { props: { projectId: 'p1' } })
    await flushPromises()

    await button(wrapper, 'New note').trigger('click')
    await wrapper.get('#note-title').setValue('Logins')
    await wrapper.get('#note-body').setValue('vagrant/vagrant')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(notesApi.create).toHaveBeenCalledWith('p1', 'Logins', 'vagrant/vagrant')
    expect(wrapper.find('form').exists()).toBe(false)

    await button(wrapper, 'Ports').trigger('click')
    expect((wrapper.get('#note-body').element as HTMLTextAreaElement).value).toBe('web: 8080')
    await wrapper.get('#note-body').setValue('web: 8081')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(notesApi.update).toHaveBeenCalledWith('p1', 'n1', 'Ports', 'web: 8081')
    expect(wrapper.text()).toContain('web: 8081')
  })

  it('shows a validation error next to the title', async () => {
    vi.mocked(notesApi.list).mockResolvedValue([])
    vi.mocked(notesApi.create).mockRejectedValue({
      kind: 'validation',
      message: 'Use at most 200 characters.',
      field: 'title',
    })
    const wrapper = mount(ProjectNotes, { props: { projectId: 'p1' } })
    await flushPromises()
    await button(wrapper, 'New note').trigger('click')
    await wrapper.get('#note-title').setValue('x')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toBe('Use at most 200 characters.')
  })
})
