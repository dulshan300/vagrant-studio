import type { LifecycleAction, MachineProvisioning, ProvisionerRun } from '@/types/operations'
import type { MachineProvisioners, ProvisionRun } from '@/types/vagrantfile'

/** Commands that can run provisioners, in the order the dialog offers them. */
export const PROVISION_COMMANDS = ['provision', 'reload', 'up'] as const
export type ProvisionCommand = (typeof PROVISION_COMMANDS)[number] & LifecycleAction

export const PROVISION_COMMAND_LABELS: Record<ProvisionCommand, string> = {
  provision: 'Provision running machines',
  reload: 'Restart, then provision',
  up: 'Start, then provision',
}

export interface ChoosableProvisioner {
  name: string
  run: ProvisionRun | null
  shared: boolean
}

export interface ProvisionerChoice {
  /** Named provisioners every target has, in the first target's order. */
  named: ChoosableProvisioner[]
  /** Provisioners that can't be chosen alone: unnamed, or not on every target. */
  others: number
}

/**
 * What the dialog can offer for these machines. `--provision-with` is one list
 * for all targets, and Vagrant silently skips names a machine doesn't have,
 * so only names every target defines can be chosen.
 */
export function provisionerChoice(
  machines: MachineProvisioners[],
  targets: readonly string[],
): ProvisionerChoice {
  const lists = targets.map((t) => machines.find((m) => m.machine === t)?.provisioners ?? [])
  const [first = [], ...rest] = lists
  const named = first
    .filter((p) => p.name !== null)
    .filter((p) => rest.every((list) => list.some((q) => q.name === p.name)))
    .map((p) => ({ name: p.name!, run: p.run, shared: p.shared }))
  const total = new Set(
    lists.flatMap((list, i) => list.map((p, j) => p.name ?? `#${targets[i]}-${j}`)),
  ).size
  return { named, others: total - named.length }
}

/** How a machine's last provisioning went, for its row on the Machines page. */
export function provisioningOutcome(m: MachineProvisioning): {
  status: ProvisionerRun['status']
  text: string
} {
  const bad = m.runs.find((r) => r.status !== 'succeeded')
  if (bad) {
    return {
      status: bad.status,
      text: `${provisionerLabel(bad)} ${bad.status === 'failed' ? 'failed' : 'was interrupted'}`,
    }
  }
  return {
    status: 'succeeded',
    text: m.runs.length === 1 ? '1 provisioner ran' : `${m.runs.length} provisioners ran`,
  }
}

export function provisionerLabel(run: Pick<ProvisionerRun, 'name' | 'kind'>): string {
  return run.name ? `“${run.name}”` : `Unnamed ${run.kind} provisioner`
}
