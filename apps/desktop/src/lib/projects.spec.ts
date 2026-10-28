import { describe, expect, it } from 'vitest'
import type { Project } from '@/types/projects'
import { queryProjects, recentProjects, relativeTime } from './projects'

function project(overrides: Partial<Project>): Project {
  return {
    id: overrides.name ?? 'id',
    name: 'Project',
    slug: 'project',
    description: '',
    rootPath: '/p',
    repositoryUrl: null,
    status: 'active',
    favorite: false,
    tags: [],
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    archivedAt: null,
    lastOpenedAt: null,
    environmentCount: 0,
    ...overrides,
  }
}

const base = { search: '', favoritesOnly: false, tags: [], sort: 'name' as const }

describe('queryProjects', () => {
  const web = project({ name: 'Web', tags: ['web', 'nginx'], rootPath: 'C:\\code\\web' })
  const db = project({ name: 'database', description: 'Postgres box', tags: ['db'] })
  const api = project({ name: 'API', favorite: true, tags: ['web'] })

  it('puts favorites first, then sorts by name case-insensitively', () => {
    expect(queryProjects([web, db, api], base).map((p) => p.name)).toEqual([
      'API',
      'database',
      'Web',
    ])
  })

  it('searches name, description, path and tags', () => {
    const names = (search: string) =>
      queryProjects([web, db, api], { ...base, search }).map((p) => p.name)
    expect(names('postgres')).toEqual(['database'])
    expect(names('code\\web')).toEqual(['Web'])
    expect(names('NGINX')).toEqual(['Web'])
  })

  it('requires every selected tag and supports favorites only', () => {
    expect(queryProjects([web, db, api], { ...base, tags: ['web', 'nginx'] })).toEqual([web])
    expect(queryProjects([web, db, api], { ...base, favoritesOnly: true })).toEqual([api])
  })

  it('compares timestamps by value, not by string', () => {
    // ".47Z" sorts after ".471Z" as a string but is earlier in time.
    const a = project({ name: 'a', createdAt: '2026-01-01T00:00:00.47Z' })
    const b = project({ name: 'b', createdAt: '2026-01-01T00:00:00.471Z' })
    expect(queryProjects([a, b], { ...base, sort: 'created' }).map((p) => p.name)).toEqual([
      'b',
      'a',
    ])
  })

  it('does not mutate its input', () => {
    const input = [web, db]
    queryProjects(input, base)
    expect(input).toEqual([web, db])
  })
})

describe('recentProjects', () => {
  it('lists only opened, active projects, newest first', () => {
    const old = project({ name: 'old', lastOpenedAt: '2026-01-01T00:00:00Z' })
    const recent = project({ name: 'new', lastOpenedAt: '2026-02-01T00:00:00Z' })
    const never = project({ name: 'never' })
    const archived = project({
      name: 'gone',
      status: 'archived',
      lastOpenedAt: '2026-03-01T00:00:00Z',
    })
    expect(recentProjects([old, never, recent, archived]).map((p) => p.name)).toEqual([
      'new',
      'old',
    ])
  })
})

describe('relativeTime', () => {
  const now = new Date('2026-10-28T12:00:00Z')
  it('formats past times', () => {
    expect(relativeTime('2026-10-28T11:59:30Z', now)).toBe('just now')
    expect(relativeTime('2026-10-28T11:00:00Z', now)).toMatch(/hour/)
    expect(relativeTime('2026-10-21T12:00:00Z', now)).toMatch(/week/)
  })
})
