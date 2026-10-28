<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { CircleAlert, FileCheck, RefreshCw, ScrollText, Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import ProviderSelect from '@/components/providers/ProviderSelect.vue'
import { useProviderChoices } from '@/composables/useProviderChoices'
import { projectsApi } from '@/services/projects'
import StatusBadge from '@/components/status/StatusBadge.vue'
import OperationStatusBadge from '@/components/operations/OperationStatusBadge.vue'
import { isActive, machineState, operationLabel } from '@/lib/operations'
import { relativeTime } from '@/lib/projects'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { Environment } from '@/types/projects'

const props = defineProps<{ environment: Environment }>()
const emit = defineEmits<{ remove: []; updated: [environment: Environment] }>()
const providers = useProviderChoices()

const operations = useOperationsStore()
const actionError = ref<string | null>(null)

onMounted(() => {
  void operations.loadStatus(props.environment.id).catch(() => undefined)
})

const status = computed(() => operations.statuses[props.environment.id] ?? null)
const latest = computed(() => operations.latestFor(props.environment.id))
const busy = computed(() => !!latest.value && isActive(latest.value))

async function changeProvider(provider: string | null) {
  actionError.value = null
  try {
    emit('updated', await projectsApi.setEnvironmentProvider(props.environment.id, provider))
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}

async function run(action: (id: string) => Promise<unknown>) {
  actionError.value = null
  try {
    await action(props.environment.id)
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}
</script>

<template>
  <li class="px-5 py-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="font-medium">{{ environment.name }}</span>
          <span class="rounded bg-surface-muted px-1.5 py-0.5 text-[11px] capitalize">{{
            environment.environmentType
          }}</span>
          <label :for="`env-provider-${environment.id}`" class="sr-only">Provider</label>
          <ProviderSelect
            :id="`env-provider-${environment.id}`"
            :model-value="environment.defaultProvider"
            :choices="providers.choices.value"
            allow-unset
            class="!py-0.5 text-xs"
            :title="'Provider used for new machines; existing machines keep theirs.'"
            @update:model-value="changeProvider"
          />
        </div>
        <code
          class="mt-0.5 block truncate font-mono text-xs text-muted"
          :title="environment.rootPath"
          >{{ environment.rootPath }}</code
        >
      </div>
      <div class="flex items-center gap-1.5">
        <Button
          variant="secondary"
          size="sm"
          :disabled="busy || !environment.vagrantfilePath"
          @click="run(operations.refreshStatus)"
        >
          <RefreshCw :class="busy && latest?.operationType === 'status' ? 'animate-spin' : ''" />
          Refresh status
        </Button>
        <Button
          variant="ghost"
          size="sm"
          :disabled="busy || !environment.vagrantfilePath"
          @click="run(operations.validate)"
        >
          <FileCheck /> Validate
        </Button>
        <Button
          variant="ghost"
          size="icon"
          class="size-8 text-muted"
          :aria-label="`Remove environment ${environment.name}`"
          @click="$emit('remove')"
        >
          <Trash2 />
        </Button>
      </div>
    </div>

    <p v-if="!environment.vagrantfilePath" class="mt-2 text-xs text-muted">
      No Vagrantfile detected in this folder, so Vagrant commands are disabled.
    </p>

    <div v-if="status" class="mt-3 rounded-md border">
      <div class="flex items-center justify-between border-b px-3 py-1.5 text-[11px] text-muted">
        <span>Reported by Vagrant {{ relativeTime(status.observedAt) }}</span>
      </div>
      <p v-if="status.error" class="flex items-start gap-2 px-3 py-2 text-xs text-status-failed">
        <CircleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
        <span class="whitespace-pre-wrap">{{ status.error.message }}</span>
      </p>
      <ul v-else class="divide-y">
        <li
          v-for="m in status.machines"
          :key="m.name"
          class="flex items-center justify-between gap-3 px-3 py-2"
          :title="m.stateLong ?? undefined"
        >
          <span class="font-mono text-xs">{{ m.name }}</span>
          <span class="flex items-center gap-2">
            <span class="text-[11px] text-muted">{{ m.provider }}</span>
            <StatusBadge :state="machineState(m.state)" />
          </span>
        </li>
        <li v-if="!status.machines.length" class="px-3 py-2 text-xs text-muted">
          Vagrant reported no machines.
        </li>
      </ul>
    </div>
    <p v-else-if="environment.vagrantfilePath" class="mt-3 text-xs text-muted">
      Machine state hasn't been checked yet. Use Refresh status to ask Vagrant.
    </p>

    <button
      v-if="latest"
      type="button"
      class="mt-2 flex items-center gap-2 text-xs text-muted hover:text-foreground"
      @click="operations.open(latest.id)"
    >
      <ScrollText class="size-3.5" aria-hidden="true" />
      Last: {{ operationLabel(latest.operationType) }}
      <OperationStatusBadge :status="latest.status" />
      <span class="underline-offset-2 hover:underline">View output</span>
    </button>
    <p v-if="actionError" class="mt-2 text-xs text-status-failed" role="alert">{{ actionError }}</p>
  </li>
</template>
