import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import EnvironmentMachines from './EnvironmentMachines.vue'
import { terminalApi } from '@/services/terminal'
import { useOperationsStore } from '@/stores/operations'
import type { EnvironmentStatus, HostReadiness } from '@/types/operations'
import type { Environment } from '@/types/projects'

vi.mock('@/services/app', () => ({ hasBackend: () => false }))
vi.mock('@/services/terminal', () => ({ terminalApi: { openSsh: vi.fn() } }))

const environment = { id: 'e1', name: 'default' } as Environment

const status: EnvironmentStatus = {
  environmentId: 'e1',
  operationId: null,
  observedAt: '2026-10-29T10:00:00Z',
  machines: [
    { name: 'web', provider: 'virtualbox', state: 'running', stateShort: null, stateLong: null },
    { name: 'db', provider: 'virtualbox', state: 'poweroff', stateShort: null, stateLong: null },
    { name: 'app', provider: 'docker', state: 'running', stateShort: null, stateLong: null },
  ],
  error: null,
}

function mountMachines() {
  const pinia = createPinia()
  setActivePinia(pinia)
  useOperationsStore().readiness = {
    providers: [
      { id: 'virtualbox', capabilities: ['lifecycle.basic', 'ssh'] },
      { id: 'docker', capabilities: ['lifecycle.basic'] },
    ],
  } as unknown as HostReadiness
  return mount(EnvironmentMachines, {
    props: { environment, status },
    global: { plugins: [pinia] },
  })
}

describe('EnvironmentMachines', () => {
  afterEach(() => vi.clearAllMocks())

  it('offers SSH only for running machines whose provider supports it', () => {
    const wrapper = mountMachines()
    expect(wrapper.find('[aria-label="SSH into web"]').exists()).toBe(true)
    expect(wrapper.find('[aria-label="SSH into db"]').exists()).toBe(false)
    expect(wrapper.find('[aria-label="SSH into app"]').exists()).toBe(false)
  })

  it('opens a terminal and shows why it could not', async () => {
    vi.mocked(terminalApi.openSsh).mockRejectedValueOnce({
      kind: 'unavailable',
      message: 'No terminal application was found.',
    })
    const wrapper = mountMachines()
    await wrapper.find('[aria-label="SSH into web"]').trigger('click')
    await flushPromises()
    expect(terminalApi.openSsh).toHaveBeenCalledWith('e1', 'web')
    expect(wrapper.find('[role="alert"]').text()).toContain('No terminal application was found.')
  })
})
