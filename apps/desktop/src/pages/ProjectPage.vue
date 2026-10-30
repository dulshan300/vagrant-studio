<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Archive,
  ArchiveRestore,
  ChevronLeft,
  Code,
  Clock,
  Ellipsis,
  FileJson,
  Folder,
  FolderOpen,
  Layers,
  Link,
  Pencil,
  Plus,
  Star,
  Trash2,
} from '@lucide/vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import AddEnvironmentDialog from '@/components/projects/AddEnvironmentDialog.vue'
import EditProjectDialog from '@/components/projects/EditProjectDialog.vue'
import GitPanel from '@/components/projects/GitPanel.vue'
import GitStatusBadge from '@/components/projects/GitStatusBadge.vue'
import EnvironmentPanel from '@/components/projects/EnvironmentPanel.vue'
import DeleteProjectDialog from '@/components/projects/DeleteProjectDialog.vue'
import PortableMetadataDialog from '@/components/projects/PortableMetadataDialog.vue'
import ProjectNotes from '@/components/projects/ProjectNotes.vue'
import ProjectTasks from '@/components/projects/ProjectTasks.vue'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { relativeTime } from '@/lib/projects'
import { cn } from '@/lib/utils'
import { editorApi } from '@/services/editor'
import { gitApi } from '@/services/git'
import { networkApi } from '@/services/network'
import { errorMessage } from '@/services/errors'
import { projectsApi } from '@/services/projects'
import { useProjectsStore } from '@/stores/projects'
import type { GitStatus } from '@/types/git'
import type { PortUse } from '@/types/vagrantfile'
import type { Project, ProjectDetails } from '@/types/projects'

const route = useRoute()
const router = useRouter()
const store = useProjectsStore()

const details = ref<ProjectDetails | null>(null)
const loadError = ref<string | null>(null)
const actionError = ref<string | null>(null)
const showDelete = ref(false)
const showMetadata = ref(false)
const showAddEnvironment = ref(false)
const showEdit = ref(false)

const project = computed(() => details.value?.project ?? null)

/** Forwarded ports of every environment, for cross-project port warnings. */
const forwardedPorts = ref<PortUse[]>([])

async function load(id: string) {
  void networkApi
    .listForwardedPorts()
    .then((r) => (forwardedPorts.value = r.ports))
    .catch(() => undefined)
  try {
    details.value = await projectsApi.get(id)
    loadError.value = null
    void store.markOpened(id).catch(() => undefined)
  } catch (e) {
    details.value = null
    loadError.value = errorMessage(e)
  }
}

watch(() => route.params.id as string, load, { immediate: true })

// Git status is shared by the header badge and the version-control panel.
const gitStatus = ref<GitStatus | null>(null)
const gitLoading = ref(false)
const gitPanel = ref<HTMLElement | null>(null)

async function loadGit() {
  const id = route.params.id as string
  gitLoading.value = true
  try {
    gitStatus.value = await gitApi.status(id)
  } catch (e) {
    gitStatus.value = { state: 'error', message: errorMessage(e) }
  } finally {
    gitLoading.value = false
  }
}
watch(() => route.params.id as string, loadGit, { immediate: true })

