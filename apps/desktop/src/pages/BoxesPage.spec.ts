import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import BoxesPage from './BoxesPage.vue'
import { boxesApi } from '@/services/boxes'
import type { BoxStore, BoxUser } from '@/types/boxes'
import type { Operation } from '@/types/operations'

vi.mock('@/services/boxes', () => ({
  boxesApi: {
    list: vi.fn(),
    checkUpdates: vi.fn(),
    add: vi.fn(),
    update: vi.fn(),
    requestRemoval: vi.fn(),
    remove: vi.fn(),
  },
  pickBoxFile: vi.fn(),
}))

const user: BoxUser = {
  machine: 'default',
  provider: 'vmware_desktop',
  state: 'running',
  folder: 'D:/Projects/alma-lab',
  folderExists: true,
  projectId: 'p1',
  projectName: 'Alma lab',
  environmentId: 'e1',
  environmentName: 'default',
}

const store: BoxStore = {
  boxes: [
    {
      name: 'almalinux/9',
      provider: 'vmware_desktop',
      version: '9.6.20250522',
      architecture: 'amd64',
      users: [user, { ...user, folder: '/mnt/d/gone', folderExists: false, projectId: null }],
    },
    {
      name: 'hashicorp/bionic64',
      provider: 'virtualbox',
      version: '1.0.282',
      architecture: 'n/a',
      users: [],
    },
  ],
  usageError: null,
}

function operation(overrides: Partial<Operation>): Operation {
  return {
    id: 'op1',
    projectId: null,
    environmentId: null,
    machineName: null,
    targetLabel: 'Box · almalinux/9',
    operationType: 'box_remove',
    status: 'queued',
    commandLine: 'vagrant box remove',
    workingDir: null,
    startedAt: null,
    finishedAt: null,
    exitCode: null,
    logPath: null,
    errorSummary: null,
    createdAt: '2026-10-29T00:00:00Z',
    ...overrides,
  }
}

async function mountPage() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/boxes', component: BoxesPage },
      { path: '/projects/:id', name: 'project', component: { template: '<div />' } },
    ],
  })
  await router.push('/boxes')
  await router.isReady()
  const wrapper = mount(BoxesPage, {
    global: { plugins: [router, createPinia()] },
    attachTo: document.body,
  })
  await flushPromises()
  return wrapper
}

const button = (root: ParentNode, text: string) =>
  [...root.querySelectorAll('button')].find((b) => b.textContent?.includes(text))!

describe('BoxesPage', () => {
  let wrapper: ReturnType<typeof mount> | undefined

  beforeEach(() => {
    vi.mocked(boxesApi.list).mockResolvedValue(store)
  })

  afterEach(() => {
    wrapper?.unmount()
    vi.clearAllMocks()
  })

  it('lists boxes with the machines that still use them', async () => {
    wrapper = await mountPage()
    const rows = wrapper.findAll('tbody tr')
    expect(rows).toHaveLength(2)
    expect(rows[0]!.text()).toContain('almalinux/9')
    expect(rows[0]!.text()).toContain('1 machine')
    expect(rows[0]!.text()).toContain('Alma lab · default')
    expect(rows[0]!.text()).not.toContain('/mnt/d/gone')
    expect(rows[1]!.text()).toContain('Not used')
    expect(rows[1]!.text()).not.toContain('n/a')
  })

  it('offers Update only for versions the check found outdated', async () => {
    vi.mocked(boxesApi.checkUpdates).mockResolvedValue([
      {
        name: 'almalinux/9',
        provider: 'vmware_desktop',
        state: 'outdated',
        current: '9.6.20250522',
        latest: '9.8.20260907',
      },
      { name: 'hashicorp/bionic64', provider: 'virtualbox', state: 'upToDate', version: '1.0.282' },
    ])
    vi.mocked(boxesApi.update).mockResolvedValue(operation({ operationType: 'box_update' }))
    wrapper = await mountPage()
    expect(wrapper.text()).not.toContain('Update')

    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Check for updates'))!
      .trigger('click')
    await flushPromises()
    const rows = wrapper.findAll('tbody tr')
    expect(rows[0]!.text()).toContain('9.8.20260907 available')
    expect(rows[1]!.text()).toContain('newest')
    const updates = wrapper.findAll('button').filter((b) => b.text().includes('Update'))
    expect(updates).toHaveLength(1)

    await updates[0]!.trigger('click')
    await flushPromises()
    expect(boxesApi.update).toHaveBeenCalledWith({
      name: 'almalinux/9',
      provider: 'vmware_desktop',
      version: '9.6.20250522',
      architecture: 'amd64',
    })
    expect(wrapper.text()).toContain('Box changes')
  })

  it('removes a box in use only after a typed confirmation, then re-reads the list', async () => {
    vi.mocked(boxesApi.requestRemoval).mockResolvedValue({
      token: 't1',
      phrase: 'almalinux/9',
      expiresAt: '2026-10-29T00:10:00Z',
    })
    vi.mocked(boxesApi.remove).mockResolvedValue(operation({}))
    wrapper = await mountPage()

    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Remove'))!
      .trigger('click')
    await flushPromises()
    const dialog = document.body.querySelector('[role="dialog"]')!
    expect(dialog.textContent).toContain('Vagrant records a machine created from this box')
    expect(dialog.textContent).toContain('Alma lab')
    expect(boxesApi.requestRemoval).toHaveBeenCalledWith({
      name: 'almalinux/9',
      provider: 'vmware_desktop',
      version: '9.6.20250522',
      architecture: 'amd64',
    })

    const confirm = button(dialog, 'Remove box')
    expect(confirm.disabled).toBe(true)
    const input = dialog.querySelector<HTMLInputElement>('#remove-box-confirm')!
    input.value = 'almalinux/9'
    input.dispatchEvent(new Event('input'))
    await flushPromises()
    expect(confirm.disabled).toBe(false)
    confirm.click()
    await flushPromises()
    expect(boxesApi.remove).toHaveBeenCalledWith(expect.objectContaining({ name: 'almalinux/9' }), {
      token: 't1',
      typed: 'almalinux/9',
    })
    expect(boxesApi.list).toHaveBeenCalledTimes(1)

    // The operation finishing (an engine event) makes the page re-read the store.
    const { useOperationsStore } = await import('@/stores/operations')
    useOperationsStore().handle({
      type: 'operation.completed',
      timestamp: '',
      operationId: 'op1',
      environmentId: null,
      machineName: null,
      payload: {
        operation: operation({ status: 'succeeded', finishedAt: '2026-10-29T00:01:00Z' }),
      },
    })
    await flushPromises()
    expect(boxesApi.list).toHaveBeenCalledTimes(2)
  })
})
