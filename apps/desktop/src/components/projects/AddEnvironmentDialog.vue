<script setup lang="ts">
import { ref, watch } from 'vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import FolderInput from '@/components/ui/form/FolderInput.vue'
import { Button } from '@/components/ui/button'
import { useFormErrors } from '@/composables/useFormErrors'
import { projectsApi } from '@/services/projects'
import type { Environment, EnvironmentType } from '@/types/projects'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ projectId: string; projectRoot: string }>()
const emit = defineEmits<{ added: [environment: Environment] }>()
const errors = useFormErrors()

const name = ref('')
const folder = ref('')
const type = ref<EnvironmentType>('development')
const busy = ref(false)

watch(open, (isOpen) => {
  if (!isOpen) return
  errors.clear()
  name.value = ''
  folder.value = props.projectRoot
  type.value = 'development'
})

function onPicked(path: string) {
  if (!name.value) name.value = path.split(/[\\/]/).filter(Boolean).pop() ?? ''
}

async function submit() {
  busy.value = true
  try {
    const env = await projectsApi.addEnvironment(props.projectId, {
      name: name.value,
      relativePath: folder.value,
      environmentType: type.value,
    })
    emit('added', env)
    open.value = false
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
    title="Add environment"
    description="Registers a Vagrant working directory inside this project. Nothing is written to disk."
  >
    <form id="add-environment" class="space-y-4" @submit.prevent="submit">
      <FormField
        label="Folder"
        for="ae-folder"
        hint="Must be inside the project folder."
        :error="errors.field.value.environments"
      >
        <FolderInput
          id="ae-folder"
          v-model="folder"
          :invalid="!!errors.field.value.environments"
          @picked="onPicked"
        />
      </FormField>
      <div class="grid gap-4 sm:grid-cols-2">
        <FormField label="Name" for="ae-name" :error="errors.field.value.environmentName">
          <TextInput id="ae-name" v-model="name" :invalid="!!errors.field.value.environmentName" />
        </FormField>
        <FormField label="Type" for="ae-type">
          <select
            id="ae-type"
            v-model="type"
            class="w-full rounded-md border bg-surface px-3 py-2 text-sm"
          >
            <option value="development">Development</option>
            <option value="testing">Testing</option>
            <option value="staging">Staging</option>
            <option value="other">Other</option>
          </select>
        </FormField>
      </div>
      <p v-if="errors.general.value" class="text-sm text-status-failed" role="alert">
        {{ errors.general.value }}
      </p>
    </form>
    <template #footer>
      <Button variant="secondary" @click="open = false">Cancel</Button>
      <Button type="submit" form="add-environment" :disabled="busy || !name.trim() || !folder">
        Add environment
      </Button>
    </template>
  </AppDialog>
</template>
