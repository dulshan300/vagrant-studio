import type { CommandError } from '@/types/projects'

export function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'kind' in value &&
    'message' in value &&
    typeof (value as { message: unknown }).message === 'string'
  )
}

/** Human-readable message for anything thrown by a service call. */
export function errorMessage(error: unknown): string {
  if (isCommandError(error)) return error.message
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  return 'Something went wrong.'
}

export class BackendUnavailableError extends Error {
  constructor() {
    super('This needs the Vagrant Studio desktop app; it is not available in a plain browser.')
  }
}
