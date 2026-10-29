import { describe, expect, it } from 'vitest'
import type { InstalledBox, OutdatedBox } from '@/types/boxes'
import { activeUsers, boxRef, sortBoxes, updateStateFor } from './boxes'

function box(overrides: Partial<InstalledBox> = {}): InstalledBox {
  return {
    name: 'almalinux/9',
    provider: 'vmware_desktop',
    version: '9.6.20250522',
    architecture: 'amd64',
    users: [],
    ...overrides,
  }
}

describe('boxRef', () => {
  it('drops the n/a architecture and refuses incomplete boxes', () => {
    expect(boxRef(box({ architecture: 'n/a' }))).toEqual({
      name: 'almalinux/9',
      provider: 'vmware_desktop',
      version: '9.6.20250522',
      architecture: null,
    })
    expect(boxRef(box({ version: null }))).toBeNull()
  })
})

describe('updateStateFor', () => {
  const results: OutdatedBox[] = [
    {
      name: 'almalinux/9',
      provider: 'vmware_desktop',
      state: 'outdated',
      current: '9.6.20250522',
      latest: '9.8.20260907',
    },
    { name: 'ubuntu/jammy64', provider: 'virtualbox', state: 'upToDate', version: '2' },
    { name: 'my/box', provider: 'virtualbox', state: 'noMetadata' },
  ]

  it('matches the installed version', () => {
    expect(updateStateFor(box(), results)?.state).toBe('outdated')
    expect(updateStateFor(box({ version: '9.8.20260907' }), results)).toBeNull()
    expect(
      updateStateFor(
        box({ name: 'ubuntu/jammy64', provider: 'virtualbox', version: '1' }),
        results,
      ),
    ).toBeNull()
    expect(
      updateStateFor(box({ name: 'my/box', provider: 'virtualbox', version: '0' }), results)?.state,
    ).toBe('noMetadata')
    expect(updateStateFor(box(), null)).toBeNull()
  })
})

describe('activeUsers and sortBoxes', () => {
  it('ignores machines whose folder is gone', () => {
    const user = {
      machine: 'default',
      provider: 'vmware_desktop',
      state: 'running',
      folder: 'D:/p',
      folderExists: true,
      projectId: null,
      projectName: null,
      environmentId: null,
      environmentName: null,
    }
    const b = box({ users: [user, { ...user, folder: '/mnt/d/old', folderExists: false }] })
    expect(activeUsers(b).map((u) => u.folder)).toEqual(['D:/p'])
  })

  it('orders by name, provider and newest version first', () => {
    const sorted = sortBoxes([
      box({ version: '9.10.1' }),
      box({ name: 'Alpine/3' }),
      box({ version: '9.9.0' }),
    ])
    expect(sorted.map((b) => `${b.name} ${b.version}`)).toEqual([
      'almalinux/9 9.10.1',
      'almalinux/9 9.9.0',
      'Alpine/3 9.6.20250522',
    ])
  })
})
