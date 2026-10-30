<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { LoaderCircle } from '@lucide/vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import { Button } from '@/components/ui/button'
import { targetMachines } from '@/lib/operations'
import { errorMessage } from '@/services/errors'
import { operationsApi } from '@/services/operations'
import { useOperationsStore } from '@/stores/operations'
import type { Confirmation, SnapshotChange } from '@/types/operations'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ environmentId: string; machine: string }>()
const operations = useOperationsStore()

const names = ref<string[] | null>(null)
const newName = ref('')
const error = ref<string | null>(null)
const typed = ref('')
const pending = ref<{
  change: SnapshotChange
  name: string
  confirmation: Confirmation
} | null>(null)

const busy = computed(() =>
  operations
    .activeFor(props.environmentId)
    .some((op) => targetMachines(op).includes(props.machine)),
)

async function load() {
  try {
    const result = await operationsApi.snapshots(props.environmentId, props.machine)
    names.value = result.names
  } catch (e) {
    error.value = errorMessage(e)
  }
}

watch(open, (isOpen) => {
  if (!isOpen) return
  names.value = null
  newName.value = ''
  error.value = null
  pending.value = null
  void load()
})
// The list changes when an operation on this machine ends.
watch(busy, (isBusy) => {
  if (!isBusy && open.value) void load()
})

async function save() {
  error.value = null
  try {
    const op = await operationsApi.saveSnapshot(
      props.environmentId,
      props.machine,
      newName.value.trim(),
    )
    operations.track(op)
    newName.value = ''
  } catch (e) {
    error.value = errorMessage(e)
  }
}

async function ask(change: SnapshotChange, name: string) {
  error.value = null
  typed.value = ''
  try {
    pending.value = {
      change,
      name,
      confirmation: await operationsApi.requestSnapshotConfirmation(
        props.environmentId,
        props.machine,
        change,
        name,
      ),
    }
  } catch (e) {
    error.value = errorMessage(e)
  }
}

async function confirm() {
  if (!pending.value) return
  const { change, name, confirmation } = pending.value
  // Any attempt uses up the token, so a retry asks again.
  pending.value = null
  try {
    const op = await operationsApi.changeSnapshot(
      props.environmentId,
      props.machine,
      change,
      name,
      {
        token: confirmation.token,
        typed: typed.value,
      },
    )
    operations.track(op)
  } catch (e) {
    error.value = errorMessage(e)
  }
}
</script>

<template>
  <AppDialog v-model:open="open" :title="`Snapshots of ${machine}`">
    <div class="space-y-4">
      <form class="flex gap-2" @submit.prevent="save">
        <TextInput v-model="newName" placeholder="Snapshot name" aria-label="Snapshot name" />
        <Button type="submit" :disabled="busy || !newName.trim()">Save snapshot</Button>
      </form>

      <p v-if="busy" class="flex items-center gap-1.5 text-xs text-muted">
        <LoaderCircle class="size-3 animate-spin" aria-hidden="true" />
        An operation is running on this machine.
      </p>

      <p v-if="names === null && !error" class="text-xs text-muted">Reading snapshots…</p>
      <p v-else-if="names?.length === 0" class="text-xs text-muted">No snapshots yet.</p>
      <ul v-else-if="names" class="divide-y rounded-md border">
        <li v-for="name in names" :key="name" class="flex items-center gap-1 px-3 py-1.5">
          <span class="min-w-0 flex-1 truncate font-mono text-xs">{{ name }}</span>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2"
            :disabled="busy"
            @click="ask('restore', name)"
          >
            Restore
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 text-status-failed"
            :disabled="busy"
            @click="ask('delete', name)"
          >
            Delete
          </Button>
        </li>
      </ul>

      <div v-if="pending" class="space-y-2 rounded-md border p-3">
        <p class="text-xs">
          <template v-if="pending.change === 'restore'">
            Restoring puts {{ machine }} back to this snapshot. Anything that changed on the machine
            since then is lost. Provisioners don't run.
          </template>
          <template v-else>Deleting removes this snapshot from {{ machine }} for good.</template>
        </p>
        <label for="snapshot-confirm" class="block text-xs font-medium">
          Type <strong class="font-mono">{{ pending.confirmation.phrase }}</strong> to confirm
        </label>
        <TextInput id="snapshot-confirm" v-model="typed" autocomplete="off" />
        <div class="flex justify-end gap-2">
          <Button variant="secondary" size="sm" @click="pending = null">Cancel</Button>
          <Button
            variant="destructive"
            size="sm"
            :disabled="typed.trim() !== pending.confirmation.phrase"
            @click="confirm"
          >
            {{ pending.change === 'restore' ? 'Restore' : 'Delete' }}
          </Button>
        </div>
      </div>

      <p v-if="error" class="text-sm whitespace-pre-wrap text-status-failed" role="alert">
        {{ error }}
      </p>
    </div>
  </AppDialog>
</template>
