import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import SnapshotsDialog from './SnapshotsDialog.vue'
import { operationsApi } from '@/services/operations'
import type { Operation } from '@/types/operations'

vi.mock('@/services/operations', () => ({
  operationsApi: {
    snapshots: vi.fn(),
    saveSnapshot: vi.fn(),
    requestSnapshotConfirmation: vi.fn(),
    changeSnapshot: vi.fn(),
  },
  onOperationEvent: vi.fn(),
}))

const op = { id: 'o1', status: 'queued' } as Operation

let mounted: VueWrapper | null = null

async function mountDialog() {
  vi.mocked(operationsApi.snapshots).mockResolvedValue({ machine: 'web', names: ['base'] })
  const wrapper = mount(SnapshotsDialog, {
    props: { open: false, environmentId: 'e1', machine: 'web' },
    global: { plugins: [createPinia()] },
    attachTo: document.body,
  })
  await wrapper.setProps({ open: true })
  await flushPromises()
  await nextTick()
  mounted = wrapper
}

const buttons = () => [...document.body.querySelectorAll('button')]
const button = (text: string) => buttons().find((b) => b.textContent?.trim() === text)!

describe('SnapshotsDialog', () => {
  afterEach(() => {
    vi.clearAllMocks()
    mounted?.unmount()
  })

  it('lists snapshots and saves a new one', async () => {
    vi.mocked(operationsApi.saveSnapshot).mockResolvedValue(op)
    await mountDialog()
    expect(document.body.textContent).toContain('base')
    const input = document.body.querySelector<HTMLInputElement>('[aria-label="Snapshot name"]')!
    input.value = 'after setup '
    input.dispatchEvent(new Event('input'))
    await nextTick()
    document.body.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }))
    await flushPromises()
    expect(operationsApi.saveSnapshot).toHaveBeenCalledWith('e1', 'web', 'after setup')
  })

  it('restores only after the snapshot name is typed', async () => {
    vi.mocked(operationsApi.requestSnapshotConfirmation).mockResolvedValue({
      token: 't',
      phrase: 'base',
      expiresAt: '2026-10-30T00:00:00Z',
    })
    vi.mocked(operationsApi.changeSnapshot).mockResolvedValue(op)
    await mountDialog()
    button('Restore').click()
    await flushPromises()
    const confirm = buttons()
      .filter((b) => b.textContent?.trim() === 'Restore')
      .at(-1)!
    expect(confirm.disabled).toBe(true)
    const input = document.body.querySelector<HTMLInputElement>('#snapshot-confirm')!
    input.value = 'base'
    input.dispatchEvent(new Event('input'))
    await nextTick()
    expect(confirm.disabled).toBe(false)
    confirm.click()
    await flushPromises()
    expect(operationsApi.changeSnapshot).toHaveBeenCalledWith('e1', 'web', 'restore', 'base', {
      token: 't',
      typed: 'base',
    })
  })
})
