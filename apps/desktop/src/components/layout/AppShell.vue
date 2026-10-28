<script setup lang="ts">
import AppSidebar from './AppSidebar.vue'
import { useNavigationShortcuts } from '@/composables/useNavigationShortcuts'

useNavigationShortcuts()
</script>

<template>
  <div class="flex h-full overflow-hidden">
    <AppSidebar />
    <main class="min-w-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-6xl px-8 py-8">
        <slot />
      </div>
    </main>
  </div>
</template>
