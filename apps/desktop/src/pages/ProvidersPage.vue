<script setup lang="ts">
import { Plug } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import { Card } from '@/components/ui/card'
</script>

<template>
  <PageHeader
    title="Providers"
    description="Virtualization providers detected on this computer and what they support."
  />
  <Card>
    <EmptyState
      :icon="Plug"
      title="Provider detection isn't available yet"
      description="Vagrant Studio will detect installed providers and their prerequisites, and explain how to fix anything missing. It never installs software or changes host settings for you."
    />
  </Card>
</template>
