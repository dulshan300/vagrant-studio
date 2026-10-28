<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{ class?: string }>(), { class: '' })
</script>

<template>
  <section :class="cn('rounded-lg border bg-surface shadow-xs', props.class)">
    <slot />
  </section>
</template>