async function showGitPanel() {
  await nextTick()
  gitPanel.value?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

// Set by the New project dialog when creating the repository failed.
watch(
  () => route.query.gitError,
  (message) => {
    if (typeof message !== 'string') return
    actionError.value = `The project was created, but Git wasn't initialized: ${message}`
    void router.replace({ query: {} })
  },
  { immediate: true },
)

function saved(updated: Project) {
  details.value = { ...details.value!, project: updated }
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

async function openInEditor() {
  actionError.value = null
  try {
    await editorApi.open(project.value!.id)
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}

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
    <header class="flex flex-wrap items-start justify-between gap-4 pb-6 lg:flex-nowrap">
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1">
          <h1 class="truncate text-xl font-semibold tracking-tight">{{ project.name }}</h1>
          <Button
            variant="ghost"
            size="icon"
            class="size-8"
            :aria-label="project.favorite ? 'Remove from favorites' : 'Add to favorites'"
            :aria-pressed="project.favorite"
            @click="toggleFavorite"
          >
            <Star
              :class="cn(project.favorite && 'fill-status-transition text-status-transition')"
            />
          </Button>
        </div>
        <p v-if="project.description" class="mt-1 text-sm text-muted">
          {{ project.description }}
        </p>
        <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-muted">
          <span class="flex min-w-0 items-center gap-1.5" :title="project.rootPath">
            <Folder class="size-3.5 shrink-0" aria-hidden="true" />
            <code class="truncate font-mono">{{ project.rootPath }}</code>
          </span>
          <GitStatusBadge :status="gitStatus" :loading="gitLoading" @open="showGitPanel" />
          <span
            v-if="project.repositoryUrl"
            class="flex min-w-0 items-center gap-1.5"
            :title="project.repositoryUrl"
          >
            <Link class="size-3.5 shrink-0" aria-hidden="true" />
            <span class="truncate">{{ project.repositoryUrl }}</span>
          </span>
          <span
            class="flex items-center gap-1.5"
            :title="`Added ${relativeTime(project.createdAt)}`"
          >
            <Clock class="size-3.5 shrink-0" aria-hidden="true" />
            Updated {{ relativeTime(project.updatedAt) }}
          </span>
          <span v-if="project.tags.length" class="flex flex-wrap items-center gap-1">
            <span
              v-for="tag in project.tags"
              :key="tag"
              class="rounded-full bg-surface-muted px-2 py-0.5 text-[11px] text-foreground"
              >{{ tag }}</span
            >
          </span>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-2">
        <Button variant="secondary" size="sm" @click="openFolder">
          <FolderOpen /> Open folder
        </Button>
        <Button
          variant="secondary"
          size="sm"
          title="Open the project folder in your code editor (choose it in Settings)"
          @click="openInEditor"
        >
          <Code /> Open in editor
        </Button>
        <Button variant="secondary" size="sm" @click="showEdit = true">
          <Pencil /> Edit details
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              variant="secondary"
              size="icon"
              class="size-8"
              aria-label="More project actions"
            >
              <Ellipsis />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuItem @select="showMetadata = true">
              <FileJson /> Portable file…
            </DropdownMenuItem>
            <DropdownMenuItem @select="toggleArchived">
              <template v-if="project.status === 'archived'"><ArchiveRestore /> Restore</template>
              <template v-else><Archive /> Archive</template>
            </DropdownMenuItem>
            <DropdownMenuSeparator class="my-1 h-px bg-border" />
            <DropdownMenuItem destructive @select="showDelete = true">
              <Trash2 /> Remove project…
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>

    <p
      v-if="project.status === 'archived'"
      class="mb-4 flex items-center gap-2 rounded-md bg-surface-muted px-3 py-2 text-sm"
    >
      <Archive class="size-4" aria-hidden="true" /> This project is archived. Restore it to show it
      in the main list again.
    </p>
    <p v-if="actionError" class="mb-4 text-sm text-status-failed" role="alert">{{ actionError }}</p>

    <section aria-labelledby="environments-heading">
      <div class="mb-3 flex items-center justify-between gap-4">
        <h2 id="environments-heading" class="flex items-center gap-2 text-sm font-semibold">
          Environments
          <span
            class="rounded-full bg-surface-muted px-2 py-0.5 text-[11px] font-medium text-muted"
          >
            {{ details.environments.length }}
          </span>
        </h2>
        <Button
          v-if="details.environments.length"
          variant="secondary"
          size="sm"
          @click="showAddEnvironment = true"
        >
          <Plus /> Add environment
        </Button>
      </div>
      <ul v-if="details.environments.length" class="space-y-4">
        <EnvironmentPanel
          v-for="env in details.environments"
          :key="env.id"
          :environment="env"
          :forwarded-ports="forwardedPorts"
          @remove="removeEnvironment(env.id, env.name)"
          @updated="load(project.id)"
        />
      </ul>
      <Card v-else>
        <EmptyState
          :icon="Layers"
          title="No environments"
          description="Register a folder containing a Vagrantfile to manage its machines."
        >
          <Button variant="secondary" @click="showAddEnvironment = true">Add environment</Button>
        </EmptyState>
      </Card>
    </section>

    <div ref="gitPanel" class="mt-6 scroll-mt-4">
      <GitPanel
        :project-id="project.id"
        :project-root="project.rootPath"
        :status="gitStatus"
        :loading="gitLoading"
        @update="gitStatus = $event"
        @refresh="loadGit"
      />
    </div>

    <div class="mt-4 grid items-start gap-4 lg:grid-cols-2">
      <ProjectTasks :project-id="project.id" />
      <ProjectNotes :project-id="project.id" />
    </div>

    <EditProjectDialog v-model:open="showEdit" :project="project" @saved="saved" />
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
