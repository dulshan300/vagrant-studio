import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getAppInfo } from '@/services/app'
import type { AppInfo } from '@/types/app'

export const useAppStore = defineStore('app', () => {
  const info = ref<AppInfo | null>(null)
  const loaded = ref(false)
  const error = ref<string | null>(null)

  async function load() {
    try {
      info.value = await getAppInfo()
      error.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  return { info, loaded, error, load }
})
