<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { CircleAlert, CircleCheck, CircleHelp, Info, TriangleAlert } from '@lucide/vue'
import { Card, CardHeader } from '@/components/ui/card'
import ExternalLink from '@/components/ui/ExternalLink.vue'
import { hasBackend } from '@/services/app'
import { useAppStore } from '@/stores/app'
import { useOperationsStore } from '@/stores/operations'
import { describeHost } from './host'

const app = useAppStore()
const ops = useOperationsStore()
onMounted(() => {
  if (!app.loaded) void app.load()
  if (hasBackend() && !ops.readiness && !ops.readinessLoading) void ops.checkHost()
})

const host = computed(() => (app.info ? describeHost(app.info.host) : null))
const vagrant = computed(() => ops.readiness?.vagrant ?? null)
</script>

<template>
  <Card>
    <CardHeader title="Host readiness" description="What this computer can run right now.">
      <template #actions>
        <RouterLink :to="{ name: 'providers' }" class="text-xs text-accent hover:underline"
          >Details</RouterLink
        >
      </template>
    </CardHeader>
    <div class="px-5 py-4">
      <p v-if="!app.loaded" class="text-muted">Checking host…</p>
      <p v-else-if="app.error" class="flex items-start gap-2 text-status-failed">
        <CircleAlert class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
        Could not query the backend: {{ app.error }}
      </p>
      <p v-else-if="!app.info" class="flex items-start gap-2 text-muted">
        <CircleHelp class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
        Running outside the desktop shell, so host checks are unavailable.
      </p>

      <dl v-else class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-3">
        <dt class="text-muted">Operating system</dt>
        <dd>{{ host }}</dd>

        <dt class="text-muted">Vagrant</dt>
        <dd>
          <span v-if="!vagrant" class="text-muted">{{ ops.readinessError ?? 'Checking…' }}</span>
          <span v-else-if="vagrant.version" class="flex items-start gap-2">
            <CircleCheck class="mt-0.5 size-4 shrink-0 text-status-running" aria-hidden="true" />
            <span>
              {{ vagrant.version }}
              <code class="block font-mono text-xs [overflow-wrap:anywhere] text-muted">{{
                vagrant.path
              }}</code>
            </span>
          </span>
          <span v-else class="flex items-start gap-2">
            <CircleAlert class="mt-0.5 size-4 shrink-0 text-status-failed" aria-hidden="true" />
            <span>
              {{ vagrant.findings[0]?.message }}
              <ExternalLink
                class="block text-accent underline-offset-2 hover:underline"
                href="https://developer.hashicorp.com/vagrant/install"
                >Install Vagrant</ExternalLink
              >
            </span>
          </span>
        </dd>

        <template v-if="ops.readiness">
          <dt class="text-muted">Providers</dt>
          <dd class="space-y-1">
            <span v-for="p in ops.readiness.providers" :key="p.id" class="flex items-center gap-2">
              <CircleCheck
                v-if="p.state === 'ready'"
                class="size-4 text-status-running"
                aria-label="Ready"
              />
              <TriangleAlert
                v-else-if="p.state === 'attention'"
                class="size-4 text-status-transition"
                aria-label="Needs attention"
              />
              <CircleAlert v-else class="size-4 text-status-stopped" aria-label="Not available" />
              {{ p.name }}
              <span class="text-xs text-muted">
                {{
                  p.state === 'ready'
                    ? (p.version ?? 'ready')
                    : p.state === 'attention'
                      ? 'needs attention'
                      : 'not installed'
                }}
              </span>
            </span>
          </dd>
        </template>
      </dl>

      <p
        v-if="app.info?.host.isWsl"
        class="mt-4 flex items-start gap-2 rounded-md bg-surface-muted px-3 py-2 text-xs text-muted"
      >
        <Info class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
        Running inside WSL. This is a development setup: the Vagrant installation, providers and
        machines here are separate from the Windows host's.
      </p>
    </div>
  </Card>
</template>
