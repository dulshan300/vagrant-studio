<script setup lang="ts">
import { Server } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import { Card } from '@/components/ui/card'
</script>

<template>
  <PageHeader
    title="Virtual Machines"
    description="Every machine across your projects, with the state Vagrant reports."
  />
  <Card>
    <EmptyState
      :icon="Server"
      title="No machines to show"
      description="Machines appear here once a project has an environment. Their state always comes from Vagrant itself."
    />
  </Card>
</template>
