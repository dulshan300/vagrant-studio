import { describe, expect, it } from 'vitest'
import { buildChoices, pickDefault } from './useProviderChoices'
import type { ProviderCheck } from '@/types/operations'

const check = (id: string, state: ProviderCheck['state'], problem?: string): ProviderCheck => ({
  id,
  name: id,
  state,
  version: null,
  findings: problem ? [{ level: 'error', message: problem, fix: null }] : [],
})

describe('provider choices', () => {
  it('marks uninstalled providers unavailable and explains problems', () => {
    const choices = buildChoices([
      check('vmware_desktop', 'attention', 'The Vagrant VMware Utility is not reachable.'),
      check('virtualbox', 'unavailable', 'VirtualBox is not installed.'),
    ])
    expect(choices.map((c) => [c.id, c.available, c.note])).toEqual([
      ['vmware_desktop', true, 'The Vagrant VMware Utility is not reachable.'],
      ['virtualbox', false, 'VirtualBox is not installed.'],
    ])
  })

  it('treats providers as selectable while checks are still running', () => {
    expect(buildChoices(null).every((c) => c.available && c.state === 'unknown')).toBe(true)
  })

  it('prefers the configured default, then a ready provider', () => {
    const choices = buildChoices([
      check('vmware_desktop', 'attention'),
      check('virtualbox', 'ready'),
    ])
    expect(pickDefault(choices, 'vmware_desktop')).toBe('vmware_desktop')
    expect(pickDefault(choices, null)).toBe('virtualbox')
    const none = buildChoices([
      check('vmware_desktop', 'unavailable'),
      check('virtualbox', 'unavailable'),
    ])
    expect(pickDefault(none, 'virtualbox')).toBeNull()
  })
})
