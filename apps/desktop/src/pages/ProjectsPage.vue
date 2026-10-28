<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { FolderInput, FolderKanban, Plus, Search, SearchX } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import ProjectCard from '@/components/projects/ProjectCard.vue'
import CreateProjectDialog from '@/components/projects/CreateProjectDialog.vue'
import ImportProjectDialog from '@/components/projects/ImportProjectDialog.vue'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { queryProjects, type ProjectSort } from '@/lib/projects'
import { errorMessage } from '@/services/errors'
import { useProjectsStore } from '@/stores/projects'
import { cn } from '@/lib/utils'

const store = useProjectsStore()
onMounted(() => void store.load())

const view = ref<'active' | 'favorites' | 'archived'>('active')
const search = ref('')
const selectedTags = ref<string[]>([])
const sort = ref<ProjectSort>('recent')
const showCreate = ref(false)
const showImport = ref(false)
const actionError = ref<string | null>(null)

const visible = computed(() =>
  queryProjects(view.value === 'archived' ? store.archived : store.active, {
    search: search.value,
    favoritesOnly: view.value === 'favorites',
    tags: selectedTags.value,
    sort: sort.value,
  }),
)

const filtering = computed(() => search.value.trim() !== '' || selectedTags.value.length > 0)

function toggleTag(tag: string) {
  selectedTags.value = selectedTags.value.includes(tag)
    ? selectedTags.value.filter((t) => t !== tag)
    : [...selectedTags.value, tag]
}

async function toggleFavorite(id: string, favorite: boolean) {
  try {
    actionError.value = null
    await store.setFavorite(id, !favorite)
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}

const views = [
  { value: 'active', label: 'All' },
  { value: 'favorites', label: 'Favorites' },
  { value: 'archived', label: 'Archived' },
] as const
</script>

<template>
  <PageHeader title="Projects" description="Organize Vagrant environments into projects.">
    <template #actions>
      <Button variant="secondary" @click="showImport = true">
        <FolderInput aria-hidden="true" /> Import
      </Button>
      <Button @click="showCreate = true"><Plus aria-hidden="true" /> New project</Button>
    </template>
  </PageHeader>

  <div v-if="store.projects.length" class="mb-5 space-y-3">
    <div class="flex flex-wrap items-center gap-3">
      <div
        role="tablist"
        aria-label="Project view"
        class="inline-flex rounded-lg border bg-surface-muted p-1"
      >
        <button
          v-for="v in views"
          :key="v.value"
          role="tab"
          :aria-selected="view === v.value"
          :class="
            cn(
              'rounded-md px-3 py-1 text-sm text-muted hover:text-foreground',
              view === v.value && 'bg-surface text-foreground shadow-xs',
            )
          "
          @click="view = v.value"
        >
          {{ v.label }}
          <span v-if="v.value === 'archived' && store.archived.length" class="text-xs">
            ({{ store.archived.length }})
          </span>
        </button>
      </div>

      <div class="relative min-w-48 flex-1">
        <Search
          class="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted"
          aria-hidden="true"
        />
        <input
          v-model="search"
          type="search"
          placeholder="Search name, description, path or tag"
          aria-label="Search projects"
          class="w-full rounded-md border bg-surface py-1.5 pr-3 pl-8 text-sm"
        />
      </div>

      <label class="flex items-center gap-2 text-xs text-muted">
        Sort
        <select
          v-model="sort"
          class="rounded-md border bg-surface px-2 py-1.5 text-sm text-foreground"
        >
          <option value="recent">Recently opened</option>
          <option value="name">Name</option>
          <option value="created">Newest</option>
        </select>
      </label>
    </div>

    <div
      v-if="store.tags.length"
      class="flex flex-wrap items-center gap-1.5"
      aria-label="Filter by tag"
    >
      <button
        v-for="tag in store.tags"
        :key="tag"
        type="button"
        :aria-pressed="selectedTags.includes(tag)"
        :class="
          cn(
            'rounded-full border px-2.5 py-0.5 text-xs text-muted hover:text-foreground',
            selectedTags.includes(tag) && 'border-accent bg-accent/10 text-foreground',
          )
        "
        @click="toggleTag(tag)"
      >
        {{ tag }}
      </button>
    </div>
  </div>

  <p v-if="actionError" class="mb-4 text-sm text-status-failed" role="alert">{{ actionError }}</p>

  <Card v-if="store.error">
    <EmptyState
      :icon="FolderKanban"
      title="Projects couldn't be loaded"
      :description="store.error"
    />
  </Card>

  <Card v-else-if="store.loaded && !store.projects.length">
    <EmptyState
      :icon="FolderKanban"
      title="No projects yet"
      description="Create a project from a template or import an existing folder with a Vagrantfile. Importing never changes your files or starts machines."
    >
      <div class="flex gap-2">
        <Button variant="secondary" @click="showImport = true">Import existing folder</Button>
        <Button @click="showCreate = true">New project</Button>
      </div>
    </EmptyState>
  </Card>

  <Card v-else-if="store.loaded && !visible.length">
    <EmptyState
      :icon="SearchX"
      :title="
        filtering
          ? 'No matching projects'
          : view === 'archived'
            ? 'No archived projects'
            : 'No favorite projects'
      "
      :description="
        filtering
          ? 'Try a different search or clear the tag filters.'
          : view === 'archived'
            ? 'Archived projects are hidden from the main list but keep all their data.'
            : 'Star a project to pin it here.'
      "
    />
  </Card>

  <div v-else class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
    <ProjectCard
      v-for="project in visible"
      :key="project.id"
      :project="project"
      @toggle-favorite="toggleFavorite(project.id, project.favorite)"
    />
  </div>

  <CreateProjectDialog v-model:open="showCreate" />
  <ImportProjectDialog v-model:open="showImport" />
</template>
