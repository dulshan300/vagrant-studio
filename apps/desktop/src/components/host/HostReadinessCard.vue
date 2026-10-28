<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { CircleAlert, CircleCheck, CircleHelp, Info } from '@lucide/vue'
import { Card, CardHeader } from '@/components/ui/card'
import ExternalLink from '@/components/ui/ExternalLink.vue'
import { useAppStore } from '@/stores/app'
import { describeHost } from './host'

const app = useAppStore()
onMounted(() => {
  if (!app.loaded) void app.load()
})

const host = computed(() => (app.info ? describeHost(app.info.host) : null))
</script>

<template>
  <Card>
    <CardHeader title="Host readiness" description="What this computer can run right now." />
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
          <span v-if="app.info.vagrantPath" class="flex items-start gap-2">
            <CircleCheck class="mt-0.5 size-4 shrink-0 text-status-running" aria-hidden="true" />
            <span>
              Found
              <code class="block font-mono text-xs [overflow-wrap:anywhere]">{{
                app.info.vagrantPath
              }}</code>
              <span class="block text-xs text-muted"
                >Version and provider checks run once the CLI integration is in place.</span
              >
            </span>
          </span>
          <span v-else class="flex items-start gap-2">
            <CircleAlert class="mt-0.5 size-4 shrink-0 text-status-failed" aria-hidden="true" />
            <span>
              Not found on PATH or in the usual install locations.
              <ExternalLink
                class="block text-accent underline-offset-2 hover:underline"
                href="https://developer.hashicorp.com/vagrant/install"
                >Install Vagrant for your operating system</ExternalLink
              >
            </span>
          </span>
        </dd>
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
