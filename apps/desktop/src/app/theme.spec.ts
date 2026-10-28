import { describe, expect, it } from 'vitest'
import {
  THEME_STORAGE_KEY,
  applyTheme,
  isThemePreference,
  loadThemePreference,
  resolveTheme,
  saveThemePreference,
} from './theme'

describe('theme', () => {
  it('resolves system preference from the OS setting', () => {
    expect(resolveTheme('system', true)).toBe('dark')
    expect(resolveTheme('system', false)).toBe('light')
    expect(resolveTheme('light', true)).toBe('light')
    expect(resolveTheme('dark', false)).toBe('dark')
  })

  it('validates stored values', () => {
    expect(isThemePreference('dark')).toBe(true)
    expect(isThemePreference('purple')).toBe(false)
    expect(isThemePreference(null)).toBe(false)
  })

  it('falls back to system for missing, invalid or blocked storage', () => {
    expect(loadThemePreference(undefined)).toBe('system')
    expect(loadThemePreference({ getItem: () => 'purple' })).toBe('system')
    expect(
      loadThemePreference({
        getItem: () => {
          throw new Error('SecurityError')
        },
      }),
    ).toBe('system')
  })

  it('round-trips through storage', () => {
    const data = new Map<string, string>()
    const storage = {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: (k: string, v: string) => void data.set(k, v),
    }
    saveThemePreference(storage, 'dark')
    expect(data.get(THEME_STORAGE_KEY)).toBe('dark')
    expect(loadThemePreference(storage)).toBe('dark')
  })

  it('does not throw when storage writes fail', () => {
    expect(() =>
      saveThemePreference(
        {
          setItem: () => {
            throw new Error('QuotaExceededError')
          },
        },
        'light',
      ),
    ).not.toThrow()
  })

  it('toggles the dark class on the root element', () => {
    const root = document.createElement('html')
    applyTheme(root, 'dark')
    expect(root.classList.contains('dark')).toBe(true)
    expect(root.dataset.theme).toBe('dark')
    applyTheme(root, 'light')
    expect(root.classList.contains('dark')).toBe(false)
  })
})
