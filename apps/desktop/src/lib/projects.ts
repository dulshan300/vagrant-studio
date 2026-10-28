import type { Project } from '@/types/projects'

export type ProjectSort = 'recent' | 'name' | 'created'

export interface ProjectQuery {
  search: string
  favoritesOnly: boolean
  tags: string[]
  sort: ProjectSort
}

/** Timestamps are RFC 3339 with variable fraction digits, so compare parsed values, not strings. */
const time = (iso: string | null): number => (iso ? Date.parse(iso) : 0)

/** Filters and sorts projects for display. Favorites always come first. */
export function queryProjects(projects: readonly Project[], query: ProjectQuery): Project[] {
  const needle = query.search.trim().toLowerCase()
  const filtered = projects.filter((p) => {
    if (query.favoritesOnly && !p.favorite) return false
    if (query.tags.length && !query.tags.every((t) => p.tags.includes(t))) return false
    if (!needle) return true
    return [p.name, p.description, p.rootPath, p.repositoryUrl ?? '', ...p.tags].some((field) =>
      field.toLowerCase().includes(needle),
    )
  })
  return filtered.sort((a, b) => {
    if (a.favorite !== b.favorite) return a.favorite ? -1 : 1
    switch (query.sort) {
      case 'name':
        return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })
      case 'created':
        return time(b.createdAt) - time(a.createdAt)
      case 'recent':
        return time(b.lastOpenedAt ?? b.updatedAt) - time(a.lastOpenedAt ?? a.updatedAt)
    }
  })
}

/** Most recently opened projects (only ones that have been opened). */
export function recentProjects(projects: readonly Project[], limit = 5): Project[] {
  return projects
    .filter((p) => p.lastOpenedAt !== null && p.status === 'active')
    .sort((a, b) => time(b.lastOpenedAt) - time(a.lastOpenedAt))
    .slice(0, limit)
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['week', 7 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
]

export function relativeTime(iso: string, now: Date = new Date()): string {
  const seconds = (new Date(iso).getTime() - now.getTime()) / 1000
  const format = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })
  for (const [unit, size] of UNITS) {
    if (Math.abs(seconds) >= size) return format.format(Math.round(seconds / size), unit)
  }
  return 'just now'
}
