// Mirrors the camelCase serde DTOs in crates/domain and crates/project-service.

export type ProjectStatus = 'active' | 'archived'
export type EnvironmentType = 'development' | 'testing' | 'staging' | 'other'
export type ProjectListFilter = 'active' | 'archived' | 'all'

export interface Project {
  id: string
  name: string
  slug: string
  description: string
  rootPath: string
  repositoryUrl: string | null
  status: ProjectStatus
  favorite: boolean
  tags: string[]
  createdAt: string
  updatedAt: string
  archivedAt: string | null
  lastOpenedAt: string | null
  environmentCount: number
}

export interface Environment {
  id: string
  projectId: string
  name: string
  rootPath: string
  vagrantfilePath: string | null
  defaultProvider: string | null
  environmentType: EnvironmentType
  createdAt: string
  updatedAt: string
}

export interface ProjectDetails {
  project: Project
  environments: Environment[]
}

export interface TagCount {
  name: string
  count: number
}

export interface Template {
  id: string
  name: string
  description: string
  machines: string[]
}

export interface CreateProjectRequest {
  name: string
  description: string
  location: string
  repositoryUrl: string | null
  tags: string[]
  templateId: string | null
}

export interface EnvironmentRequest {
  name: string
  relativePath: string
  environmentType: EnvironmentType | null
}

export interface ImportProjectRequest {
  rootPath: string
  name: string
  description: string
  repositoryUrl: string | null
  tags: string[]
  environments: EnvironmentRequest[]
}

export interface DetectedEnvironment {
  name: string
  relativePath: string
  vagrantfile: string | null
  hasVagrantState: boolean
  fromMetadata: boolean
}

export interface FolderInspection {
  path: string
  suggestedName: string
  suggestedDescription: string
  suggestedRepositoryUrl: string | null
  suggestedTags: string[]
  environments: DetectedEnvironment[]
  hasPortableMetadata: boolean
  portableError: string | null
  existingProjectId: string | null
  scanTruncated: boolean
}

/** Only the fields present are changed; `repositoryUrl: null` clears it. */
export interface ProjectChanges {
  name?: string
  description?: string
  repositoryUrl?: string | null
  tags?: string[]
}

export interface DeleteProjectRequest {
  deleteFiles: boolean
  confirmName: string | null
}

export interface DeleteOutcome {
  filesMovedToTrash: boolean
}

export interface PortablePreview {
  path: string
  content: string
  existing: string | null
}

export interface AppSettings {
  defaultProjectLocation: string | null
}

/** Error shape returned by every backend command (src-tauri/src/commands/error.rs). */
export interface CommandError {
  kind:
    'validation' | 'notFound' | 'conflict' | 'confirmationRequired' | 'unsafe' | 'io' | 'internal'
  message: string
  field: string | null
  existing: string | null
}
