<script setup lang="ts">
import { Badge } from '@/components/ui/badge'
import type { OutdatedBox } from '@/types/boxes'

defineProps<{ state: OutdatedBox | null | undefined }>()
</script>

<template>
  <Badge v-if="state?.state === 'outdated'" class="ml-2 text-status-transition">
    {{ state.latest }} available
  </Badge>
  <span v-else-if="state?.state === 'upToDate'" class="ml-2 text-xs text-muted">newest</span>
  <span v-else-if="state?.state === 'noMetadata'" class="ml-2 text-xs text-muted">
    not from a catalog
  </span>
  <span
    v-else-if="state?.state === 'error'"
    class="ml-2 text-xs text-status-failed"
    :title="state.message"
  >
    check failed
  </span>
</template>
