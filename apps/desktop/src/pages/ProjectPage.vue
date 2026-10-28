<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  Archive,
  ArchiveRestore,
  ChevronLeft,
  FileJson,
  FolderOpen,
  Layers,
  Plus,
  Star,
  Trash2,
} from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import AddEnvironmentDialog from '@/components/projects/AddEnvironmentDialog.vue'
import DeleteProjectDialog from '@/components/projects/DeleteProjectDialog.vue'
import PortableMetadataDialog from '@/components/projects/PortableMetadataDialog.vue'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import TagInput from '@/components/ui/form/TagInput.vue'
import { Button } from '@/components/ui/button'
import { Card, CardHeader } from '@/components/ui/card'
import { useFormErrors } from '@/composables/useFormErrors'
import { relativeTime } from '@/lib/projects'
import { cn } from '@/lib/utils'
import { errorMessage } from '@/services/errors'
import { projectsApi } from '@/services/projects'
import { useProjectsStore } from '@/stores/projects'
import type { ProjectDetails } from '@/types/projects'

const route = useRoute()
const store = useProjectsStore()
const errors = useFormErrors()

const details = ref<ProjectDetails | null>(null)
const loadError = ref<string | null>(null)
const actionError = ref<string | null>(null)
const showDelete = ref(false)
const showMetadata = ref(false)
const showAddEnvironment = ref(false)

const form = ref({ name: '', description: '', repositoryUrl: '', tags: [] as string[] })
const saving = ref(false)
const savedAt = ref<number | null>(null)

const project = computed(() => details.value?.project ?? null)
const dirty = computed(() => {
  const p = project.value
  if (!p) return false
  return (
    form.value.name !== p.name ||
    form.value.description !== p.description ||
    form.value.repositoryUrl !== (p.repositoryUrl ?? '') ||
    form.value.tags.join() !== p.tags.join()
  )
})

function resetForm() {
  const p = project.value
  if (!p) return
  form.value = {
    name: p.name,
    description: p.description,
    repositoryUrl: p.repositoryUrl ?? '',
    tags: [...p.tags],
  }
}

async function load(id: string) {
  try {
    details.value = await projectsApi.get(id)
    loadError.value = null
    resetForm()
    void store.markOpened(id).catch(() => undefined)
  } catch (e) {
    details.value = null
    loadError.value = errorMessage(e)
  }
}

watch(() => route.params.id as string, load, { immediate: true })

async function save() {
  if (!project.value) return
  saving.value = true
  errors.clear()
  try {
    const updated = await store.update(project.value.id, {
      name: form.value.name,
      description: form.value.description,
      repositoryUrl: form.value.repositoryUrl || null,
      tags: form.value.tags,
    })
    details.value = { ...details.value!, project: updated }
    resetForm()
    savedAt.value = Date.now()
  } catch (e) {
    errors.set(e)
  } finally {
    saving.value = false
  }
}

