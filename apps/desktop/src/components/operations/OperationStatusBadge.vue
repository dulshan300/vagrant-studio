<script setup lang="ts">
import { computed } from 'vue'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { OperationStatus } from '@/types/operations'

const props = defineProps<{ status: OperationStatus }>()

const STYLES: Record<OperationStatus, { label: string; dot: string; pulse?: boolean }> = {
  queued: { label: 'Queued', dot: 'bg-status-stopped', pulse: true },
  running: { label: 'Running', dot: 'bg-status-provisioning', pulse: true },
  succeeded: { label: 'Succeeded', dot: 'bg-status-running' },
  failed: { label: 'Failed', dot: 'bg-status-failed' },
  cancel_requested: { label: 'Cancelling', dot: 'bg-status-transition', pulse: true },
  cancelled: { label: 'Cancelled', dot: 'bg-status-transition' },
  interrupted: { label: 'Interrupted', dot: 'bg-status-failed' },
}
const style = computed(() => STYLES[props.status])
</script>

<template>
  <Badge :data-status="status">
    <span
      aria-hidden="true"
      :class="cn('size-2 rounded-full', style.dot, style.pulse && 'animate-pulse')"
    />
    {{ style.label }}
  </Badge>
</template>
