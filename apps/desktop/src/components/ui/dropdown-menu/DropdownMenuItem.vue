<script setup lang="ts">
import { DropdownMenuItem } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{ destructive?: boolean; disabled?: boolean }>(), {
  destructive: false,
  disabled: false,
})
const emit = defineEmits<{ select: [] }>()
</script>

<template>
  <DropdownMenuItem
    :disabled="props.disabled"
    :class="
      cn(
        'flex cursor-default items-center gap-2 rounded px-2 py-1.5 text-sm outline-none select-none data-[disabled]:opacity-50 data-[highlighted]:bg-surface-muted [&_svg]:size-4 [&_svg]:shrink-0',
        props.destructive && 'text-status-failed',
      )
    "
    @select="emit('select')"
  >
    <slot />
  </DropdownMenuItem>
</template>