async function run(action: () => Promise<void>) {
  actionError.value = null
  try {
    await action()
    if (project.value) await load(project.value.id)
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}

const toggleFavorite = () =>
  run(() => store.setFavorite(project.value!.id, !project.value!.favorite))
const toggleArchived = () =>
  run(() => store.setArchived(project.value!.id, project.value!.status !== 'archived'))
const openFolder = () => run(() => projectsApi.openFolder(project.value!.id))

async function removeEnvironment(id: string, name: string) {
  if (
    !window.confirm(
      `Remove environment “${name}” from this project? Its folder and any machines are not affected.`,
    )
  )
    return
  await run(() => projectsApi.removeEnvironment(id))
}
</script>

<template>
  <RouterLink
    :to="{ name: 'projects' }"
    class="mb-4 inline-flex items-center gap-1 text-xs text-muted hover:text-foreground"
  >
    <ChevronLeft class="size-3.5" aria-hidden="true" /> Projects
  </RouterLink>

  <Card v-if="loadError">
    <EmptyState :icon="Layers" title="Project unavailable" :description="loadError" />
  </Card>

  <template v-else-if="project && details">
    <PageHeader :title="project.name" :description="project.description || undefined">
      <template #actions>
        <Button
          variant="ghost"
          size="icon"
          :aria-label="project.favorite ? 'Remove from favorites' : 'Add to favorites'"
          :aria-pressed="project.favorite"
          @click="toggleFavorite"
        >
          <Star :class="cn(project.favorite && 'fill-status-transition text-status-transition')" />
        </Button>
        <Button variant="secondary" @click="openFolder"><FolderOpen /> Open folder</Button>
      </template>
    </PageHeader>

    <p
      v-if="project.status === 'archived'"
      class="mb-4 flex items-center gap-2 rounded-md bg-surface-muted px-3 py-2 text-sm"
    >
      <Archive class="size-4" aria-hidden="true" /> This project is archived. Restore it to show it
      in the main list again.
    </p>
    <p v-if="actionError" class="mb-4 text-sm text-status-failed" role="alert">{{ actionError }}</p>

    <div class="grid gap-5 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
      <Card>
        <CardHeader title="Environments" description="Vagrant working directories in this project.">
          <template #actions>
            <Button variant="secondary" size="sm" @click="showAddEnvironment = true">
              <Plus /> Add
            </Button>
          </template>
        </CardHeader>
        <ul v-if="details.environments.length" class="divide-y">
          <li v-for="env in details.environments" :key="env.id" class="px-5 py-3">
            <div class="flex items-center justify-between gap-3">
              <span class="font-medium">{{ env.name }}</span>
              <span class="flex items-center gap-2">
                <span class="rounded bg-surface-muted px-1.5 py-0.5 text-[11px] capitalize">{{
                  env.environmentType
                }}</span>
                <Button
                  variant="ghost"
                  size="icon"
                  class="size-7 text-muted"
                  :aria-label="`Remove environment ${env.name}`"
                  @click="removeEnvironment(env.id, env.name)"
                >
                  <Trash2 />
                </Button>
              </span>
            </div>
            <code
              class="mt-0.5 block truncate font-mono text-xs text-muted"
              :title="env.rootPath"
              >{{ env.rootPath }}</code
            >
            <p class="mt-0.5 text-xs text-muted">
              {{
                env.vagrantfilePath
                  ? `Vagrantfile: ${env.vagrantfilePath}`
                  : 'No Vagrantfile detected'
              }}
            </p>
          </li>
        </ul>
        <EmptyState
          v-else
          :icon="Layers"
          title="No environments"
          description="Register a folder containing a Vagrantfile to manage its machines."
        >
          <Button variant="secondary" @click="showAddEnvironment = true">Add environment</Button>
        </EmptyState>
      </Card>

      <Card>
        <CardHeader title="Details" />
        <form class="space-y-4 px-5 py-4" @submit.prevent="save">
          <FormField label="Name" for="pp-name" :error="errors.field.value.name">
            <TextInput id="pp-name" v-model="form.name" :invalid="!!errors.field.value.name" />
          </FormField>
          <FormField
            label="Description"
            for="pp-description"
            :error="errors.field.value.description"
          >
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
          <div class="flex items-center justify-end gap-2">
            <span v-if="savedAt && !dirty" class="mr-auto text-xs text-muted">Saved</span>
            <Button v-if="dirty" type="button" variant="ghost" @click="resetForm">Discard</Button>
            <Button type="submit" :disabled="!dirty || saving">{{
              saving ? 'Saving…' : 'Save'
            }}</Button>
          </div>
        </form>

        <dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 border-t px-5 py-4 text-xs">
          <dt class="text-muted">Folder</dt>
          <dd>
            <code class="font-mono break-all">{{ project.rootPath }}</code>
          </dd>
          <dt class="text-muted">Added</dt>
          <dd>{{ relativeTime(project.createdAt) }}</dd>
          <dt class="text-muted">Updated</dt>
          <dd>{{ relativeTime(project.updatedAt) }}</dd>
        </dl>

        <div class="flex flex-wrap gap-2 border-t px-5 py-4">
          <Button variant="secondary" size="sm" @click="showMetadata = true">
            <FileJson /> Portable file…
          </Button>
          <Button variant="secondary" size="sm" @click="toggleArchived">
            <template v-if="project.status === 'archived'"><ArchiveRestore /> Restore</template>
            <template v-else><Archive /> Archive</template>
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="ml-auto text-status-failed"
            @click="showDelete = true"
          >
            <Trash2 /> Remove…
          </Button>
        </div>
      </Card>
    </div>

    <DeleteProjectDialog v-model:open="showDelete" :project="project" />
    <PortableMetadataDialog v-model:open="showMetadata" :project-id="project.id" />
    <AddEnvironmentDialog
      v-model:open="showAddEnvironment"
      :project-id="project.id"
      :project-root="project.rootPath"
      @added="load(project.id)"
    />
  </template>

  <p v-else class="text-muted">Loading…</p>
</template>
