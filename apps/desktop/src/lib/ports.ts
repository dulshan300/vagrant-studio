import type { PortUse, Protocol } from '@/types/vagrantfile'

/** Addresses that mean "every address" when Vagrant binds a forwarded port. */
const ANY = new Set(['', '0.0.0.0', '*', '::'])

function address(ip: string | null): string {
  const a = (ip ?? '').trim().toLowerCase()
  return a === 'localhost' ? '127.0.0.1' : a
}

/** Whether two host addresses can't both be bound for the same port. */
export function addressesOverlap(a: string | null, b: string | null): boolean {
  const x = address(a)
  const y = address(b)
  return x === y || ANY.has(x) || ANY.has(y)
}

export interface PortRef {
  host: number
  hostIp: string | null
  protocol: Protocol | null
}

/** Ports of other environments that would collide with `port`. */
export function othersUsing(port: PortRef, others: readonly PortUse[]): PortUse[] {
  const protocol = port.protocol ?? 'tcp'
  return others.filter(
    (o) =>
      o.host === port.host && o.protocol === protocol && addressesOverlap(o.hostIp, port.hostIp),
  )
}

export interface PortClash {
  host: number
  protocol: Protocol
  /** This environment's machines forwarding the port. */
  machines: string[]
  /** Other environments' forwards of the same port. */
  others: PortUse[]
}

/** Host ports this environment shares with other environments. */
export function clashesFor(environmentId: string, ports: readonly PortUse[]): PortClash[] {
  const mine = ports.filter((p) => p.environmentId === environmentId)
  const others = ports.filter((p) => p.environmentId !== environmentId)
  const clashes = new Map<string, PortClash>()
  for (const p of mine) {
    const hits = othersUsing(p, others)
    if (!hits.length) continue
    const key = `${p.protocol}:${p.host}`
    const clash = clashes.get(key) ?? {
      host: p.host,
      protocol: p.protocol,
      machines: [],
      others: [],
    }
    if (!clash.machines.includes(p.machine)) clash.machines.push(p.machine)
    for (const h of hits) if (!clash.others.includes(h)) clash.others.push(h)
    clashes.set(key, clash)
  }
  return [...clashes.values()].sort((a, b) => a.host - b.host)
}

/** "Project · environment (machine)", naming the project only when it differs. */
export function portOwnerLabel(use: PortUse, currentProjectId?: string): string {
  const where =
    use.projectId === currentProjectId
      ? use.environmentName
      : `${use.projectName} · ${use.environmentName}`
  return `${where} (${use.machine})`
}
