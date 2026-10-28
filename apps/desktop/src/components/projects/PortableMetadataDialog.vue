<script setup lang="ts">
import { ref, watch } from 'vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import { Button } from '@/components/ui/button'
import { useFormErrors } from '@/composables/useFormErrors'
import { projectsApi } from '@/services/projects'
import type { PortablePreview } from '@/types/projects'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ projectId: string }>()
const errors = useFormErrors()
const preview = ref<PortablePreview | null>(null)
const busy = ref(false)
const saved = ref(false)

watch(open, async (isOpen) => {
  if (!isOpen) return
  errors.clear()
  saved.value = false
  preview.value = null
  try {
    preview.value = await projectsApi.previewMetadata(props.projectId)
  } catch (e) {
    errors.set(e)
  }
})

const unchanged = () => preview.value?.existing === preview.value?.content

async function write() {
  if (!preview.value) return
  busy.value = true
  try {
    // Overwrite is only ever sent after the user has seen the existing file below.
    preview.value = await projectsApi.writeMetadata(
      props.projectId,
      preview.value.existing !== null,
    )
    saved.value = true
  } catch (e) {
    errors.set(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    title="Portable project file"
    description="Describes this project so it can be imported elsewhere. Contains no secrets, absolute paths or machine IDs. Commit it to Git only if you want to share it."
    size="lg"
  >
    <div v-if="preview" class="space-y-3">
      <p class="text-xs text-muted">
        <code class="font-mono break-all">{{ preview.path }}</code>
      </p>
      <p v-if="saved" class="text-sm text-status-running">Saved.</p>
      <p v-else-if="preview.existing === null" class="text-sm">This file doesn't exist yet.</p>
      <p v-else-if="unchanged()" class="text-sm">The file is already up to date.</p>
      <p v-else class="text-sm text-status-transition">
        The file exists with different content. Saving replaces it.
      </p>

      <div
        class="grid gap-3"
        :class="preview.existing !== null && !unchanged() && !saved && 'sm:grid-cols-2'"
      >
        <div v-if="preview.existing !== null && !unchanged() && !saved">
          <h3 class="mb-1 text-xs font-medium">Current file</h3>
          <pre class="max-h-72 overflow-auto rounded-md bg-surface-muted p-3 font-mono text-xs">{{
            preview.existing
          }}</pre>
        </div>
        <div>
          <h3 class="mb-1 text-xs font-medium">
            {{ saved || unchanged() ? 'Content' : 'New content' }}
          </h3>
          <pre class="max-h-72 overflow-auto rounded-md bg-surface-muted p-3 font-mono text-xs">{{
            preview.content
          }}</pre>
        </div>
      </div>
    </div>
    <p v-else-if="!errors.general.value" class="text-muted">Loading…</p>
    <p v-if="errors.general.value" class="text-sm text-status-failed" role="alert">
      {{ errors.general.value }}
    </p>

    <template #footer>
      <Button variant="secondary" @click="open = false">Close</Button>
      <Button v-if="preview && !unchanged() && !saved" :disabled="busy" @click="write">
        {{ preview.existing === null ? 'Create file' : 'Replace file' }}
      </Button>
    </template>
  </AppDialog>
</template>
