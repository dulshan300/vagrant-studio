<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{ class?: string }>(), { class: '' })
</script>

<template>
  <span
    :class="
      cn(
        'inline-flex items-center gap-1.5 rounded-full border bg-surface-muted px-2 py-0.5 text-xs font-medium',
        props.class,
      )
    "
  >
    <slot />
  </span>
</template>
