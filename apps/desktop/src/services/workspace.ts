import { invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type { ProjectNote, ProjectTask, TaskChanges } from '@/types/workspace'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const notesApi = {
  list: (projectId: string) => call<ProjectNote[]>('list_project_notes', { projectId }),
  create: (projectId: string, title: string, body: string) =>
    call<ProjectNote>('create_project_note', { projectId, title, body }),
  update: (projectId: string, id: string, title: string, body: string) =>
    call<ProjectNote>('update_project_note', { projectId, id, title, body }),
  remove: (projectId: string, id: string) => call<void>('delete_project_note', { projectId, id }),
}

export const tasksApi = {
  list: (projectId: string) => call<ProjectTask[]>('list_project_tasks', { projectId }),
  create: (projectId: string, title: string) =>
    call<ProjectTask>('create_project_task', { projectId, title }),
  update: (projectId: string, id: string, changes: TaskChanges) =>
    call<ProjectTask>('update_project_task', { projectId, id, changes }),
  remove: (projectId: string, id: string) => call<void>('delete_project_task', { projectId, id }),
  /** `ids` lists every task of the project once, in the new order. */
  reorder: (projectId: string, ids: string[]) =>
    call<ProjectTask[]>('reorder_project_tasks', { projectId, ids }),
}
