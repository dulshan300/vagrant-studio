import { describe, expect, it } from 'vitest'
import { NAV_ITEMS } from '@/app/navigation'
import { routes } from '@/app/router'
import { navigationShortcutIndex } from './useNavigationShortcuts'

const key = (k: string, mods: Partial<KeyboardEvent> = {}) => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
})

describe('navigation shortcuts', () => {
  it('maps Ctrl/Cmd + 1…7 to sidebar entries', () => {
    expect(navigationShortcutIndex(key('1', { ctrlKey: true }))).toBe(0)
    expect(navigationShortcutIndex(key('7', { metaKey: true }))).toBe(6)
  })

  it('ignores other keys and modifier combinations', () => {
    expect(navigationShortcutIndex(key('1'))).toBeNull()
    expect(navigationShortcutIndex(key('8', { ctrlKey: true }))).toBeNull()
    expect(navigationShortcutIndex(key('0', { ctrlKey: true }))).toBeNull()
    expect(navigationShortcutIndex(key('a', { ctrlKey: true }))).toBeNull()
    expect(navigationShortcutIndex(key('1', { ctrlKey: true, shiftKey: true }))).toBeNull()
  })

  it('has a route for every sidebar entry', () => {
    const routePaths = new Set(routes.map((r) => r.path))
    for (const item of NAV_ITEMS) expect(routePaths.has(item.path)).toBe(true)
    expect(NAV_ITEMS).toHaveLength(7)
  })
})
