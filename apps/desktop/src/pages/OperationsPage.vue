<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Activity, RefreshCw } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import OperationStatusBadge from '@/components/operations/OperationStatusBadge.vue'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { duration, isActive, operationLabel } from '@/lib/operations'
import { relativeTime } from '@/lib/projects'
import { cn } from '@/lib/utils'
import { useOperationsStore } from '@/stores/operations'

const store = useOperationsStore()
onMounted(() => void store.load())

const filter = ref<'all' | 'active' | 'problems'>('all')
const visible = computed(() =>
  store.operations.filter((op) => {
    if (filter.value === 'active') return isActive(op)
    if (filter.value === 'problems') return ['failed', 'interrupted'].includes(op.status)
    return true
  }),
)

const now = ref(Date.now())
const timer = setInterval(() => (now.value = Date.now()), 1000)
onBeforeUnmount(() => clearInterval(timer))

const filters = [
  { value: 'all', label: 'All' },
  { value: 'active', label: 'Running' },
  { value: 'problems', label: 'Failed' },
] as const
</script>

<template>
  <PageHeader
    title="Operations"
    description="Every Vagrant command run from Vagrant Studio, with its output. History is kept across restarts."
  >
    <template #actions>
      <Button variant="secondary" @click="store.load()"><RefreshCw /> Reload</Button>
    </template>
  </PageHeader>

  <div
    role="tablist"
    aria-label="Filter operations"
    class="mb-4 inline-flex rounded-lg border bg-surface-muted p-1"
  >
    <button
      v-for="f in filters"
      :key="f.value"
      role="tab"
      :aria-selected="filter === f.value"
      :class="
        cn(
          'rounded-md px-3 py-1 text-sm text-muted hover:text-foreground',
          filter === f.value && 'bg-surface text-foreground shadow-xs',
        )
      "
      @click="filter = f.value"
    >
      {{ f.label }}
      <span v-if="f.value === 'active' && store.active.length" class="text-xs"
        >({{ store.active.length }})</span
      >
    </button>
  </div>

  <Card v-if="store.error">
    <EmptyState :icon="Activity" title="Operations couldn't be loaded" :description="store.error" />
  </Card>
  <Card v-else-if="store.loaded && !visible.length">
    <EmptyState
      :icon="Activity"
      :title="filter === 'all' ? 'No operations yet' : 'Nothing here'"
      :description="
        filter === 'all'
          ? 'Refresh the status of an environment or validate its Vagrantfile from a project page. Each command and its output is recorded here.'
          : 'No operations match this filter.'
      "
    />
  </Card>
  <Card v-else class="overflow-hidden">
    <table class="w-full text-left text-sm">
      <thead class="border-b bg-surface-muted text-xs text-muted">
        <tr>
          <th class="px-4 py-2 font-medium">Operation</th>
          <th class="px-4 py-2 font-medium">Target</th>
          <th class="px-4 py-2 font-medium">Status</th>
          <th class="px-4 py-2 font-medium">Started</th>
          <th class="px-4 py-2 text-right font-medium">Duration</th>
        </tr>
      </thead>
      <tbody class="divide-y">
        <tr
          v-for="op in visible"
          :key="op.id"
          class="cursor-pointer hover:bg-surface-muted"
          tabindex="0"
          @click="store.open(op.id)"
          @keydown.enter="store.open(op.id)"
        >
          <td class="px-4 py-2.5">
            <span class="font-medium">{{ operationLabel(op.operationType) }}</span>
            <span
              v-if="op.errorSummary && op.status !== 'succeeded'"
              class="block max-w-sm truncate text-xs text-muted"
            >
              {{ op.errorSummary }}
            </span>
          </td>
          <td class="px-4 py-2.5 text-muted">{{ op.targetLabel || 'Host' }}</td>
          <td class="px-4 py-2.5"><OperationStatusBadge :status="op.status" /></td>
          <td class="px-4 py-2.5 text-muted">{{ relativeTime(op.startedAt ?? op.createdAt) }}</td>
          <td class="px-4 py-2.5 text-right text-muted tabular-nums">{{ duration(op, now) }}</td>
        </tr>
      </tbody>
    </table>
  </Card>
</template>
