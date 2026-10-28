<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import { Button } from '@/components/ui/button'
import { useFormErrors } from '@/composables/useFormErrors'
import { useProjectsStore } from '@/stores/projects'
import type { Project } from '@/types/projects'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ project: Project }>()
const store = useProjectsStore()
const router = useRouter()
const errors = useFormErrors()

const deleteFiles = ref(false)
const confirmName = ref('')
const busy = ref(false)

watch(open, (isOpen) => {
  if (!isOpen) return
  deleteFiles.value = false
  confirmName.value = ''
  errors.clear()
})

async function submit() {
  busy.value = true
  try {
    await store.remove(props.project.id, {
      deleteFiles: deleteFiles.value,
      confirmName: deleteFiles.value ? confirmName.value : null,
    })
    open.value = false
    await router.push({ name: 'projects' })
  } catch (e) {
    errors.set(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <AppDialog v-model:open="open" :title="`Remove “${project.name}”?`">
    <div class="space-y-4">
      <p>
        Vagrant Studio forgets this project: its environments, notes and history are removed from
        the app. The project folder and any virtual machines are not touched.
      </p>

      <label class="flex items-start gap-3 rounded-md border p-3">
        <input v-model="deleteFiles" type="checkbox" class="mt-1" />
        <span>
          <span class="block font-medium">Also move the project folder to the trash</span>
          <code class="block font-mono text-xs break-all text-muted">{{ project.rootPath }}</code>
          <span class="text-xs text-muted">
            Refused if Vagrant machines may still exist for it; destroy them first.
          </span>
        </span>
      </label>

      <div v-if="deleteFiles" class="space-y-1.5">
        <label for="dp-confirm" class="block text-xs font-medium">
          Type <strong>{{ project.name }}</strong> to confirm
        </label>
        <TextInput
          id="dp-confirm"
          v-model="confirmName"
          :invalid="!!errors.field.value.confirmName"
        />
        <p v-if="errors.field.value.confirmName" class="text-xs text-status-failed">
          {{ errors.field.value.confirmName }}
        </p>
      </div>

      <p v-if="errors.general.value" class="text-sm text-status-failed" role="alert">
        {{ errors.general.value }}
      </p>
    </div>
    <template #footer>
      <Button variant="secondary" @click="open = false">Cancel</Button>
      <Button
        variant="destructive"
        :disabled="busy || (deleteFiles && confirmName.trim() !== project.name)"
        @click="submit"
      >
        {{ deleteFiles ? 'Remove and move folder to trash' : 'Remove from Vagrant Studio' }}
      </Button>
    </template>
  </AppDialog>
</template>
