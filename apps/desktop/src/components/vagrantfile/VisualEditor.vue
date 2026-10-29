<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ArrowDown, ArrowUp, Plus, Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import MachineSettingsForm from './MachineSettingsForm.vue'
import { hostPortKey, move, newForwardedPort, newMachine, tcpHostPorts } from '@/lib/vagrantfile'
import { cn } from '@/lib/utils'
import { hasBackend } from '@/services/app'
import { networkApi } from '@/services/network'
import type { BridgedInterface, VagrantConfig } from '@/types/vagrantfile'

/** Forms for the supported Vagrantfile subset (crates/config-model). */
const config = defineModel<VagrantConfig>({ required: true })
const props = defineProps<{
  errors: Record<string, string>
  /** The environment's provider, offered first for CPU and memory. */
  provider?: string | null
}>()

/** `shared` (settings on `config`) or a machine index. */
const selected = ref<'shared' | number>('shared')
watch(
  () => config.value.machines.length,
  (n) => {
    if (typeof selected.value === 'number' && selected.value >= n) {
      selected.value = n ? n - 1 : 'shared'
    }
  },
)

const machine = computed(() =>
  typeof selected.value === 'number' ? config.value.machines[selected.value] : undefined,
)
const multi = computed(() => config.value.machines.length > 0)

const hasErrors = (prefix: string) =>
  Object.keys(props.errors).some((path) => path.startsWith(prefix))
const err = (path: string) => props.errors[path] ?? null

function addMachine() {
  config.value.machines.push(newMachine(config.value.machines))
  selected.value = config.value.machines.length - 1
}

function moveMachine(delta: -1 | 1) {
  if (typeof selected.value !== 'number') return
  const to = selected.value + delta
  move(config.value.machines, selected.value, delta)
  if (to >= 0 && to < config.value.machines.length) selected.value = to
}

// Host facts for the network forms; failures only mean no suggestions or warnings.
const bridges = ref<BridgedInterface[]>([])
const portsInUse = ref<Record<string, boolean>>({})
onMounted(async () => {
  if (hasBackend()) bridges.value = await networkApi.listBridgedInterfaces().catch(() => [])
})

const PORT_CHECK_DELAY_MS = 500
let portTimer: ReturnType<typeof setTimeout> | undefined
watch(
  () => JSON.stringify(tcpHostPorts(config.value)),
  () => {
    clearTimeout(portTimer)
    if (!hasBackend()) return
    portTimer = setTimeout(async () => {
      const ports = tcpHostPorts(config.value)
      const result = await networkApi.checkHostPorts(ports).catch(() => null)
      if (!result) return
      portsInUse.value = Object.fromEntries(
        ports.map((p, i) => [hostPortKey(p.hostIp, p.port), result[i] ?? false]),
      )
    }, PORT_CHECK_DELAY_MS)
  },
  { immediate: true },
)
onBeforeUnmount(() => clearTimeout(portTimer))

const navItem = (active: boolean) =>
  cn(
    'flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-left text-sm',
    active ? 'bg-surface-muted font-medium' : 'hover:bg-surface-muted/60',
  )
</script>

<template>
  <div class="grid gap-6 md:grid-cols-[13rem_minmax(0,1fr)]">
    <nav aria-label="Machines in this Vagrantfile" class="space-y-1">
      <button
        type="button"
        :class="navItem(selected === 'shared')"
        :aria-current="selected === 'shared' ? 'true' : undefined"
        @click="selected = 'shared'"
      >
        <span>{{ multi ? 'Shared settings' : 'Machine “default”' }}</span>
        <span
          v-if="hasErrors('defaults.')"
          class="size-2 rounded-full bg-status-failed"
          aria-label="has problems"
        />
      </button>
      <button
        v-for="(m, i) in config.machines"
        :key="i"
        type="button"
        :class="navItem(selected === i)"
        :aria-current="selected === i ? 'true' : undefined"
        @click="selected = i"
      >
        <span class="truncate font-mono text-xs">{{ m.name || '(unnamed)' }}</span>
        <span class="flex items-center gap-1.5">
          <span v-if="m.primary" class="text-[10px] text-muted uppercase">primary</span>
          <span
            v-if="hasErrors(`machines.${i}.`)"
            class="size-2 rounded-full bg-status-failed"
            aria-label="has problems"
          />
        </span>
      </button>
      <Button variant="ghost" size="sm" class="w-full justify-start" @click="addMachine">
        <Plus /> Add machine
      </Button>
      <p class="px-1 pt-2 text-xs text-muted">
        <template v-if="multi">
          Shared settings apply to every machine; a machine's own value wins where both are set.
        </template>
        <template v-else>
          With no named machines, Vagrant runs one machine called “default”. Adding a machine turns
          these settings into shared settings, and “default” is no longer created.
        </template>
      </p>
    </nav>

    <div class="min-w-0">
      <template v-if="machine && typeof selected === 'number'">
        <div class="mb-6 flex flex-wrap items-end gap-4 border-b pb-4">
          <FormField
            label="Machine name"
            :for="`m${selected}-name`"
            :error="err(`machines.${selected}.name`)"
            class="min-w-48 flex-1"
          >
            <TextInput
              :id="`m${selected}-name`"
              v-model="machine.name"
              class="font-mono"
              :invalid="!!err(`machines.${selected}.name`)"
              spellcheck="false"
            />
          </FormField>
          <div class="space-y-1 pb-2">
            <label class="flex items-center gap-1.5 text-sm">
              <input v-model="machine.primary" type="checkbox" /> Primary
            </label>
            <p
              v-if="err(`machines.${selected}.primary`)"
              class="text-xs text-status-failed"
              role="alert"
            >
              {{ err(`machines.${selected}.primary`) }}
            </p>
          </div>
          <label
            class="flex items-center gap-1.5 pb-2 text-sm"
            title="Off: a plain “vagrant up” skips this machine; start it by name."
          >
            <input v-model="machine.autostart" type="checkbox" /> Starts with “up”
          </label>
          <div class="flex items-center gap-1 pb-1">
            <Button
              variant="ghost"
              size="icon"
              class="size-8 text-muted"
              aria-label="Move machine up"
              :disabled="selected === 0"
              @click="moveMachine(-1)"
            >
              <ArrowUp />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="size-8 text-muted"
              aria-label="Move machine down"
              :disabled="selected === config.machines.length - 1"
              @click="moveMachine(1)"
            >
              <ArrowDown />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="text-status-failed"
              @click="config.machines.splice(selected, 1)"
            >
              <Trash2 /> Remove
            </Button>
          </div>
        </div>
        <MachineSettingsForm
          :key="`m${selected}`"
          v-model:settings="machine.settings"
          :path="`machines.${selected}.settings`"
          :id-prefix="`m${selected}`"
          :errors="errors"
          :inherited="config.defaults"
          :new-forwarded-port="() => newForwardedPort(config)"
          :bridges="bridges"
          :ports-in-use="portsInUse"
          :provider="provider"
        />
      </template>
      <MachineSettingsForm
        v-else
        key="shared"
        v-model:settings="config.defaults"
        path="defaults"
        id-prefix="shared"
        :errors="errors"
        :new-forwarded-port="() => newForwardedPort(config)"
        :bridges="bridges"
        :ports-in-use="portsInUse"
        :provider="provider"
      />
    </div>
  </div>
</template>
