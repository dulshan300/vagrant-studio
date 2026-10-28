<script setup lang="ts">
import { Boxes } from '@lucide/vue'
import { NAV_ITEMS } from '@/app/navigation'

const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform)
const modifier = isMac ? '⌘' : 'Ctrl+'
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
            exact-active-class="bg-surface-muted !text-foreground font-medium"
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
  </aside>
</template>
