<script setup lang="ts">
import { Boxes, SquareTerminal } from '@lucide/vue'
import { NAV_ITEMS } from '@/app/navigation'
import { useTerminalsStore } from '@/stores/terminals'

const terminals = useTerminalsStore()

const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform)
const modifier = isMac ? '⌘' : 'Ctrl+'
// Sections other than the dashboard stay highlighted on nested routes (e.g. /projects/:id).
const activeClass = 'bg-surface-muted !text-foreground font-medium'
</script>

<template>
  <aside class="flex w-60 shrink-0 flex-col border-r bg-sidebar">
    <div class="flex h-14 items-center gap-2.5 px-5">
      <div
        class="flex size-7 items-center justify-center rounded-md bg-accent text-accent-foreground"
      >
        <Boxes class="size-4" aria-hidden="true" />
      </div>
      <span class="font-semibold tracking-tight">Vagrant Studio</span>
    </div>

    <nav aria-label="Main" class="flex-1 px-3 py-2">
      <ul class="space-y-0.5">
        <li v-for="(item, index) in NAV_ITEMS" :key="item.name">
          <RouterLink
            :to="item.path"
            class="group flex items-center gap-3 rounded-md px-2.5 py-2 text-muted transition-colors hover:bg-surface-muted hover:text-foreground"
            :active-class="item.path === '/' ? undefined : activeClass"
            :exact-active-class="activeClass"
            :title="`${item.label} (${modifier}${index + 1})`"
          >
            <component :is="item.icon" class="size-4 shrink-0" aria-hidden="true" />
            <span class="flex-1 truncate">{{ item.label }}</span>
            <kbd
              class="hidden font-mono text-[10px] text-muted opacity-0 transition-opacity group-hover:opacity-100 lg:inline"
              >{{ modifier }}{{ index + 1 }}</kbd
            >
          </RouterLink>
        </li>
      </ul>
    </nav>

    <div v-if="terminals.tabs.length && !terminals.panelOpen" class="border-t px-3 py-2">
      <button
        class="flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-muted hover:bg-surface-muted hover:text-foreground"
        title="Show the terminal panel (Ctrl+`)"
        @click="terminals.panelOpen = true"
      >
        <SquareTerminal class="size-4 shrink-0" aria-hidden="true" />
        <span class="flex-1 truncate text-left">Terminals</span>
        <span class="text-xs tabular-nums"
          >{{ terminals.running }}/{{ terminals.tabs.length }}</span
        >
      </button>
    </div>
  </aside>
</template>
