<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import TagInput from '@/components/ui/form/TagInput.vue'
import FolderInput from '@/components/ui/form/FolderInput.vue'
import { Button } from '@/components/ui/button'
import ProviderPicker from '@/components/providers/ProviderPicker.vue'
import { useFormErrors } from '@/composables/useFormErrors'
import { useProviderChoices } from '@/composables/useProviderChoices'
import { settingsApi } from '@/services/projects'
import { useProjectsStore } from '@/stores/projects'

const open = defineModel<boolean>('open', { required: true })
const store = useProjectsStore()
const router = useRouter()
const errors = useFormErrors()

const name = ref('')
const description = ref('')
const location = ref('')
const repositoryUrl = ref('')
const tags = ref<string[]>([])
const templateId = ref<string>('')
const provider = ref<string | null>(null)
const busy = ref(false)
const providers = useProviderChoices()

// Follow the default (settings, then first ready provider) until the user picks one.
let providerTouched = false
watch(providers.defaultProvider, (p) => {
  if (!providerTouched) provider.value = p
})
watch(provider, (p) => {
  if (p !== providers.defaultProvider.value) providerTouched = true
})

/** Mirrors slugify() in crates/domain so the preview matches the folder created. */
const folderName = computed(() => {
  const slug = name.value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 64)
    .replace(/-+$/, '')
  return slug || 'project'
})

watch(open, async (isOpen) => {
  if (!isOpen) return
  errors.clear()
  name.value = ''
  description.value = ''
  repositoryUrl.value = ''
  tags.value = []
  templateId.value = ''
  providerTouched = false
  provider.value = providers.defaultProvider.value
  try {
    await store.loadTemplates()
    location.value ||= (await settingsApi.get()).defaultProjectLocation ?? ''
  } catch (e) {
    errors.set(e)
  }
})

async function submit() {
  busy.value = true
  try {
    const details = await store.create({
      name: name.value,
      description: description.value,
      location: location.value,
      repositoryUrl: repositoryUrl.value || null,
      tags: tags.value,
      templateId: templateId.value || null,
      provider: templateId.value ? provider.value : null,
    })
    open.value = false
    await router.push({ name: 'project', params: { id: details.project.id } })
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
    title="New project"
    description="Creates a new folder for the project. Existing folders are never overwritten."
    size="lg"
  >
    <form id="create-project" class="space-y-4" @submit.prevent="submit">
      <FormField label="Name" for="cp-name" :error="errors.field.value.name">
        <TextInput id="cp-name" v-model="name" :invalid="!!errors.field.value.name" autofocus />
      </FormField>

      <FormField
        label="Location"
        for="cp-location"
        :error="errors.field.value.location"
        :hint="
          location ? `The project folder will be ${folderName} inside this folder.` : undefined
        "
      >
        <FolderInput id="cp-location" v-model="location" :invalid="!!errors.field.value.location" />
      </FormField>

      <fieldset class="space-y-2">
        <legend class="mb-1.5 text-xs font-medium">Starting point</legend>
        <label
          class="flex cursor-pointer items-start gap-3 rounded-md border p-3 has-[:checked]:border-accent has-[:checked]:bg-surface-muted"
        >
          <input v-model="templateId" type="radio" value="" class="mt-1" />
          <span>
            <span class="block font-medium">Empty folder</span>
            <span class="text-xs text-muted">Add your own Vagrantfile later.</span>
          </span>
        </label>
        <label
          v-for="t in store.templates"
          :key="t.id"
          class="flex cursor-pointer items-start gap-3 rounded-md border p-3 has-[:checked]:border-accent has-[:checked]:bg-surface-muted"
        >
          <input v-model="templateId" type="radio" :value="t.id" class="mt-1" />
          <span>
            <span class="block font-medium">{{ t.name }}</span>
            <span class="text-xs text-muted">
              {{ t.description }} Machines: {{ t.machines.join(', ') }}.
            </span>
          </span>
        </label>
        <p v-if="errors.field.value.templateId" class="text-xs text-status-failed">
          {{ errors.field.value.templateId }}
        </p>
      </fieldset>

      <fieldset v-if="templateId" class="space-y-2">
        <legend class="mb-1.5 text-xs font-medium">Provider</legend>
        <ProviderPicker
          v-model="provider"
          name="cp-provider"
          :choices="providers.choices.value"
          :checking="providers.checking.value"
        />
        <p class="text-xs text-muted">
          The Vagrantfile is generated for this provider and it becomes the environment's default.
          Change your preferred default in Settings.
        </p>
        <p v-if="errors.field.value.provider" class="text-xs text-status-failed">
          {{ errors.field.value.provider }}
        </p>
      </fieldset>

      <FormField label="Description" for="cp-description" :error="errors.field.value.description">
        <TextInput id="cp-description" v-model="description" multiline />
      </FormField>

      <div class="grid gap-4 sm:grid-cols-2">
        <FormField
          label="Repository URL"
          for="cp-repo"
          hint="Optional. Don't include tokens or passwords."
          :error="errors.field.value.repositoryUrl"
        >
          <TextInput
            id="cp-repo"
            v-model="repositoryUrl"
            :invalid="!!errors.field.value.repositoryUrl"
            placeholder="https://github.com/owner/repo.git"
          />
        </FormField>
        <FormField label="Tags" for="cp-tags" :error="errors.field.value.tags">
          <TagInput id="cp-tags" v-model="tags" :suggestions="store.tags" />
        </FormField>
      </div>

      <p v-if="errors.general.value" class="text-sm text-status-failed" role="alert">
        {{ errors.general.value }}
      </p>
    </form>

    <template #footer>
      <Button variant="secondary" @click="open = false">Cancel</Button>
      <Button
        type="submit"
        form="create-project"
        :disabled="busy || !name.trim() || !location || (!!templateId && !provider)"
      >
        {{ busy ? 'Creating…' : 'Create project' }}
      </Button>
    </template>
  </AppDialog>
</template>
