import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import OperationsPage from './OperationsPage.vue'
import OperationDrawer from '@/components/operations/OperationDrawer.vue'
import { operationsApi } from '@/services/operations'
import { projectsApi } from '@/services/projects'
import { useOperationsStore } from '@/stores/operations'
import type { LogLine, Operation } from '@/types/operations'
import type { Project, ProjectDetails } from '@/types/projects'

vi.mock('@/services/operations', () => ({
  operationsApi: {
    list: vi.fn(),
    get: vi.fn(),
    log: vi.fn(),
    environmentStatus: vi.fn(),
    revealLog: vi.fn(),
    openLogsFolder: vi.fn(),
    operationProvisioning: vi.fn(),
  },
  onOperationEvent: vi.fn(),
}))
vi.mock('@/services/projects', () => ({ projectsApi: { list: vi.fn(), get: vi.fn() } }))

function operation(overrides: Partial<Operation>): Operation {
  return {
    id: 'op1',
    projectId: 'p1',
    environmentId: 'e1',
    machineName: null,
    targetLabel: 'Web · default · web',
    operationType: 'up',
    status: 'failed',
    commandLine: 'vagrant up --no-tty web',
    workingDir: '/p',
    startedAt: '2026-10-30T10:00:00Z',
    finishedAt: '2026-10-30T10:01:00Z',
    exitCode: 1,
    logPath: '/logs/op1.log',
    errorSummary: null,
    createdAt: '2026-10-30T10:00:00Z',
    ...overrides,
  }
}

const recent = [operation({ id: 'host', projectId: null, environmentId: null, targetLabel: '' })]
const machineHistory = [operation({ id: 'op1' })]

const log: LogLine[] = [
  { kind: 'meta', text: '$ vagrant up --no-tty web' },
  { kind: 'out', text: '==> web: Booting VM...' },
  { kind: 'out', text: '    web: Warning: Connection reset. Retrying...' },
  { kind: 'out', text: '==> web: Running provisioner: shell...' },
  { kind: 'err', text: 'The SSH command responded with a non-zero exit status.' },
]

async function mountPage(path: string) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/operations', component: OperationsPage }],
  })
  await router.push(path)
  await router.isReady()
  const wrapper = mount(
    {
      components: { OperationsPage, OperationDrawer },
      template: '<OperationsPage /><OperationDrawer />',
    },
    { global: { plugins: [router, createPinia()] }, attachTo: document.body },
  )
  await flushPromises()
  return { wrapper, router }
}

const buttonIn = (root: ParentNode, label: string) =>
  [...root.querySelectorAll('button')].find(
    (b) => b.getAttribute('aria-label') === label || b.textContent?.trim().includes(label),
  )!

describe('OperationsPage', () => {
  let wrapper: ReturnType<typeof mount> | undefined

  beforeEach(() => {
    vi.mocked(operationsApi.list).mockImplementation(async (filter = {}) =>
      filter.projectId ? machineHistory : recent,
    )
    vi.mocked(operationsApi.get).mockResolvedValue(machineHistory[0]!)
    vi.mocked(operationsApi.log).mockResolvedValue(log)
    vi.mocked(operationsApi.operationProvisioning).mockResolvedValue([])
    vi.mocked(operationsApi.environmentStatus).mockResolvedValue({
      environmentId: 'e1',
      operationId: null,
      observedAt: '2026-10-30T09:00:00Z',
      machines: [
        {
          name: 'web',
          provider: 'virtualbox',
          state: 'running',
          stateShort: null,
          stateLong: null,
        },
        { name: 'db', provider: 'virtualbox', state: 'running', stateShort: null, stateLong: null },
      ],
      error: null,
    })
    vi.mocked(projectsApi.list).mockResolvedValue([{ id: 'p1', name: 'Web' } as Project])
    vi.mocked(projectsApi.get).mockResolvedValue({
      project: { id: 'p1', name: 'Web' },
      environments: [{ id: 'e1', projectId: 'p1', name: 'default' }],
    } as ProjectDetails)
    Element.prototype.scrollIntoView = vi.fn()
    Element.prototype.scrollTo = vi.fn()
  })

  afterEach(() => {
    wrapper?.unmount()
    document.body.innerHTML = ''
    vi.clearAllMocks()
  })

  it('lists history for the machine named in the URL', async () => {
    ;({ wrapper } = await mountPage('/operations?project=p1&environment=e1&machine=web'))
    expect(operationsApi.list).toHaveBeenCalledWith(
      { projectId: 'p1', environmentId: 'e1', machine: 'web' },
      200,
    )
    const rows = wrapper.findAll('tbody tr')
    expect(rows).toHaveLength(1)
    expect(rows[0]!.text()).toContain('Web · default · web')
    const selects = wrapper.findAll('select')
    expect(selects.map((s) => (s.element as HTMLSelectElement).value)).toEqual(['p1', 'e1', 'web'])
    expect(selects[2]!.findAll('option').map((o) => o.text())).toEqual([
      'All machines',
      'web',
      'db',
    ])
  })

  it('narrowing the scope updates the URL; clearing shows everything again', async () => {
    let router
    ;({ wrapper, router } = await mountPage('/operations'))
    expect(wrapper.findAll('tbody tr')[0]!.text()).toContain('Host')

    await wrapper.find('select[aria-label="Project"]').setValue('p1')
    await flushPromises()
    expect(router.currentRoute.value.query).toEqual({ project: 'p1' })
    expect(operationsApi.list).toHaveBeenLastCalledWith(
      { projectId: 'p1', environmentId: null, machine: null },
      200,
    )

    await wrapper.find('button[aria-label="Show all operations"]').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.query).toEqual({})
    expect(wrapper.findAll('tbody tr')[0]!.text()).toContain('Host')
  })

  it('searches, filters and jumps through the output', async () => {
    ;({ wrapper } = await mountPage('/operations?project=p1'))
    await useOperationsStore().open('op1')
    await flushPromises()
    const drawer = document.querySelector('[role="dialog"]')!
    const lines = () => [...drawer.querySelectorAll('[data-pos]')]
    expect(lines()).toHaveLength(5)

    const search = drawer.querySelector<HTMLInputElement>('input[type="search"]')!
    search.value = 'web:'
    search.dispatchEvent(new Event('input'))
    await flushPromises()
    expect(drawer.textContent).toContain('1 of 3')
    expect(drawer.querySelectorAll('mark')).toHaveLength(3)
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))
    await flushPromises()
    expect(drawer.textContent).toContain('2 of 3')
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true }))
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true }))
    await flushPromises()
    expect(drawer.textContent).toContain('3 of 3')

    buttonIn(drawer, 'Problems').click()
    await flushPromises()
    expect(lines().map((l) => l.textContent)).toEqual([
      '    web: Warning: Connection reset. Retrying...',
      'The SSH command responded with a non-zero exit status.',
    ])
    buttonIn(drawer, 'stderr').click()
    await flushPromises()
    expect(lines()).toHaveLength(1)

    buttonIn(drawer, 'Show log file').click()
    await flushPromises()
    expect(operationsApi.revealLog).toHaveBeenCalledWith('op1')
  })

  it('opens the logs folder', async () => {
    ;({ wrapper } = await mountPage('/operations'))
    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Logs folder'))!
      .trigger('click')
    expect(operationsApi.openLogsFolder).toHaveBeenCalled()
  })
})
