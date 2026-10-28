<script setup lang="ts">
import { CircleAlert, CircleCheck, TriangleAlert } from '@lucide/vue'
import type { Finding } from '@/types/operations'

defineProps<{ findings: Finding[] }>()
</script>

<template>
  <ul class="space-y-2">
    <li v-for="(f, i) in findings" :key="i" class="flex items-start gap-2 text-sm">
      <CircleCheck
        v-if="f.level === 'ok'"
        class="mt-0.5 size-4 shrink-0 text-status-running"
        aria-label="OK"
      />
      <TriangleAlert
        v-else-if="f.level === 'warning'"
        class="mt-0.5 size-4 shrink-0 text-status-transition"
        aria-label="Warning"
      />
      <CircleAlert v-else class="mt-0.5 size-4 shrink-0 text-status-failed" aria-label="Problem" />
      <span class="min-w-0">
        <span class="break-words">{{ f.message }}</span>
        <span v-if="f.fix" class="mt-0.5 block text-xs break-words text-muted">{{ f.fix }}</span>
      </span>
    </li>
  </ul>
</template>
