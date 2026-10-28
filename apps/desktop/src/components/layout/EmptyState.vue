<script setup lang="ts">
import type { Component } from 'vue'

defineProps<{ icon: Component; title: string; description: string }>()
</script>

<template>
  <div class="flex flex-col items-center justify-center px-6 py-14 text-center">
    <div
      class="mb-4 flex size-11 items-center justify-center rounded-full bg-surface-muted text-muted"
    >
      <component :is="icon" class="size-5" aria-hidden="true" />
    </div>
    <h3 class="font-semibold">{{ title }}</h3>
    <p class="mt-1 max-w-md text-muted">{{ description }}</p>
    <div v-if="$slots.default" class="mt-5">
      <slot />
    </div>
  </div>
</template>
