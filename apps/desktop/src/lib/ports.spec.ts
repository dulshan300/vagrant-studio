import { describe, expect, it } from 'vitest'
import { addressesOverlap, clashesFor, othersUsing, portOwnerLabel } from './ports'
import type { PortUse } from '@/types/vagrantfile'

function use(env: string, machine: string, host: number, hostIp: string | null = null): PortUse {
  return {
    projectId: env === 'e1' || env === 'e3' ? 'p1' : 'p2',
    projectName: env === 'e2' ? 'Other' : 'Mine',
    environmentId: env,
    environmentName: env === 'e3' ? 'staging' : 'default',
    machine,
    hostIp,
    host,
    guest: 80,
    protocol: 'tcp',
  }
}

describe('ports', () => {
  it('treats unset and wildcard addresses as every address', () => {
    expect(addressesOverlap(null, '127.0.0.1')).toBe(true)
    expect(addressesOverlap('0.0.0.0', '192.168.1.5')).toBe(true)
    expect(addressesOverlap('localhost', '127.0.0.1')).toBe(true)
    expect(addressesOverlap('127.0.0.1', '127.0.0.2')).toBe(false)
  })

  it('finds other environments forwarding the same port', () => {
    const others = [use('e2', 'web', 8080), use('e2', 'db', 5432, '127.0.0.1')]
    expect(othersUsing({ host: 8080, hostIp: '127.0.0.1', protocol: null }, others)).toHaveLength(1)
    expect(othersUsing({ host: 8080, hostIp: null, protocol: 'udp' }, others)).toHaveLength(0)
    expect(othersUsing({ host: 5432, hostIp: '127.0.0.2', protocol: 'tcp' }, others)).toHaveLength(
      0,
    )
  })

  it('groups clashes per host port for one environment', () => {
    const ports = [
      use('e1', 'web', 8080, '127.0.0.1'),
      use('e1', 'api', 8080, '127.0.0.1'),
      use('e1', 'db', 5432),
      use('e2', 'web', 8080),
      use('e3', 'web', 8080, '127.0.0.1'),
    ]
    const clashes = clashesFor('e1', ports)
    expect(clashes).toHaveLength(1)
    expect(clashes[0]!.machines).toEqual(['web', 'api'])
    expect(clashes[0]!.others.map((o) => o.environmentId)).toEqual(['e2', 'e3'])
    expect(portOwnerLabel(clashes[0]!.others[0]!, 'p1')).toBe('Other · default (web)')
    expect(portOwnerLabel(clashes[0]!.others[1]!, 'p1')).toBe('staging (web)')
  })
})
