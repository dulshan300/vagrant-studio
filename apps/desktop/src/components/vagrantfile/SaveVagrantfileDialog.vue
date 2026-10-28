<script setup lang="ts">
import { ref, watch } from 'vue'
import { AlertTriangle } from '@lucide/vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import DiffView from '@/components/editor/DiffView.vue'
import { Button } from '@/components/ui/button'
import { errorMessage, isCommandError } from '@/services/errors'
import { vagrantfileApi } from '@/services/vagrantfile'
import type { SaveVagrantfileOutcome } from '@/types/vagrantfile'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{
  environmentId: string
  path: string
  /** Content on disk ('' when the file doesn't exist yet). */
  original: string
  proposed: string
  exists: boolean
  baseRevision: string | null
  /** Generated content would replace a file the visual editor didn't write. */
  replacesUnmanaged: boolean
  /** What a visual save loses (e.g. comments). */
  warnings: string[]
}>()
const emit = defineEmits<{
  saved: [outcome: SaveVagrantfileOutcome, validate: boolean]
  stale: []
}>()

const replaceConfirmed = ref(false)
const validateAfter = ref(true)
const changes = ref(0)
const busy = ref(false)
const error = ref<string | null>(null)

watch(open, (isOpen) => {
  if (!isOpen) return
  replaceConfirmed.value = false
  error.value = null
})

async function save() {
  busy.value = true
  error.value = null
  try {
    const outcome = await vagrantfileApi.save(props.environmentId, {
      content: props.proposed,
      baseRevision: props.baseRevision,
      replaceUnmanaged: replaceConfirmed.value,
    })
    open.value = false
    emit('saved', outcome, validateAfter.value)
  } catch (e) {
    if (isCommandError(e) && e.kind === 'stale') {
      open.value = false
      emit('stale')
    } else {
      error.value = errorMessage(e)
    }
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    size="lg"
    :title="exists ? 'Save the Vagrantfile?' : 'Create the Vagrantfile?'"
    :description="path"
  >
    <div class="space-y-4">
      <p class="text-sm">
        <template v-if="exists">
          {{ changes === 1 ? 'One section changes' : `${changes} sections change` }}. The current
          file is backed up first, then replaced in one step.
        </template>
        <template v-else>A new file is created with this content.</template>
      </p>
      <DiffView :original="original" :modified="proposed" @changes="changes = $event" />

      <ul v-if="warnings.length" class="space-y-1 text-sm">
        <li v-for="w in warnings" :key="w" class="flex gap-2">
          <AlertTriangle class="mt-0.5 size-4 shrink-0 text-status-transition" aria-hidden="true" />
          {{ w }}
        </li>
      </ul>

      <label
        v-if="replacesUnmanaged"
        class="flex items-start gap-2 rounded-md border border-status-transition/50 p-3 text-sm"
      >
        <input v-model="replaceConfirmed" type="checkbox" class="mt-1" />
        <span>
          Replace this hand-written Vagrantfile with the visual editor's version. Its formatting
          changes as shown above; the current file is kept as a backup.
        </span>
      </label>

      <label class="flex items-center gap-2 text-sm">
        <input v-model="validateAfter" type="checkbox" />
        Check it with <code class="font-mono text-xs">vagrant validate</code> after saving
      </label>

      <p v-if="error" class="text-sm text-status-failed" role="alert">{{ error }}</p>
    </div>
    <template #footer>
      <Button variant="secondary" @click="open = false">Keep editing</Button>
      <Button :disabled="busy || (replacesUnmanaged && !replaceConfirmed)" @click="save">
        {{ exists ? 'Save' : 'Create' }}
      </Button>
    </template>
  </AppDialog>
</template>
