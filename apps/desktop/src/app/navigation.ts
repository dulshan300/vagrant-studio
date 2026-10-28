import type { Component } from 'vue'
import {
  Activity,
  FolderKanban,
  LayoutDashboard,
  Package,
  Plug,
  Server,
  Settings,
} from '@lucide/vue'

export interface NavItem {
  name: string
  label: string
  path: string
  icon: Component
}

/** Global sections, in sidebar order. Ctrl/⌘ + 1…7 jumps to each. */
export const NAV_ITEMS: readonly NavItem[] = [
  { name: 'dashboard', label: 'Dashboard', path: '/', icon: LayoutDashboard },
  { name: 'projects', label: 'Projects', path: '/projects', icon: FolderKanban },
  { name: 'machines', label: 'Virtual Machines', path: '/machines', icon: Server },
  { name: 'boxes', label: 'Boxes', path: '/boxes', icon: Package },
  { name: 'providers', label: 'Providers', path: '/providers', icon: Plug },
  { name: 'operations', label: 'Operations', path: '/operations', icon: Activity },
  { name: 'settings', label: 'Settings', path: '/settings', icon: Settings },
]
