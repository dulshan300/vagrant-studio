import { describe, expect, it } from 'vitest'
import { provisionerChoice, provisionerLabel, provisioningOutcome } from './provisioning'
import type { MachineProvisioners, ShellProvisioner } from '@/types/vagrantfile'
import type { MachineProvisioning, ProvisionerRun } from '@/types/operations'

function prov(name: string | null, shared = false): ShellProvisioner & { shared: boolean } {
  return { name, source: { kind: 'inline', script: 'echo\n' }, run: null, privileged: null, shared }
}

const machines: MachineProvisioners[] = [
  { machine: 'web', provisioners: [prov(null, true), prov('hello', true), prov('setup')] },
  { machine: 'db', provisioners: [prov(null, true), prov('hello', true), prov('always')] },
]

function run(name: string | null, status: ProvisionerRun['status']): ProvisionerRun {
  return { machine: 'web', name, kind: 'shell', status, output: [], truncated: false }
}

function result(runs: ProvisionerRun[]): MachineProvisioning {
  return { operationId: 'op', machine: 'web', finishedAt: '2026-10-29T10:00:00Z', runs }
}

describe('provisionerChoice', () => {
  it('offers named provisioners of a single machine', () => {
    const c = provisionerChoice(machines, ['web'])
    expect(c.named.map((p) => p.name)).toEqual(['hello', 'setup'])
    expect(c.others).toBe(1)
  })

  it('offers only names every target has', () => {
    const c = provisionerChoice(machines, ['web', 'db'])
    expect(c.named).toEqual([{ name: 'hello', run: null, shared: true }])
    // Unnamed on each machine (2), setup and always.
    expect(c.others).toBe(4)
  })

  it('offers nothing for machines missing from the Vagrantfile', () => {
    expect(provisionerChoice(machines, ['web', 'gone']).named).toEqual([])
  })
})

describe('provisioningOutcome', () => {
  it('names the provisioner that failed or was interrupted', () => {
    expect(provisioningOutcome(result([run(null, 'succeeded'), run('fail', 'failed')]))).toEqual({
      status: 'failed',
      text: '“fail” failed',
    })
    expect(provisioningOutcome(result([run(null, 'unfinished')])).text).toBe(
      'Unnamed shell provisioner was interrupted',
    )
  })

  it('counts provisioners that ran', () => {
    expect(provisioningOutcome(result([run('a', 'succeeded')])).text).toBe('1 provisioner ran')
    expect(provisioningOutcome(result([run('a', 'succeeded'), run(null, 'succeeded')]))).toEqual({
      status: 'succeeded',
      text: '2 provisioners ran',
    })
  })

  it('labels runs', () => {
    expect(provisionerLabel({ name: 'seed data', kind: 'shell' })).toBe('“seed data”')
  })
})
