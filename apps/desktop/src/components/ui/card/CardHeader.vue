<script setup lang="ts">
defineProps<{ title: string; description?: string }>()
</script>

<template>
  <header class="flex items-start justify-between gap-4 border-b px-5 py-4">
    <div class="min-w-0">
      <h2 class="text-sm font-semibold">{{ title }}</h2>
      <p v-if="description" class="mt-0.5 text-xs text-muted">{{ description }}</p>
    </div>
    <slot name="actions" />
  </header>
</template>
