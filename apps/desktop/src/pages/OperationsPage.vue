<script setup lang="ts">
import { Activity } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import { Card } from '@/components/ui/card'
</script>

<template>
  <PageHeader
    title="Operations"
    description="History and live output of every Vagrant command run from Vagrant Studio."
  />
  <Card>
    <EmptyState
      :icon="Activity"
      title="No operations yet"
      description="Each command you run will be recorded here with its output, exit code and duration, and kept across restarts."
    />
  </Card>
</template>
