<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { CircleAlert, FileCheck, FolderSearch, Info } from '@lucide/vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import TagInput from '@/components/ui/form/TagInput.vue'
import FolderInput from '@/components/ui/form/FolderInput.vue'
import { Button } from '@/components/ui/button'
import ProviderSelect from '@/components/providers/ProviderSelect.vue'
import { useFormErrors } from '@/composables/useFormErrors'
import { useProviderChoices } from '@/composables/useProviderChoices'
import { projectsApi } from '@/services/projects'
import { useProjectsStore } from '@/stores/projects'
import type { DetectedEnvironment, FolderInspection } from '@/types/projects'

const open = defineModel<boolean>('open', { required: true })
const store = useProjectsStore()
const router = useRouter()
const errors = useFormErrors()

const folder = ref('')
const inspection = ref<FolderInspection | null>(null)
const inspecting = ref(false)
const name = ref('')
const description = ref('')
const repositoryUrl = ref('')
const tags = ref<string[]>([])
const selected = ref<DetectedEnvironment[]>([])
const provider = ref<string | null>(null)
const busy = ref(false)
const providers = useProviderChoices()

watch(open, (isOpen) => {
  if (!isOpen) return
  errors.clear()
  folder.value = ''
  inspection.value = null
  provider.value = providers.defaultProvider.value
})

async function inspect() {
  if (!folder.value) return
  inspecting.value = true
  errors.clear()
  try {
    const result = await projectsApi.inspectFolder(folder.value)
    inspection.value = result
    name.value = result.suggestedName
    description.value = result.suggestedDescription
    repositoryUrl.value = result.suggestedRepositoryUrl ?? ''
    tags.value = [...result.suggestedTags]
    selected.value = [...result.environments]
  } catch (e) {
    inspection.value = null
    errors.set(e)
  } finally {
    inspecting.value = false
  }
}

async function submit() {
  if (!inspection.value) return
  busy.value = true
  try {
    const details = await store.importProject({
      rootPath: inspection.value.path,
      name: name.value,
      description: description.value,
      repositoryUrl: repositoryUrl.value || null,
      tags: tags.value,
      environments: selected.value.map((e) => ({
        name: e.name,
        relativePath: e.relativePath,
        environmentType: null,
        defaultProvider: provider.value,
      })),
    })
    open.value = false
    await router.push({ name: 'project', params: { id: details.project.id } })
  } catch (e) {
    errors.set(e)
  } finally {
    busy.value = false
  }
}

function openExisting() {
  if (!inspection.value?.existingProjectId) return
  open.value = false
  void router.push({ name: 'project', params: { id: inspection.value.existingProjectId } })
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    title="Import existing project"
    description="Registers a folder you already have. Importing never changes its files or starts machines."
    size="lg"
  >
    <div class="space-y-4">
      <FormField label="Project folder" for="ip-folder" :error="errors.field.value.rootPath">
        <div class="flex gap-2">
          <div class="flex-1">
            <FolderInput id="ip-folder" v-model="folder" @picked="inspect" />
          </div>
          <Button variant="secondary" :disabled="!folder || inspecting" @click="inspect">
            <FolderSearch aria-hidden="true" />
            {{ inspecting ? 'Scanning…' : 'Scan' }}
          </Button>
        </div>
      </FormField>

      <template v-if="inspection">
        <div
          v-if="inspection.existingProjectId"
          class="flex items-start gap-2 rounded-md bg-surface-muted p-3 text-sm"
        >
          <Info class="mt-0.5 size-4 shrink-0 text-accent" aria-hidden="true" />
          <span>
            This folder is already a project.
            <button class="text-accent underline-offset-2 hover:underline" @click="openExisting">
              Open it
            </button>
          </span>
        </div>

        <template v-else>
          <p
            v-if="inspection.hasPortableMetadata && !inspection.portableError"
            class="flex items-center gap-2 text-xs text-muted"
          >
            <FileCheck class="size-4 text-status-running" aria-hidden="true" />
            Details below come from the folder's .vagrant-studio/project.json.
          </p>
          <p
            v-if="inspection.portableError"
            class="flex items-start gap-2 rounded-md bg-surface-muted p-3 text-xs"
          >
            <CircleAlert class="mt-0.5 size-4 shrink-0 text-status-transition" aria-hidden="true" />
            project.json was ignored: {{ inspection.portableError }}
          </p>

          <FormField label="Name" for="ip-name" :error="errors.field.value.name">
            <TextInput id="ip-name" v-model="name" :invalid="!!errors.field.value.name" />
          </FormField>
          <FormField label="Description" for="ip-description">
            <TextInput id="ip-description" v-model="description" multiline />
          </FormField>
          <div class="grid gap-4 sm:grid-cols-2">
            <FormField
              label="Repository URL"
              for="ip-repo"
              :error="errors.field.value.repositoryUrl"
            >
              <TextInput
                id="ip-repo"
                v-model="repositoryUrl"
                :invalid="!!errors.field.value.repositoryUrl"
              />
            </FormField>
            <FormField label="Tags" for="ip-tags" :error="errors.field.value.tags">
              <TagInput id="ip-tags" v-model="tags" :suggestions="store.tags" />
            </FormField>
          </div>

          <fieldset>
            <legend class="mb-1.5 text-xs font-medium">Vagrant environments found</legend>
            <p v-if="!inspection.environments.length" class="text-xs text-muted">
              No Vagrantfile found. You can still import the folder and add environments later.
            </p>
            <ul v-else class="divide-y rounded-md border">
              <li v-for="env in inspection.environments" :key="env.relativePath">
                <label class="flex cursor-pointer items-start gap-3 px-3 py-2">
                  <input v-model="selected" type="checkbox" :value="env" class="mt-1" />
                  <span class="min-w-0">
                    <span class="font-medium">{{ env.name }}</span>
                    <code class="ml-2 font-mono text-xs text-muted">{{ env.relativePath }}</code>
                    <span class="block text-xs text-muted">
                      {{ env.vagrantfile ?? 'No Vagrantfile in this folder' }}
                      <template v-if="env.hasVagrantState"> · machines may already exist</template>
                    </span>
                  </span>
                </label>
              </li>
            </ul>
            <div
              v-if="inspection.environments.length"
              class="mt-3 flex flex-wrap items-center gap-2 text-xs"
            >
              <label for="ip-provider" class="font-medium">Provider for these environments</label>
              <ProviderSelect
                id="ip-provider"
                v-model="provider"
                :choices="providers.choices.value"
                allow-unset
              />
            </div>
            <p v-if="inspection.scanTruncated" class="mt-1.5 text-xs text-muted">
              The folder is large, so only part of it was scanned.
            </p>
            <p v-if="errors.field.value.environments" class="mt-1.5 text-xs text-status-failed">
              {{ errors.field.value.environments }}
            </p>
          </fieldset>
        </template>
      </template>

      <p v-if="errors.general.value" class="text-sm text-status-failed" role="alert">
        {{ errors.general.value }}
      </p>
    </div>

    <template #footer>
      <Button variant="secondary" @click="open = false">Cancel</Button>
      <Button
        :disabled="busy || !inspection || !!inspection.existingProjectId || !name.trim()"
        @click="submit"
      >
        {{ busy ? 'Importing…' : 'Import project' }}
      </Button>
    </template>
  </AppDialog>
</template>
