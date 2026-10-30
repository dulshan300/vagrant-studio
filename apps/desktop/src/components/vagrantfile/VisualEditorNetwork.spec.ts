import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import VisualEditor from './VisualEditor.vue'
import { starterModel } from '@/lib/vagrantfile'
import { networkApi } from '@/services/network'

vi.mock('@/services/app', () => ({ hasBackend: () => true }))
vi.mock('@/services/network', () => ({
  networkApi: {
    checkHostPorts: vi.fn(),
    listBridgedInterfaces: vi.fn(),
    listForwardedPorts: vi.fn(() => Promise.resolve({ ports: [], unchecked: [] })),
  },
}))

describe('VisualEditor network settings', () => {
  afterEach(() => {
    vi.useRealTimers()
    vi.clearAllMocks()
  })

  it('adds a public network bridged to the first adapter VirtualBox offers', async () => {
    vi.mocked(networkApi.listBridgedInterfaces).mockResolvedValue([
      { name: 'Intel(R) Wi-Fi 6 AX200 160MHz', wireless: true },
    ])
    vi.mocked(networkApi.checkHostPorts).mockResolvedValue([])
    const model = starterModel('virtualbox')
    const wrapper = mount(VisualEditor, {
      props: { modelValue: model, 'onUpdate:modelValue': () => undefined, errors: {} },
    })
    await flushPromises()
    const add = wrapper.findAll('h3').find((h) => h.text().startsWith('Public networks'))!
    await add.get('button').trigger('click')
    expect(model.defaults.publicNetworks).toEqual([
      { ip: null, bridge: 'Intel(R) Wi-Fi 6 AX200 160MHz' },
    ])
    expect(wrapper.find('datalist option').attributes('value')).toBe(
      'Intel(R) Wi-Fi 6 AX200 160MHz',
    )
    expect(wrapper.text()).not.toContain('No host adapter set')

    await wrapper.get('[aria-label="Public network 1 host adapter"]').setValue('')
    expect(model.defaults.publicNetworks[0]!.bridge).toBeNull()
    expect(wrapper.text()).toContain('No host adapter set')
  })

  it('warns about forwarded TCP host ports that are already in use', async () => {
    vi.useFakeTimers()
    vi.mocked(networkApi.listBridgedInterfaces).mockResolvedValue([])
    vi.mocked(networkApi.checkHostPorts).mockResolvedValue([true, false])
    const model = starterModel(null)
    model.defaults.forwardedPorts.push(
      { guest: 80, host: 8080, hostIp: null, protocol: null },
      { guest: 443, host: 8443, hostIp: '127.0.0.1', protocol: null },
      { guest: 53, host: 8080, hostIp: null, protocol: 'udp' },
    )
    const wrapper = mount(VisualEditor, {
      props: { modelValue: model, 'onUpdate:modelValue': () => undefined, errors: {} },
    })
    await vi.runAllTimersAsync()
    await flushPromises()
    expect(networkApi.checkHostPorts).toHaveBeenCalledWith([
      { hostIp: null, port: 8080 },
      { hostIp: '127.0.0.1', port: 8443 },
    ])
    const warnings = wrapper.findAll('p').filter((p) => p.text().includes('already uses host port'))
    // Only the TCP entry on 8080; the UDP one isn't checked by Vagrant.
    expect(warnings).toHaveLength(1)
    expect(warnings[0]!.text()).toContain('host port 8080')
  })

  it('names other environments that forward the same host port', async () => {
    vi.mocked(networkApi.listBridgedInterfaces).mockResolvedValue([])
    vi.mocked(networkApi.checkHostPorts).mockResolvedValue([])
    const use = (environmentId: string, host: number) => ({
      projectId: 'p2',
      projectName: 'Shop',
      environmentId,
      environmentName: 'default',
      machine: 'web',
      hostIp: null,
      host,
      guest: 80,
      protocol: 'tcp' as const,
    })
    vi.mocked(networkApi.listForwardedPorts).mockResolvedValue({
      // The edited environment's own saved ports are ignored.
      ports: [use('e2', 8080), use('e1', 8443)],
      unchecked: [],
    })
    const model = starterModel(null)
    model.defaults.forwardedPorts.push(
      { guest: 80, host: 8080, hostIp: '127.0.0.1', protocol: null },
      { guest: 443, host: 8443, hostIp: null, protocol: null },
    )
    const wrapper = mount(VisualEditor, {
      props: {
        modelValue: model,
        'onUpdate:modelValue': () => undefined,
        errors: {},
        environmentId: 'e1',
      },
    })
    await flushPromises()
    const notes = wrapper.findAll('p').filter((p) => p.text().startsWith('Also forwarded by'))
    expect(notes.map((n) => n.text())).toEqual([
      'Also forwarded by Shop · default (web). Only one of them can be running at a time.',
    ])
  })
})
