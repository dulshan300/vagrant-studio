<script setup lang="ts">
import { onMounted } from 'vue'
import { Plug, RefreshCw } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import FindingList from '@/components/host/FindingList.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardHeader } from '@/components/ui/card'
import { CAPABILITY_LABEL, SUPPORT_LABEL } from '@/lib/providers'
import { cn } from '@/lib/utils'
import { useOperationsStore } from '@/stores/operations'

const store = useOperationsStore()
onMounted(() => {
  if (!store.readiness && !store.readinessLoading) void store.checkHost()
})

const STATE = {
  ready: { label: 'Ready', dot: 'bg-status-running' },
  attention: { label: 'Needs attention', dot: 'bg-status-transition' },
  unavailable: { label: 'Not available', dot: 'bg-status-stopped' },
} as const
</script>

<template>
  <PageHeader
    title="Providers"
    description="What this computer can run. Checks are read-only: Vagrant Studio never installs software or changes system settings."
  >
    <template #actions>
      <Button variant="secondary" :disabled="store.readinessLoading" @click="store.checkHost()">
        <RefreshCw :class="store.readinessLoading && 'animate-spin'" /> Check again
      </Button>
    </template>
  </PageHeader>

  <Card v-if="store.readinessError">
    <EmptyState :icon="Plug" title="Checks couldn't run" :description="store.readinessError" />
  </Card>
  <p v-else-if="!store.readiness" class="text-muted">
    Checking this computer… Vagrant can take a few seconds to respond.
  </p>

  <div v-else class="space-y-5">
    <Card>
      <CardHeader title="Vagrant" :description="store.readiness.vagrant.path ?? undefined" />
      <div class="space-y-4 px-5 py-4">
        <FindingList :findings="store.readiness.vagrant.findings" />
        <div v-if="store.readiness.plugins.length">
          <h3 class="mb-1.5 text-xs font-medium text-muted">Plugins</h3>
          <ul class="flex flex-wrap gap-1.5">
            <li v-for="p in store.readiness.plugins" :key="p.name">
              <Badge>{{ p.name }} {{ p.version }}</Badge>
            </li>
          </ul>
        </div>
      </div>
    </Card>

    <div class="grid gap-5 lg:grid-cols-2">
      <Card v-for="p in store.readiness.providers" :key="p.id">
        <CardHeader :title="p.name" :description="p.version ? `Version ${p.version}` : p.id">
          <template #actions>
            <Badge>
              <span aria-hidden="true" :class="cn('size-2 rounded-full', STATE[p.state].dot)" />
              {{ STATE[p.state].label }}
            </Badge>
          </template>
        </CardHeader>
        <div class="space-y-4 px-5 py-4">
          <p v-if="p.kind === 'container'" class="text-xs text-muted">
            Runs containers, not virtual machines: boxes and VM settings such as CPU and memory
            don't apply.
          </p>
          <FindingList :findings="p.findings" />
          <div v-if="p.capabilities.length">
            <h3 class="mb-1.5 text-xs font-medium text-muted">
              Supports · {{ SUPPORT_LABEL[p.support] }}
            </h3>
            <ul class="flex flex-wrap gap-1.5">
              <li v-for="c in p.capabilities" :key="c">
                <Badge>{{ CAPABILITY_LABEL[c] }}</Badge>
              </li>
            </ul>
          </div>
        </div>
      </Card>
    </div>
    <p class="text-xs text-muted">
      Only providers Vagrant Studio has integration for are listed. “Detected” means Vagrant can use
      the provider here, but Vagrant Studio hasn't yet verified it on this operating system; none
      has passed the full verification plan so far. Docker is mainly used for automated tests.
    </p>
  </div>
</template>
