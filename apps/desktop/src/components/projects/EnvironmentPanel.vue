<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  Code,
  TriangleAlert,
  Ellipsis,
  FileCheck,
  FileCode,
  LoaderCircle,
  RefreshCw,
  ScrollText,
  Trash2,
} from '@lucide/vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import ProviderSelect from '@/components/providers/ProviderSelect.vue'
import { useProviderChoices } from '@/composables/useProviderChoices'
import { projectsApi } from '@/services/projects'
import EnvironmentMachines from '@/components/machines/EnvironmentMachines.vue'
import OperationStatusBadge from '@/components/operations/OperationStatusBadge.vue'
import { operationLabel } from '@/lib/operations'
import { clashesFor, portOwnerLabel } from '@/lib/ports'
import { relativeTime } from '@/lib/projects'
import { editorApi } from '@/services/editor'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { Environment } from '@/types/projects'
import type { PortUse } from '@/types/vagrantfile'

const props = defineProps<{ environment: Environment; forwardedPorts?: PortUse[] }>()
const emit = defineEmits<{ remove: []; updated: [environment: Environment] }>()
const providers = useProviderChoices()
const router = useRouter()

const operations = useOperationsStore()
const actionError = ref<string | null>(null)

onMounted(() => {
  void operations.loadStatus(props.environment.id).catch(() => undefined)
})

const status = computed(() => operations.statuses[props.environment.id] ?? null)
const clashes = computed(() => clashesFor(props.environment.id, props.forwardedPorts ?? []))
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

async function openInEditor() {
  actionError.value = null
  try {
    await editorApi.open(props.environment.projectId, props.environment.id)
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
  <li class="rounded-lg border bg-surface shadow-xs">
    <header class="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="font-semibold">{{ environment.name }}</span>
          <span
            class="rounded-full bg-surface-muted px-2 py-0.5 text-[11px] text-muted capitalize"
            >{{ environment.environmentType }}</span
          >
        </div>
        <div class="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1">
          <code class="truncate font-mono text-xs text-muted" :title="environment.rootPath">{{
            environment.rootPath
          }}</code>
          <label
            :for="`env-provider-${environment.id}`"
            class="flex items-center gap-2 text-xs text-muted"
          >
            Provider
            <ProviderSelect
              :id="`env-provider-${environment.id}`"
              :model-value="environment.defaultProvider"
              :choices="providers.choices.value"
              allow-unset
              class="!py-0.5 text-xs text-foreground"
              :title="'Provider used for new machines; existing machines keep theirs.'"
              @update:model-value="changeProvider"
            />
          </label>
        </div>
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
          variant="ghost"
          size="sm"
          :disabled="busy || !environment.vagrantfilePath"
          @click="run(operations.validate)"
        >
          <FileCheck /> Validate
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
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              variant="ghost"
              size="icon"
              class="size-8"
              :aria-label="`More actions for environment ${environment.name}`"
            >
              <Ellipsis />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuItem @select="openInEditor"><Code /> Open in editor</DropdownMenuItem>
            <DropdownMenuSeparator class="my-1 h-px bg-border" />
            <DropdownMenuItem destructive @select="$emit('remove')">
              <Trash2 /> Remove environment…
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>

    <ul
      v-if="clashes.length"
      class="space-y-1 border-b bg-status-transition/10 px-4 py-2 text-xs"
      aria-label="Host port conflicts"
    >
      <li v-for="c in clashes" :key="`${c.protocol}:${c.host}`" class="flex items-start gap-2">
        <TriangleAlert class="mt-0.5 size-3.5 shrink-0 text-status-transition" aria-hidden="true" />
        <span>
          Host port <span class="font-mono">{{ c.host }}</span>
          <template v-if="c.protocol === 'udp'"> (UDP)</template>
          ({{ c.machines.join(', ') }}) is also forwarded by
          <template v-for="(o, i) in c.others" :key="`${o.environmentId}:${o.machine}`">
            <template v-if="i > 0">, </template>
            <RouterLink
              v-if="o.projectId !== environment.projectId"
              :to="{ name: 'project', params: { id: o.projectId } }"
              class="font-medium underline-offset-2 hover:underline"
              >{{ portOwnerLabel(o, environment.projectId) }}</RouterLink
            >
            <span v-else class="font-medium">{{
              portOwnerLabel(o, environment.projectId)
            }}</span> </template
          >. Only one of them can be running at a time.
        </span>
      </li>
    </ul>

    <p v-if="!environment.vagrantfilePath" class="px-4 py-3 text-xs text-muted">
      No Vagrantfile detected in this folder, so Vagrant commands are disabled. Use Create
      Vagrantfile to add one.
    </p>
    <EnvironmentMachines v-if="status" :environment="environment" :status="status" />
    <p v-else-if="environment.vagrantfilePath" class="px-4 py-3 text-xs text-muted">
      Machine state hasn't been checked yet. Use Refresh status to ask Vagrant.
    </p>

    <footer
      v-if="status || latest || pending.length || actionError"
      class="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 border-t px-4 py-2 text-[11px] text-muted"
    >
      <span class="flex flex-wrap items-center gap-1.5">
        <span v-if="status">Reported by Vagrant {{ relativeTime(status.observedAt) }}</span>
        <template v-if="pending.length">
          <span v-if="status" aria-hidden="true">·</span>
          <LoaderCircle class="size-3 animate-spin" aria-hidden="true" />
          <template v-for="(op, i) in pending" :key="op.id">
            <span v-if="i > 0" aria-hidden="true">·</span>
            <button
              type="button"
              class="hover:text-foreground hover:underline"
              @click="operations.open(op.id)"
            >
              {{ operationLabel(op.operationType) }}
              {{ op.status === 'queued' ? '(queued)' : '' }}
            </button>
          </template>
        </template>
      </span>
      <button
        v-if="latest"
        type="button"
        class="flex items-center gap-2 hover:text-foreground"
        @click="operations.open(latest.id)"
      >
        <ScrollText class="size-3.5" aria-hidden="true" />
        Last: {{ operationLabel(latest.operationType) }}
        <OperationStatusBadge :status="latest.status" />
        <span class="underline-offset-2 hover:underline">View output</span>
      </button>
      <p v-if="actionError" class="w-full text-xs text-status-failed" role="alert">
        {{ actionError }}
      </p>
    </footer>
  </li>
</template>
