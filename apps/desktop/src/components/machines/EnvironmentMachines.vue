<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import {
  CircleAlert,
  CircleCheck,
  CircleX,
  History,
  LoaderCircle,
  SquareTerminal,
} from '@lucide/vue'
import { Button } from '@/components/ui/button'
import StatusBadge from '@/components/status/StatusBadge.vue'
import DestroyMachinesDialog from './DestroyMachinesDialog.vue'
import ProvisionMachinesDialog from './ProvisionMachinesDialog.vue'
import { ACTION_META } from './actions'
import {
  actionsFor,
  LIFECYCLE_ACTIONS,
  machineState,
  operationLabel,
  primaryAction,
  targetMachines,
} from '@/lib/operations'
import { relativeTime } from '@/lib/projects'
import { PROVISION_COMMANDS, provisioningOutcome, type ProvisionCommand } from '@/lib/provisioning'
import { hasBackend } from '@/services/app'
import { errorMessage } from '@/services/errors'
import { terminalApi } from '@/services/terminal'
import { useOperationsStore } from '@/stores/operations'
import type { EnvironmentStatus, LifecycleAction, Operation } from '@/types/operations'
import type { Environment } from '@/types/projects'

const props = defineProps<{ environment: Environment; status: EnvironmentStatus }>()
const operations = useOperationsStore()
// Provider capabilities (e.g. no suspend for Docker) come from the host checks.
onMounted(() => {
  if (hasBackend() && !operations.readiness && !operations.readinessLoading)
    void operations.checkHost()
  if (hasBackend()) operations.loadLatestProvisioning(props.environment.id).catch(() => undefined)
})

const selected = ref<string[]>([])
const actionError = ref<string | null>(null)
const destroyOpen = ref(false)
const destroyTargets = ref<string[]>([])
const provisionOpen = ref(false)
const provisionTargets = ref<string[]>([])
const provisionCommands = ref<ProvisionCommand[]>([])

const machines = computed(() =>
  props.status.machines.map((m) => ({
    ...m,
    ui: machineState(m.state),
    capabilities:
      operations.readiness?.providers.find((p) => p.id === m.provider)?.capabilities ?? [],
  })),
)
// Drop selections for machines Vagrant no longer reports.
watch(machines, (list) => {
  selected.value = selected.value.filter((n) => list.some((m) => m.name === n))
})

const allSelected = computed(
  () => machines.value.length > 0 && selected.value.length === machines.value.length,
)
/** The bar acts on the selection, or on every machine when nothing is selected. */
const targets = computed(() =>
  selected.value.length ? selected.value : machines.value.map((m) => m.name),
)
const scopeLabel = computed(() => {
  if (selected.value.length) return `${selected.value.length} selected`
  return machines.value.length === 1 ? '1 machine' : `All ${machines.value.length} machines`
})

const pending = computed(() => operations.activeFor(props.environment.id))

/** Operations queued or running for a machine (named alone or with others). */
function pendingFor(name: string): Operation[] {
  return pending.value.filter((op) => targetMachines(op).includes(name))
}

function allowedFor(action: LifecycleAction, names: string[]): boolean {
  return machines.value
    .filter((m) => names.includes(m.name))
    .some((m) => actionsFor(m.ui, m.capabilities).includes(action))
}

/** Provision opens a dialog that can also start or restart machines first. */
function allowed(action: LifecycleAction): boolean {
  if (action === 'provision') return PROVISION_COMMANDS.some((c) => allowedFor(c, targets.value))
  return allowedFor(action, targets.value)
}

/** The latest recorded provisioner results, by machine name. */
const lastProvisioning = computed(() =>
  Object.fromEntries(
    (operations.latestProvisioning[props.environment.id] ?? []).map((m) => [
      m.machine,
      { ...provisioningOutcome(m), at: m.finishedAt, operationId: m.operationId },
    ]),
  ),
)

function toggleAll() {
  selected.value = allSelected.value ? [] : machines.value.map((m) => m.name)
}

