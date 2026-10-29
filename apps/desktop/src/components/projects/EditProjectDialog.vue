<script setup lang="ts">
import { ref, watch } from 'vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import TagInput from '@/components/ui/form/TagInput.vue'
import { Button } from '@/components/ui/button'
import { useFormErrors } from '@/composables/useFormErrors'
import { useProjectsStore } from '@/stores/projects'
import type { Project } from '@/types/projects'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ project: Project }>()
const emit = defineEmits<{ saved: [project: Project] }>()

const store = useProjectsStore()
const errors = useFormErrors()
const form = ref({ name: '', description: '', repositoryUrl: '', tags: [] as string[] })
const saving = ref(false)

// Start from the saved values each time the dialog opens.
watch(open, (isOpen) => {
  if (!isOpen) return
  const p = props.project
  form.value = {
    name: p.name,
    description: p.description,
    repositoryUrl: p.repositoryUrl ?? '',
    tags: [...p.tags],
  }
  errors.clear()
})

async function save() {
  saving.value = true
  errors.clear()
  try {
    const updated = await store.update(props.project.id, {
      name: form.value.name,
      description: form.value.description,
      repositoryUrl: form.value.repositoryUrl || null,
      tags: form.value.tags,
    })
    emit('saved', updated)
    open.value = false
  } catch (e) {
    errors.set(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <AppDialog v-model:open="open" title="Edit details">
    <form id="edit-project-form" class="space-y-4" @submit.prevent="save">
      <FormField label="Name" for="pp-name" :error="errors.field.value.name">
        <TextInput id="pp-name" v-model="form.name" :invalid="!!errors.field.value.name" />
      </FormField>
      <FormField label="Description" for="pp-description" :error="errors.field.value.description">
        <TextInput id="pp-description" v-model="form.description" multiline />
      </FormField>
      <FormField label="Repository URL" for="pp-repo" :error="errors.field.value.repositoryUrl">
        <TextInput
          id="pp-repo"
          v-model="form.repositoryUrl"
          :invalid="!!errors.field.value.repositoryUrl"
        />
      </FormField>
      <FormField label="Tags" for="pp-tags" :error="errors.field.value.tags">
        <TagInput id="pp-tags" v-model="form.tags" :suggestions="store.tags" />
      </FormField>
      <p v-if="errors.general.value" class="text-sm text-status-failed" role="alert">
        {{ errors.general.value }}
      </p>
    </form>
    <template #footer>
      <Button variant="ghost" @click="open = false">Cancel</Button>
      <Button type="submit" form="edit-project-form" :disabled="saving">
        {{ saving ? 'Saving…' : 'Save' }}
      </Button>
    </template>
  </AppDialog>
</template>
