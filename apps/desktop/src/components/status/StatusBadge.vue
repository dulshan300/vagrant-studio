<script setup lang="ts">
import { computed } from 'vue'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import { MACHINE_STATE_STYLES, type MachineState } from './machine-state'

const props = defineProps<{ state: MachineState }>()
const style = computed(() => MACHINE_STATE_STYLES[props.state])
</script>

<template>
  <Badge :data-state="state">
    <span
      aria-hidden="true"
      :class="cn('size-2 rounded-full', style.dot, style.pulse && 'animate-pulse')"
    />
    {{ style.label }}
  </Badge>
</template>
