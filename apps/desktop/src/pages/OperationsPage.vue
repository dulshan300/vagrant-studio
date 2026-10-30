<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Activity, FolderOpen, RefreshCw, X } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import OperationStatusBadge from '@/components/operations/OperationStatusBadge.vue'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { duration, isActive, operationLabel } from '@/lib/operations'
import { relativeTime } from '@/lib/projects'
import { cn } from '@/lib/utils'
import { errorMessage } from '@/services/errors'
import { operationsApi, type OperationFilter } from '@/services/operations'
import { projectsApi } from '@/services/projects'
import { useOperationsStore } from '@/stores/operations'
import { useProjectsStore } from '@/stores/projects'
import type { Operation } from '@/types/operations'
import type { Environment } from '@/types/projects'

const store = useOperationsStore()
const projects = useProjectsStore()
const route = useRoute()
const router = useRouter()

/** Scope from the URL (`?project=…&environment=…&machine=…`), so other pages can link here. */
function queryValue(key: string): string | null {
  const v = route.query[key]
  return typeof v === 'string' && v ? v : null
}
const scope = computed<OperationFilter>(() => ({
  projectId: queryValue('project'),
  environmentId: queryValue('project') ? queryValue('environment') : null,
  machine: queryValue('project') && queryValue('environment') ? queryValue('machine') : null,
}))
const scoped = computed(() => !!scope.value.projectId)

function setScope(next: OperationFilter) {
  const query: Record<string, string> = {}
  if (next.projectId) query.project = next.projectId
  if (next.projectId && next.environmentId) query.environment = next.environmentId
  if (next.projectId && next.environmentId && next.machine) query.machine = next.machine
  void router.replace({ query })
}

const environments = ref<Environment[]>([])
watch(
  () => scope.value.projectId,
  async (id) => {
    environments.value = []
    if (!id) return
    try {
      environments.value = (await projectsApi.get(id)).environments
    } catch {
      // The project may have been removed; its history is still listed.
    }
  },
  { immediate: true },
)
watch(
  () => scope.value.environmentId,
  (id) => {
    if (id && !store.statuses[id]) void store.loadStatus(id).catch(() => undefined)
  },
  { immediate: true },
)
const machineNames = computed(() => {
  const id = scope.value.environmentId
  const names = (id ? store.statuses[id]?.machines : undefined)?.map((m) => m.name) ?? []
  const current = scope.value.machine
  return current && !names.includes(current) ? [...names, current] : names
})

/** History in scope, fetched from the backend (the store only keeps the latest 200 overall). */
const history = ref<Operation[]>([])
const historyLoaded = ref(false)
const historyError = ref<string | null>(null)
async function loadHistory() {
  if (!scoped.value) return
  try {
    history.value = await operationsApi.list(scope.value, 200)
    historyError.value = null
  } catch (e) {
    historyError.value = errorMessage(e)
  } finally {
    historyLoaded.value = true
  }
}
watch(scope, () => void loadHistory(), { deep: true })
// A new operation started anywhere may belong to the scope.
watch(
  () => store.operations.length,
  () => void loadHistory(),
)

onMounted(() => {
  void store.load()
  if (!projects.loaded) void projects.load()
  void loadHistory()
})

function reload() {
  void store.load()
  void loadHistory()
}

const loaded = computed(() => (scoped.value ? historyLoaded.value : store.loaded))
const loadError = computed(() => (scoped.value ? historyError.value : store.error))
/** Operations in scope, with live updates from the store. */
const inScope = computed(() => {
  if (!scoped.value) return store.operations
  const live = new Map(store.operations.map((o) => [o.id, o]))
  return history.value.map((o) => live.get(o.id) ?? o)
})

const filter = ref<'all' | 'active' | 'problems'>('all')
const visible = computed(() =>
  inScope.value.filter((op) => {
    if (filter.value === 'active') return isActive(op)
    if (filter.value === 'problems') return ['failed', 'interrupted'].includes(op.status)
    return true
  }),
)
const activeCount = computed(() => inScope.value.filter(isActive).length)

