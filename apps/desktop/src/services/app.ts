import { invoke, isTauri } from '@tauri-apps/api/core'
import type { AppInfo } from '@/types/app'

/** True when running inside the desktop shell rather than a plain browser (e.g. `pnpm dev`). */
export function hasBackend(): boolean {
  return isTauri()
}

export async function getAppInfo(): Promise<AppInfo | null> {
  if (!hasBackend()) return null
  return invoke<AppInfo>('get_app_info')
}
