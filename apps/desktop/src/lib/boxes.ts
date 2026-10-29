import type { BoxInfo, BoxRef, InstalledBox, OutdatedBox } from '@/types/boxes'
import type { Operation } from '@/types/operations'

/** Operation types that change the box store. */
export const BOX_CHANGES: readonly string[] = ['box_add', 'box_update', 'box_remove']

/** The exact box to update or remove; null if `box list` left out a field. */
export function boxRef(box: BoxInfo): BoxRef | null {
  if (!box.provider || !box.version) return null
  const architecture = box.architecture && box.architecture !== 'n/a' ? box.architecture : null
  return { name: box.name, provider: box.provider, version: box.version, architecture }
}

export function boxKey(box: BoxInfo): string {
  return [box.name, box.provider, box.version, box.architecture].join('|')
}

/** What the update check found for this installed version, if anything. */
export function updateStateFor(
  box: BoxInfo,
  results: readonly OutdatedBox[] | null,
): OutdatedBox | null {
  if (!results) return null
  return (
    results.find((r) => {
      if (r.name !== box.name || r.provider !== box.provider) return false
      switch (r.state) {
        case 'upToDate':
          return r.version === box.version
        case 'outdated':
          return r.current === box.version
        default:
          return true
      }
    }) ?? null
  )
}

/**
 * Machines that still exist according to Vagrant: index entries whose folder
 * is gone are ignored by Vagrant too.
 */
export function activeUsers(box: InstalledBox) {
  return box.users.filter((u) => u.folderExists)
}

/** Sorted by name, then provider, then newest version first. */
export function sortBoxes<T extends BoxInfo>(boxes: readonly T[]): T[] {
  const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })
  return [...boxes].sort(
    (a, b) =>
      collator.compare(a.name, b.name) ||
      collator.compare(a.provider ?? '', b.provider ?? '') ||
      collator.compare(b.version ?? '', a.version ?? ''),
  )
}

/** The box an add/update/remove operation is about, from its label (`Box · …`). */
export function boxOperationTarget(op: Pick<Operation, 'targetLabel'>): string {
  return op.targetLabel.replace(/^Box · /, '')
}
