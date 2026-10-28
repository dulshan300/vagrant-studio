import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { projectsApi } from '@/services/projects'
import { errorMessage } from '@/services/errors'
import type {
  CreateProjectRequest,
  DeleteProjectRequest,
  ImportProjectRequest,
  Project,
  ProjectChanges,
  ProjectDetails,
  Template,
} from '@/types/projects'

/** Cache of project metadata for list views. The backend database is the record. */
export const useProjectsStore = defineStore('projects', () => {
  const projects = ref<Project[]>([])
  const templates = ref<Template[]>([])
  const loaded = ref(false)
  const error = ref<string | null>(null)

  const active = computed(() => projects.value.filter((p) => p.status === 'active'))
  const archived = computed(() => projects.value.filter((p) => p.status === 'archived'))
  const tags = computed(() => [...new Set(active.value.flatMap((p) => p.tags))].sort())

  async function load() {
    try {
      projects.value = await projectsApi.list('all')
      error.value = null
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loaded.value = true
    }
  }

  async function loadTemplates() {
    if (!templates.value.length) templates.value = await projectsApi.templates()
    return templates.value
  }

  function upsert(project: Project) {
    const index = projects.value.findIndex((p) => p.id === project.id)
    if (index === -1) projects.value.push(project)
    else projects.value.splice(index, 1, project)
  }

  async function create(request: CreateProjectRequest): Promise<ProjectDetails> {
    const details = await projectsApi.create(request)
    upsert(details.project)
    return details
  }

  async function importProject(request: ImportProjectRequest): Promise<ProjectDetails> {
    const details = await projectsApi.import(request)
    upsert(details.project)
    return details
  }

  async function update(id: string, changes: ProjectChanges) {
    const project = await projectsApi.update(id, changes)
    upsert(project)
    return project
  }

  async function setFavorite(id: string, favorite: boolean) {
    upsert(await projectsApi.setFavorite(id, favorite))
  }

  async function setArchived(id: string, archived: boolean) {
    upsert(await projectsApi.setArchived(id, archived))
  }

  async function remove(id: string, request: DeleteProjectRequest) {
    const outcome = await projectsApi.delete(id, request)
    projects.value = projects.value.filter((p) => p.id !== id)
    return outcome
  }

  async function markOpened(id: string) {
    await projectsApi.markOpened(id)
    const project = projects.value.find((p) => p.id === id)
    if (project) project.lastOpenedAt = new Date().toISOString()
  }

  return {
    projects,
    templates,
    loaded,
    error,
    active,
    archived,
    tags,
    load,
    loadTemplates,
    upsert,
    create,
    importProject,
    update,
    setFavorite,
    setArchived,
    remove,
    markOpened,
  }
})
