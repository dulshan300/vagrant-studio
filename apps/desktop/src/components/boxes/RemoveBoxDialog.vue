<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { TriangleAlert } from '@lucide/vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import { Button } from '@/components/ui/button'
import { activeUsers, boxRef } from '@/lib/boxes'
import { boxesApi } from '@/services/boxes'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { InstalledBox } from '@/types/boxes'
import type { Confirmation, Operation } from '@/types/operations'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ box: InstalledBox; usageError: string | null }>()
const emit = defineEmits<{ started: [operation: Operation] }>()
const operations = useOperationsStore()

const confirmation = ref<Confirmation | null>(null)
const typed = ref('')
const busy = ref(false)
const error = ref<string | null>(null)

const users = computed(() => activeUsers(props.box))
const staleUsers = computed(() => props.box.users.length - users.value.length)

// A fresh one-time token each time the dialog opens (it may mount open).
watch(
  open,
  async (isOpen) => {
    if (!isOpen) return
    confirmation.value = null
    typed.value = ''
    error.value = null
    const target = boxRef(props.box)
    if (!target) {
      error.value =
        'Vagrant didn’t report this box’s provider and version, so it can’t be removed here.'
      return
    }
    try {
      confirmation.value = await boxesApi.requestRemoval(target)
    } catch (e) {
      error.value = errorMessage(e)
    }
  },
  { immediate: true },
)

async function submit() {
  const target = boxRef(props.box)
  if (!confirmation.value || !target) return
  busy.value = true
  try {
    const op = operations.track(
      await boxesApi.remove(target, { token: confirmation.value.token, typed: typed.value }),
    )
    open.value = false
    emit('started', op)
  } catch (e) {
    error.value = errorMessage(e)
    // The token is used up by any attempt; reopen for a new one.
    confirmation.value = null
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <AppDialog v-model:open="open" :title="`Remove ${box.name}?`" size="lg">
    <div class="space-y-4">
      <p>
        Vagrant deletes version <strong class="font-mono">{{ box.version }}</strong> for
        <strong>{{ box.provider }}</strong>
        <template v-if="box.architecture && box.architecture !== 'n/a'">
          ({{ box.architecture }})</template
        >
        from its box store. Other versions and providers of this box are kept. It can be downloaded
        again later, unless it was added from a file you no longer have.
      </p>

      <div
        v-if="users.length"
        class="space-y-2 rounded-md border border-status-transition/50 bg-status-transition/10 px-3 py-2.5"
      >
        <p class="flex items-start gap-2 font-medium">
          <TriangleAlert class="mt-0.5 size-4 shrink-0 text-status-transition" aria-hidden="true" />
          Vagrant records {{ users.length === 1 ? 'a machine' : `${users.length} machines` }}
          created from this box.
        </p>
        <ul class="space-y-1 pl-6 text-xs">
          <li v-for="u in users" :key="`${u.folder}|${u.machine}`">
            <span class="font-mono">{{ u.machine }}</span>
            <template v-if="u.projectName">
              in <strong>{{ u.projectName }}</strong> · {{ u.environmentName }}</template
            >
            <span v-else class="font-mono text-muted"> in {{ u.folder }}</span>
            <span v-if="u.state" class="text-muted"> (last recorded: {{ u.state }})</span>
          </li>
        </ul>
        <p class="pl-6 text-xs text-muted">
          Vagrant warns that removing a box in use could corrupt those environments: machines
          created as linked clones (the VMware Desktop default) read their disk from the box.
          Destroy them first if you no longer need them.
        </p>
      </div>
      <p v-else-if="usageError" class="text-xs text-muted">
        Couldn’t check which machines use this box: {{ usageError }}
      </p>
      <p v-else class="text-xs text-muted">
        Vagrant records no existing machine created from this box.
        <template v-if="staleUsers">
          ({{ staleUsers }} older record{{ staleUsers === 1 ? '' : 's' }} point to folders that no
          longer exist and are ignored by Vagrant.)
        </template>
      </p>

      <div v-if="confirmation" class="space-y-1.5">
        <label for="remove-box-confirm" class="block text-xs font-medium">
          Type <strong class="font-mono">{{ confirmation.phrase }}</strong> to confirm
        </label>
        <TextInput id="remove-box-confirm" v-model="typed" autocomplete="off" spellcheck="false" />
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
        Remove box
      </Button>
    </template>
  </AppDialog>
</template>
