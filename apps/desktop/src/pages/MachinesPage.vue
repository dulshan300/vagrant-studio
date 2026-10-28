<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Server } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import StatusBadge from '@/components/status/StatusBadge.vue'
import { Card } from '@/components/ui/card'
import { machineState } from '@/lib/operations'
import { relativeTime } from '@/lib/projects'
import { projectsApi } from '@/services/projects'
import { useOperationsStore } from '@/stores/operations'
import { useProjectsStore } from '@/stores/projects'
import type { Environment } from '@/types/projects'

const operations = useOperationsStore()
const projects = useProjectsStore()
const environments = ref<
  Record<string, { env: Environment; projectId: string; projectName: string }>
>({})

onMounted(async () => {
  await Promise.all([projects.load(), operations.loadStatuses().catch(() => undefined)])
  for (const p of projects.active) {
    const details = await projectsApi.get(p.id).catch(() => null)
    for (const env of details?.environments ?? [])
      environments.value[env.id] = { env, projectId: p.id, projectName: p.name }
  }
})

const rows = computed(() =>
  Object.values(operations.statuses).flatMap((status) => {
    const owner = environments.value[status.environmentId]
    if (!owner) return []
    return status.machines.map((m) => ({ ...m, owner, observedAt: status.observedAt }))
  }),
)
</script>

<template>
  <PageHeader
    title="Virtual Machines"
    description="Machines across your projects, as last reported by Vagrant. Refresh an environment to update its machines."
  />
  <Card v-if="!rows.length">
    <EmptyState
      :icon="Server"
      title="No machine states yet"
      description="Open a project and use Refresh status on an environment. Machine states always come from Vagrant itself."
    />
  </Card>
  <Card v-else class="overflow-hidden">
    <table class="w-full text-left text-sm">
      <thead class="border-b bg-surface-muted text-xs text-muted">
        <tr>
          <th class="px-4 py-2 font-medium">Machine</th>
          <th class="px-4 py-2 font-medium">Project · environment</th>
          <th class="px-4 py-2 font-medium">Provider</th>
          <th class="px-4 py-2 font-medium">State</th>
          <th class="px-4 py-2 font-medium">Checked</th>
        </tr>
      </thead>
      <tbody class="divide-y">
        <tr v-for="row in rows" :key="row.owner.env.id + row.name">
          <td class="px-4 py-2.5 font-mono text-xs">{{ row.name }}</td>
          <td class="px-4 py-2.5">
            <RouterLink
              :to="{ name: 'project', params: { id: row.owner.projectId } }"
              class="hover:underline"
              >{{ row.owner.projectName }}</RouterLink
            >
            <span class="text-muted"> · {{ row.owner.env.name }}</span>
          </td>
          <td class="px-4 py-2.5 text-muted">{{ row.provider ?? '—' }}</td>
          <td class="px-4 py-2.5" :title="row.stateLong ?? undefined">
            <StatusBadge :state="machineState(row.state)" />
          </td>
          <td class="px-4 py-2.5 text-muted">{{ relativeTime(row.observedAt) }}</td>
        </tr>
      </tbody>
    </table>
  </Card>
</template>
