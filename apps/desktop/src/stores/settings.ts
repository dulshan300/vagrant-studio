import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  applyTheme,
  loadThemePreference,
  resolveTheme,
  saveThemePreference,
  type ThemePreference,
} from '@/app/theme'

function safeLocalStorage(): Storage | undefined {
  try {
    return window.localStorage
  } catch {
    return undefined
  }
}

export const useSettingsStore = defineStore('settings', () => {
  const darkQuery = window.matchMedia?.('(prefers-color-scheme: dark)')
  const systemPrefersDark = ref(darkQuery?.matches ?? false)
  const themePreference = ref<ThemePreference>(loadThemePreference(safeLocalStorage()))
  const resolvedTheme = computed(() => resolveTheme(themePreference.value, systemPrefersDark.value))

  function sync() {
    applyTheme(document.documentElement, resolvedTheme.value)
  }

  function setThemePreference(preference: ThemePreference) {
    themePreference.value = preference
    saveThemePreference(safeLocalStorage(), preference)
    sync()
  }

  darkQuery?.addEventListener('change', (event) => {
    systemPrefersDark.value = event.matches
    sync()
  })

  sync()

  return { themePreference, resolvedTheme, setThemePreference }
})
