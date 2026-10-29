import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { nextTick } from 'vue'
import ProvisionMachinesDialog from './ProvisionMachinesDialog.vue'
import { operationsApi } from '@/services/operations'
import type { ProvisionCommand } from '@/lib/provisioning'
import type { Operation } from '@/types/operations'
import type { ProvisionerList, ShellProvisioner } from '@/types/vagrantfile'

vi.mock('@/services/operations', () => ({
  operationsApi: { provisioners: vi.fn(), runLifecycle: vi.fn() },
  onOperationEvent: vi.fn(),
}))

function prov(
  name: string | null,
  run: ShellProvisioner['run'] = null,
  shared = false,
): ShellProvisioner & { shared: boolean } {
  return { name, source: { kind: 'inline', script: 'echo\n' }, run, privileged: null, shared }
}

const list: ProvisionerList = {
  machines: [
    {
      machine: 'web',
      provisioners: [
        prov(null, null, true),
        prov('hello', null, true),
        prov('setup'),
        prov('fail', 'never'),
      ],
    },
  ],
  unavailable: null,
}

const op = { id: 'op1', status: 'queued' } as Operation

async function mountDialog(commands: ProvisionCommand[]) {
  const wrapper = mount(ProvisionMachinesDialog, {
    props: { open: false, environmentId: 'e1', machines: ['web'], commands },
    global: { plugins: [createPinia()] },
    attachTo: document.body,
  })
  await wrapper.setProps({ open: true })
  await flushPromises()
  await nextTick()
  return wrapper
}

const byLabel = (label: string) =>
  document.body.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!
const radio = (name: string, value: string) =>
  document.body.querySelector<HTMLInputElement>(`input[name="${name}"][value="${value}"]`)!
const runButton = () =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Run')!

describe('ProvisionMachinesDialog', () => {
  let wrapper: Awaited<ReturnType<typeof mountDialog>> | undefined

  beforeEach(() => {
    vi.mocked(operationsApi.provisioners).mockResolvedValue(list)
    vi.mocked(operationsApi.runLifecycle).mockResolvedValue(op)
  })

  afterEach(() => {
    wrapper?.unmount()
    vi.clearAllMocks()
  })

  it('runs every provisioner with plain vagrant provision', async () => {
    wrapper = await mountDialog(['provision', 'reload'])
    expect(document.body.textContent).toContain('1 other provisioner')
    runButton().click()
    await flushPromises()
    expect(operationsApi.runLifecycle).toHaveBeenCalledWith('e1', 'provision', ['web'], {
      provisioning: { kind: 'default' },
    })
  })

  it('runs only the chosen named provisioners, in Vagrantfile order', async () => {
    wrapper = await mountDialog(['provision', 'reload'])
    radio('provision-command', 'reload').click()
    radio('provision-mode', 'only').click()
    await nextTick()
    expect(runButton().disabled).toBe(true)
    byLabel('Run fail').click()
    await nextTick()
    byLabel('Run hello').click()
    await nextTick()
    runButton().click()
    await flushPromises()
    expect(operationsApi.runLifecycle).toHaveBeenCalledWith('e1', 'reload', ['web'], {
      provisioning: { kind: 'only', names: ['hello', 'fail'] },
    })
  })

  it('adds --provision when starting machines', async () => {
    wrapper = await mountDialog(['up'])
    expect(document.body.querySelector('input[name="provision-command"]')).toBeNull()
    runButton().click()
    await flushPromises()
    expect(operationsApi.runLifecycle).toHaveBeenCalledWith('e1', 'up', ['web'], {
      provisioning: { kind: 'all' },
    })
  })

  it('explains when the Vagrantfile can not be read', async () => {
    vi.mocked(operationsApi.provisioners).mockResolvedValue({
      machines: [],
      unavailable: 'The Vagrantfile can’t be read here.',
    })
    wrapper = await mountDialog(['provision'])
    expect(radio('provision-mode', 'only').disabled).toBe(true)
    expect(document.body.textContent).toContain('can’t be read here')
  })
})
