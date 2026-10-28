<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { FileCheck, FileCode, RefreshCw, ScrollText, Trash2 } from '@lucide/vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import ProviderSelect from '@/components/providers/ProviderSelect.vue'
import { useProviderChoices } from '@/composables/useProviderChoices'
import { projectsApi } from '@/services/projects'
import EnvironmentMachines from '@/components/machines/EnvironmentMachines.vue'
import OperationStatusBadge from '@/components/operations/OperationStatusBadge.vue'
import { operationLabel } from '@/lib/operations'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { Environment } from '@/types/projects'

const props = defineProps<{ environment: Environment }>()
const emit = defineEmits<{ remove: []; updated: [environment: Environment] }>()
const providers = useProviderChoices()
const router = useRouter()

const operations = useOperationsStore()
const actionError = ref<string | null>(null)

onMounted(() => {
  void operations.loadStatus(props.environment.id).catch(() => undefined)
})

const status = computed(() => operations.statuses[props.environment.id] ?? null)
const latest = computed(() => operations.latestFor(props.environment.id))
// Lifecycle operations refresh status themselves, so Refresh waits for them.
const pending = computed(() => operations.activeFor(props.environment.id))
const busy = computed(() => pending.value.length > 0)

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
          variant="ghost"
          size="sm"
          @click="
            router.push(
              `/projects/${environment.projectId}/environments/${environment.id}/vagrantfile`,
            )
          "
        >
          <FileCode /> {{ environment.vagrantfilePath ? 'Vagrantfile' : 'Create Vagrantfile' }}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          :disabled="busy || !environment.vagrantfilePath"
          @click="run(operations.refreshStatus)"
        >
          <RefreshCw
            :class="pending.some((op) => op.operationType === 'status') ? 'animate-spin' : ''"
          />
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
      No Vagrantfile detected in this folder, so Vagrant commands are disabled. Use Create
      Vagrantfile to add one.
    </p>

    <EnvironmentMachines v-if="status" :environment="environment" :status="status" />
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
