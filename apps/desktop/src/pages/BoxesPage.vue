<script setup lang="ts">
import { Package } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import { Card } from '@/components/ui/card'
</script>

<template>
  <PageHeader title="Boxes" description="Base images installed in your local Vagrant box store." />
  <Card>
    <EmptyState
      :icon="Package"
      title="Box management isn't available yet"
      description="You'll be able to list, add, update and remove boxes here. Nothing is removed without your confirmation."
    />
  </Card>
</template>
