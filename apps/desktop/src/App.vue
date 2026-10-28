<script setup lang="ts">
import { onMounted } from 'vue'
import AppShell from '@/components/layout/AppShell.vue'
import OperationDrawer from '@/components/operations/OperationDrawer.vue'
import { hasBackend } from '@/services/app'
import { useOperationsStore } from '@/stores/operations'

const operations = useOperationsStore()
onMounted(() => {
  if (hasBackend()) void operations.init()
})
</script>

<template>
  <AppShell>
    <RouterView />
  </AppShell>
  <OperationDrawer />
</template>
