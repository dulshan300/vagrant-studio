// Mirrors `ProjectNote` and `ProjectTask` in crates/domain (camelCase serde).

export interface ProjectNote {
  id: string
  projectId: string
  title: string
  /** Plain text; always rendered as text, never as HTML. */
  body: string
  createdAt: string
  updatedAt: string
}

export interface ProjectTask {
  id: string
  projectId: string
  title: string
  done: boolean
  sortOrder: number
  createdAt: string
  updatedAt: string
  completedAt: string | null
}

/** Mirrors `TaskChanges` in crates/project-service; absent fields stay as they are. */
export interface TaskChanges {
  title?: string
  done?: boolean
}
