<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import { Button } from '@/components/ui/button'
import {
  PROVISION_COMMAND_LABELS,
  provisionerChoice,
  type ProvisionCommand,
} from '@/lib/provisioning'
import { operationsApi } from '@/services/operations'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { Provisioning } from '@/types/operations'
import type { ProvisionerList } from '@/types/vagrantfile'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{
  environmentId: string
  machines: string[]
  /** Commands the machines' states allow, best first. */
  commands: ProvisionCommand[]
}>()
const operations = useOperationsStore()

const list = ref<ProvisionerList | null>(null)
const command = ref<ProvisionCommand>('provision')
const mode = ref<'all' | 'only'>('all')
const chosen = ref<string[]>([])
const busy = ref(false)
const error = ref<string | null>(null)

const choice = computed(() =>
  list.value ? provisionerChoice(list.value.machines, props.machines) : null,
)
const neverRun = computed(() => choice.value?.named.filter((p) => p.run === 'never') ?? [])

watch(open, async (isOpen) => {
  if (!isOpen) return
  list.value = null
  command.value = props.commands[0] ?? 'provision'
  mode.value = 'all'
  chosen.value = []
  error.value = null
  try {
    list.value = await operationsApi.provisioners(props.environmentId)
  } catch (e) {
    error.value = errorMessage(e)
  }
})

// Chosen names in Vagrantfile order, the order Vagrant runs them in anyway.
const orderedChoice = computed(() =>
  (choice.value?.named ?? []).map((p) => p.name).filter((n) => chosen.value.includes(n)),
)

function provisioning(): Provisioning {
  if (mode.value === 'only') return { kind: 'only', names: orderedChoice.value }
  // `vagrant provision` runs every provisioner by default; `up` and
  // `reload` need `--provision`.
  return command.value === 'provision' ? { kind: 'default' } : { kind: 'all' }
}

async function submit() {
  busy.value = true
  error.value = null
  try {
    await operations.runLifecycle(props.environmentId, command.value, props.machines, {
      provisioning: provisioning(),
    })
    open.value = false
  } catch (e) {
    error.value = errorMessage(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    :title="
      machines.length === 1 ? `Provision “${machines[0]}”` : `Provision ${machines.length} machines`
    "
  >
    <div class="space-y-4 text-sm">
      <fieldset v-if="commands.length > 1" class="space-y-1.5">
        <legend class="mb-1 text-xs font-medium">Command</legend>
        <label v-for="c in commands" :key="c" class="flex items-center gap-2">
          <input v-model="command" type="radio" name="provision-command" :value="c" />
          {{ PROVISION_COMMAND_LABELS[c] }}
          <code class="font-mono text-[11px] text-muted">vagrant {{ c }}</code>
        </label>
      </fieldset>

      <fieldset class="space-y-1.5">
        <legend class="mb-1 text-xs font-medium">Provisioners</legend>
        <label class="flex items-start gap-2">
          <input v-model="mode" type="radio" name="provision-mode" value="all" class="mt-1" />
          <span>
            All provisioners
            <span v-if="neverRun.length" class="block text-xs text-muted">
              Except those set to run only when chosen by name.
            </span>
          </span>
        </label>
        <label class="flex items-start gap-2">
          <input
            v-model="mode"
            type="radio"
            name="provision-mode"
            value="only"
            class="mt-1"
            :disabled="!choice?.named.length"
          />
          <span :class="!choice?.named.length && 'text-muted'">Only these</span>
        </label>
        <ul v-if="choice?.named.length" class="ml-6 space-y-1">
          <li v-for="p in choice.named" :key="p.name">
            <label class="flex items-center gap-2">
              <input
                v-model="chosen"
                type="checkbox"
                :value="p.name"
                :disabled="mode !== 'only'"
                :aria-label="`Run ${p.name}`"
              />
              <span class="font-mono text-xs">{{ p.name }}</span>
              <span v-if="p.shared" class="text-[11px] text-muted">shared</span>
              <span v-if="p.run === 'never'" class="text-[11px] text-muted">
                only when chosen
              </span>
              <span v-else-if="p.run === 'always'" class="text-[11px] text-muted">always</span>
            </label>
          </li>
        </ul>
        <p v-if="list?.unavailable" class="text-xs text-muted">{{ list.unavailable }}</p>
        <p v-else-if="choice && choice.others > 0" class="text-xs text-muted">
          {{ choice.others }} other provisioner{{ choice.others === 1 ? '' : 's' }} can't be chosen
          on {{ machines.length === 1 ? 'its' : 'their' }} own: name
          {{ choice.others === 1 ? 'it' : 'them' }} in the Vagrantfile editor{{
            machines.length > 1 ? ', or choose one machine' : ''
          }}.
        </p>
      </fieldset>

      <p v-if="error" class="text-sm text-status-failed" role="alert">{{ error }}</p>
    </div>
    <template #footer>
      <Button variant="secondary" @click="open = false">Cancel</Button>
      <Button
        :disabled="busy || !list || (mode === 'only' && !orderedChoice.length)"
        @click="submit"
      >
        Run
      </Button>
    </template>
  </AppDialog>
</template>
