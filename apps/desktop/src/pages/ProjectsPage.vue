<script setup lang="ts">
import { FolderKanban } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import { Card } from '@/components/ui/card'
</script>

<template>
  <PageHeader title="Projects" description="Organize Vagrant environments into projects." />
  <Card>
    <EmptyState
      :icon="FolderKanban"
      title="No projects yet"
      description="Create a project from a template or import an existing folder with a Vagrantfile. Importing never changes your files or starts machines."
    />
  </Card>
</template>
