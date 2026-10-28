import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type {
  AppSettings,
  CreateProjectRequest,
  DeleteOutcome,
  DeleteProjectRequest,
  Environment,
  EnvironmentRequest,
  FolderInspection,
  ImportProjectRequest,
  PortablePreview,
  Project,
  ProjectChanges,
  ProjectDetails,
  ProjectListFilter,
  TagCount,
  Template,
} from '@/types/projects'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const projectsApi = {
  list: (filter: ProjectListFilter) => call<Project[]>('list_projects', { filter }),
  tags: () => call<TagCount[]>('list_project_tags'),
  get: (id: string) => call<ProjectDetails>('get_project', { id }),
  create: (request: CreateProjectRequest) => call<ProjectDetails>('create_project', { request }),
  inspectFolder: (path: string) => call<FolderInspection>('inspect_project_folder', { path }),
  import: (request: ImportProjectRequest) => call<ProjectDetails>('import_project', { request }),
  update: (id: string, changes: ProjectChanges) => call<Project>('update_project', { id, changes }),
  setFavorite: (id: string, favorite: boolean) =>
    call<Project>('set_project_favorite', { id, favorite }),
  setArchived: (id: string, archived: boolean) =>
    call<Project>('set_project_archived', { id, archived }),
  markOpened: (id: string) => call<void>('mark_project_opened', { id }),
  delete: (id: string, request: DeleteProjectRequest) =>
    call<DeleteOutcome>('delete_project', { id, request }),
  addEnvironment: (projectId: string, request: EnvironmentRequest) =>
    call<Environment>('add_environment', { projectId, request }),
  removeEnvironment: (id: string) => call<void>('remove_environment', { id }),
  previewMetadata: (id: string) => call<PortablePreview>('preview_project_metadata', { id }),
  writeMetadata: (id: string, overwrite: boolean) =>
    call<PortablePreview>('write_project_metadata', { id, overwrite }),
  templates: () => call<Template[]>('list_templates'),
  openFolder: (id: string) => call<void>('open_project_folder', { id }),
}

export const settingsApi = {
  get: () => call<AppSettings>('get_settings'),
  save: (settings: AppSettings) => call<AppSettings>('save_settings', { settings }),
}

/** Native folder picker. Resolves to null when cancelled. */
export async function pickFolder(defaultPath?: string | null): Promise<string | null> {
  if (!hasBackend()) throw new BackendUnavailableError()
  const result = await open({
    directory: true,
    multiple: false,
    defaultPath: defaultPath ?? undefined,
  })
  return typeof result === 'string' ? result : null
}
