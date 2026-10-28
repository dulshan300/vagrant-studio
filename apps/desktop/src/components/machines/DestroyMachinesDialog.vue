<script setup lang="ts">
import { ref, watch } from 'vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import { Button } from '@/components/ui/button'
import { operationsApi } from '@/services/operations'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { Confirmation } from '@/types/operations'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ environmentId: string; environmentName: string; machines: string[] }>()
const emit = defineEmits<{ started: [] }>()
const operations = useOperationsStore()

const confirmation = ref<Confirmation | null>(null)
const typed = ref('')
const busy = ref(false)
const error = ref<string | null>(null)

// A fresh one-time token each time the dialog opens.
watch(open, async (isOpen) => {
  if (!isOpen) return
  confirmation.value = null
  typed.value = ''
  error.value = null
  try {
    confirmation.value = await operationsApi.requestConfirmation(
      props.environmentId,
      'destroy',
      props.machines,
    )
  } catch (e) {
    error.value = errorMessage(e)
  }
})

async function submit() {
  if (!confirmation.value) return
  busy.value = true
  try {
    await operations.runLifecycle(props.environmentId, 'destroy', props.machines, {
      token: confirmation.value.token,
      typed: typed.value,
    })
    open.value = false
    emit('started')
  } catch (e) {
    error.value = errorMessage(e)
    // The token is used up by any attempt; get a new one for a retry.
    confirmation.value = null
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    :title="
      machines.length === 1 ? `Destroy “${machines[0]}”?` : `Destroy ${machines.length} machines?`
    "
  >
    <div class="space-y-4">
      <p>
        Vagrant deletes
        {{ machines.length === 1 ? 'this virtual machine' : 'these virtual machines' }} and their
        disks in <strong>{{ environmentName }}</strong
        >. Anything stored only inside them is lost. The Vagrantfile and project files are kept, so
        Start can create them again.
      </p>
      <ul class="flex flex-wrap gap-1.5">
        <li
          v-for="name in machines"
          :key="name"
          class="rounded bg-surface-muted px-1.5 py-0.5 font-mono text-xs"
        >
          {{ name }}
        </li>
      </ul>

      <div v-if="confirmation" class="space-y-1.5">
        <label for="destroy-confirm" class="block text-xs font-medium">
          Type <strong class="font-mono">{{ confirmation.phrase }}</strong> to confirm
        </label>
        <TextInput id="destroy-confirm" v-model="typed" autocomplete="off" />
      </div>

      <p v-if="error" class="text-sm text-status-failed" role="alert">{{ error }}</p>
    </div>
    <template #footer>
      <Button variant="secondary" @click="open = false">Cancel</Button>
      <Button
        variant="destructive"
        :disabled="busy || !confirmation || typed.trim() !== confirmation.phrase"
        @click="submit"
      >
        Destroy
      </Button>
    </template>
  </AppDialog>
</template>