async function openSsh(name: string) {
  actionError.value = null
  try {
    await terminalApi.openSsh(props.environment.id, name)
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}

async function run(action: LifecycleAction, names: string[]) {
  actionError.value = null
  if (action === 'destroy') {
    destroyTargets.value = [...names]
    destroyOpen.value = true
    return
  }
  if (action === 'provision') {
    provisionTargets.value = [...names]
    provisionCommands.value = PROVISION_COMMANDS.filter((c) => allowedFor(c, names))
    provisionOpen.value = true
    return
  }
  try {
    await operations.runLifecycle(props.environment.id, action, names)
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}
</script>

<template>
  <div>
    <p v-if="status.error" class="flex items-start gap-2 px-4 py-3 text-xs text-status-failed">
      <CircleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
      <span class="whitespace-pre-wrap">{{ status.error.message }}</span>
    </p>
    <p v-else-if="!machines.length" class="px-4 py-3 text-xs text-muted">
      Vagrant reported no machines.
    </p>
    <template v-else>
      <div
        class="flex flex-wrap items-center gap-1 border-b bg-surface-muted/40 px-4 py-1.5"
        role="toolbar"
        :aria-label="`Actions for ${scopeLabel.toLowerCase()}`"
      >
        <label class="mr-3 flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            :checked="allSelected"
            :indeterminate="selected.length > 0 && !allSelected"
            aria-label="Select all machines"
            @change="toggleAll"
          />
          <span class="text-muted">{{ scopeLabel }}</span>
        </label>
        <Button
          v-for="action in LIFECYCLE_ACTIONS"
          :key="action"
          variant="ghost"
          size="sm"
          :class="action === 'destroy' ? 'ml-auto h-7 px-2 text-status-failed' : 'h-7 px-2'"
          :disabled="!allowed(action)"
          :title="ACTION_META[action].hint"
          @click="run(action, targets)"
        >
          <component :is="ACTION_META[action].icon" aria-hidden="true" />
          {{ ACTION_META[action].label }}
        </Button>
      </div>
      <ul class="divide-y">
        <li
          v-for="m in machines"
          :key="m.name"
          class="grid grid-cols-[auto_minmax(0,1fr)_auto_6rem] items-center gap-3 px-4 py-2.5"
          :title="m.stateLong ?? undefined"
        >
          <input
            v-model="selected"
            type="checkbox"
            :value="m.name"
            :aria-label="`Select ${m.name}`"
          />
          <div class="min-w-0">
            <div class="truncate font-mono text-sm font-medium">{{ m.name }}</div>
            <div class="flex min-w-0 items-center gap-1.5 text-[11px] text-muted">
              <span class="shrink-0">{{ m.provider ?? 'provider unknown' }}</span>
              <template v-if="lastProvisioning[m.name]">
                <span aria-hidden="true">·</span>
                <button
                  type="button"
                  :class="[
                    'flex min-w-0 items-center gap-1 hover:underline',
                    lastProvisioning[m.name]!.status === 'succeeded' ? '' : 'text-status-failed',
                  ]"
                  :title="`Last provisioning ${relativeTime(lastProvisioning[m.name]!.at)}; open its output`"
                  @click="operations.open(lastProvisioning[m.name]!.operationId)"
                >
                  <CircleCheck
                    v-if="lastProvisioning[m.name]!.status === 'succeeded'"
                    class="size-3 shrink-0"
                    aria-hidden="true"
                  />
                  <CircleX v-else class="size-3 shrink-0" aria-hidden="true" />
                  <span class="truncate">
                    {{ lastProvisioning[m.name]!.text }} ·
                    {{ relativeTime(lastProvisioning[m.name]!.at) }}
                  </span>
                </button>
              </template>
            </div>
          </div>
          <span class="flex items-center gap-2">
            <span
              v-if="pendingFor(m.name).length"
              class="flex items-center gap-1 text-[11px] text-muted"
            >
              <LoaderCircle class="size-3 animate-spin" aria-hidden="true" />
              {{ operationLabel(pendingFor(m.name)[0]!.operationType) }}
            </span>
            <StatusBadge :state="m.ui" />
          </span>
          <span class="flex items-center justify-end gap-0.5">
            <Button
              variant="ghost"
              size="icon"
              class="size-7"
              title="Operations that named this machine"
              :aria-label="`Operation history of ${m.name}`"
              as-child
            >
              <RouterLink
                :to="{
                  path: '/operations',
                  query: {
                    project: environment.projectId,
                    environment: environment.id,
                    machine: m.name,
                  },
                }"
              >
                <History />
              </RouterLink>
            </Button>
            <Button
              v-if="m.ui === 'running' && m.capabilities.includes('ssh')"
              variant="ghost"
              size="icon"
              class="size-7"
              title="vagrant ssh: open a shell on the machine in a new terminal window"
              :aria-label="`SSH into ${m.name}`"
              @click="openSsh(m.name)"
            >
              <SquareTerminal />
            </Button>
            <Button
              v-if="primaryAction(m.ui)"
              variant="ghost"
              size="icon"
              class="size-7"
              :title="ACTION_META[primaryAction(m.ui)!].hint"
              :aria-label="`${ACTION_META[primaryAction(m.ui)!].label} ${m.name}`"
              @click="run(primaryAction(m.ui)!, [m.name])"
            >
              <component :is="ACTION_META[primaryAction(m.ui)!].icon" />
            </Button>
          </span>
        </li>
      </ul>
    </template>
    <p v-if="actionError" class="border-t px-4 py-2 text-xs text-status-failed" role="alert">
      {{ actionError }}
    </p>

    <ProvisionMachinesDialog
      v-model:open="provisionOpen"
      :environment-id="environment.id"
      :machines="provisionTargets"
      :commands="provisionCommands"
    />
    <DestroyMachinesDialog
      v-model:open="destroyOpen"
      :environment-id="environment.id"
      :environment-name="environment.name"
      :machines="destroyTargets"
    />
  </div>
</template>