const now = ref(Date.now())
const timer = setInterval(() => (now.value = Date.now()), 1000)
onBeforeUnmount(() => clearInterval(timer))

const filters = [
  { value: 'all', label: 'All' },
  { value: 'active', label: 'Running' },
  { value: 'problems', label: 'Failed' },
] as const

const folderError = ref<string | null>(null)
async function openLogsFolder() {
  folderError.value = null
  try {
    await operationsApi.openLogsFolder()
  } catch (e) {
    folderError.value = errorMessage(e)
  }
}

const selectClass = 'rounded-md border bg-surface px-2 py-1 text-sm'
</script>

<template>
  <PageHeader
    title="Operations"
    description="Every Vagrant command run from Vagrant Studio, with its output. History is kept across restarts."
  >
    <template #actions>
      <Button
        variant="secondary"
        title="Open the folder with every operation's log file"
        @click="openLogsFolder"
        ><FolderOpen /> Logs folder</Button
      >
      <Button variant="secondary" @click="reload"><RefreshCw /> Reload</Button>
    </template>
  </PageHeader>
  <p v-if="folderError" class="mb-3 text-sm text-status-failed" role="alert">{{ folderError }}</p>

  <div class="mb-4 flex flex-wrap items-center gap-3">
    <div
      role="tablist"
      aria-label="Filter operations"
      class="inline-flex rounded-lg border bg-surface-muted p-1"
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
        <span v-if="f.value === 'active' && activeCount" class="text-xs">({{ activeCount }})</span>
      </button>
    </div>

    <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Show operations for">
      <select
        :class="selectClass"
        aria-label="Project"
        :value="scope.projectId ?? ''"
        @change="setScope({ projectId: ($event.target as HTMLSelectElement).value || null })"
      >
        <option value="">All projects and host commands</option>
        <option v-for="p in projects.projects" :key="p.id" :value="p.id">{{ p.name }}</option>
        <option
          v-if="scope.projectId && !projects.projects.some((p) => p.id === scope.projectId)"
          :value="scope.projectId"
        >
          Removed project
        </option>
      </select>
      <select
        v-if="scope.projectId"
        :class="selectClass"
        aria-label="Environment"
        :value="scope.environmentId ?? ''"
        @change="
          setScope({
            projectId: scope.projectId,
            environmentId: ($event.target as HTMLSelectElement).value || null,
          })
        "
      >
        <option value="">All environments</option>
        <option v-for="e in environments" :key="e.id" :value="e.id">{{ e.name }}</option>
      </select>
      <select
        v-if="scope.environmentId"
        :class="selectClass"
        aria-label="Machine"
        :value="scope.machine ?? ''"
        @change="
          setScope({ ...scope, machine: ($event.target as HTMLSelectElement).value || null })
        "
      >
        <option value="">All machines</option>
        <option v-for="m in machineNames" :key="m" :value="m">{{ m }}</option>
      </select>
      <Button
        v-if="scoped"
        variant="ghost"
        size="sm"
        aria-label="Show all operations"
        @click="setScope({})"
      >
        <X /> Clear
      </Button>
    </div>
  </div>
  <p v-if="scope.machine" class="-mt-2 mb-3 text-xs text-muted">
    Commands that named {{ scope.machine }}. Status checks and validation cover the whole
    environment and are listed under All machines.
  </p>

  <Card v-if="loadError">
    <EmptyState :icon="Activity" title="Operations couldn't be loaded" :description="loadError" />
  </Card>
  <Card v-else-if="loaded && !visible.length">
    <EmptyState
      :icon="Activity"
      :title="filter === 'all' && !scoped ? 'No operations yet' : 'Nothing here'"
      :description="
        filter === 'all' && !scoped
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
