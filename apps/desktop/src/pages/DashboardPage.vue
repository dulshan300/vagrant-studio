<script setup lang="ts">
import { FolderKanban, History } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import HostReadinessCard from '@/components/host/HostReadinessCard.vue'
import StatusBadge from '@/components/status/StatusBadge.vue'
import { MACHINE_STATES } from '@/components/status/machine-state'
import { Card, CardHeader } from '@/components/ui/card'
</script>

<template>
  <PageHeader
    title="Dashboard"
    description="Your Vagrant projects, machines and recent activity."
  />

  <div class="grid gap-5 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
    <Card>
      <CardHeader title="Recent projects" />
      <EmptyState
        :icon="FolderKanban"
        title="No projects yet"
        description="Projects you create or import will be listed here."
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
