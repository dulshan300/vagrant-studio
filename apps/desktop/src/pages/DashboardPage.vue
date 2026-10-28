<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { FolderKanban, History, Star } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import HostReadinessCard from '@/components/host/HostReadinessCard.vue'
import StatusBadge from '@/components/status/StatusBadge.vue'
import { MACHINE_STATES } from '@/components/status/machine-state'
import { Card, CardHeader } from '@/components/ui/card'
import { recentProjects, relativeTime } from '@/lib/projects'
import { useProjectsStore } from '@/stores/projects'

const projects = useProjectsStore()
onMounted(() => void projects.load())
const recent = computed(() => recentProjects(projects.projects))
</script>

<template>
  <PageHeader
    title="Dashboard"
    description="Your Vagrant projects, machines and recent activity."
  />

  <div class="grid gap-5 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
    <Card>
      <CardHeader title="Recent projects">
        <template #actions>
          <RouterLink :to="{ name: 'projects' }" class="text-xs text-accent hover:underline">
            All projects
          </RouterLink>
        </template>
      </CardHeader>
      <dl v-if="projects.loaded && !projects.error" class="grid grid-cols-3 border-b text-center">
        <div class="px-3 py-3">
          <dt class="text-xs text-muted">Projects</dt>
          <dd class="text-lg font-semibold tabular-nums">{{ projects.active.length }}</dd>
        </div>
        <div class="border-x px-3 py-3">
          <dt class="text-xs text-muted">Favorites</dt>
          <dd class="text-lg font-semibold tabular-nums">
            {{ projects.active.filter((p) => p.favorite).length }}
          </dd>
        </div>
        <div class="px-3 py-3">
          <dt class="text-xs text-muted">Archived</dt>
          <dd class="text-lg font-semibold tabular-nums">{{ projects.archived.length }}</dd>
        </div>
      </dl>
      <ul v-if="recent.length" class="divide-y">
        <li v-for="p in recent" :key="p.id">
          <RouterLink
            :to="{ name: 'project', params: { id: p.id } }"
            class="flex items-center justify-between gap-3 px-5 py-3 hover:bg-surface-muted"
          >
            <span class="flex min-w-0 items-center gap-2">
              <Star
                v-if="p.favorite"
                class="size-3.5 shrink-0 fill-status-transition text-status-transition"
                aria-label="Favorite"
              />
              <span class="truncate font-medium">{{ p.name }}</span>
            </span>
            <span class="shrink-0 text-xs text-muted">{{ relativeTime(p.lastOpenedAt!) }}</span>
          </RouterLink>
        </li>
      </ul>
      <EmptyState
        v-else
        :icon="FolderKanban"
        :title="
          projects.error
            ? 'Projects unavailable'
            : projects.active.length
              ? 'Nothing opened yet'
              : 'No projects yet'
        "
        :description="
          projects.error ??
          (projects.active.length
            ? 'Projects you open will be listed here.'
            : 'Create or import a project to get started.')
        "
      />
    </Card>

    <HostReadinessCard />

    <Card>
      <CardHeader title="Recent operations" />
      <EmptyState
        :icon="History"
        title="No operations yet"
        description="Starts, stops, provisioning runs and their results will appear here."
      />
    </Card>

    <Card>
      <CardHeader
        title="Machines"
        description="Counts come from Vagrant's reported state, never estimates."
      />
      <ul class="divide-y">
        <li
          v-for="state in MACHINE_STATES"
          :key="state"
          class="flex items-center justify-between px-5 py-2.5"
        >
          <StatusBadge :state="state" />
          <span class="text-muted tabular-nums" aria-label="No data">—</span>
        </li>
      </ul>
    </Card>
  </div>
</template>
