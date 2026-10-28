<script setup lang="ts">
import { cn } from '@/lib/utils'

const model = defineModel<string>({ required: true })
const props = withDefaults(
  defineProps<{ invalid?: boolean; multiline?: boolean; class?: string }>(),
  {
    invalid: false,
    multiline: false,
    class: '',
  },
)
const classes = () =>
  cn(
    'w-full rounded-md border bg-surface px-3 py-2 text-sm placeholder:text-muted/70 focus-visible:outline-2 focus-visible:outline-offset-0',
    props.invalid && 'border-status-failed',
    props.class,
  )
</script>

<template>
  <textarea v-if="multiline" v-model="model" rows="3" :class="classes()" :aria-invalid="invalid" />
  <input v-else v-model="model" type="text" :class="classes()" :aria-invalid="invalid" />
</template>
