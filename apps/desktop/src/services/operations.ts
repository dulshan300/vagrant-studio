import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type {
  Confirmation,
  ConfirmationProof,
  EnvironmentStatus,
  HostReadiness,
  LifecycleAction,
  LogLine,
  Operation,
  OperationEvent,
} from '@/types/operations'

/** Must match OPERATION_EVENT in src-tauri/src/commands/operations.rs. */
const OPERATION_EVENT = 'vagrant-studio://operation'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const operationsApi = {
  hostReadiness: () => call<HostReadiness>('get_host_readiness'),
  refreshStatus: (environmentId: string) =>
    call<Operation>('refresh_environment_status', { environmentId }),
  validate: (environmentId: string) => call<Operation>('validate_environment', { environmentId }),
  runLifecycle: (
    environmentId: string,
    action: LifecycleAction,
    machines: string[],
    confirmation: ConfirmationProof | null = null,
  ) => call<Operation>('run_lifecycle', { environmentId, action, machines, confirmation }),
  requestConfirmation: (environmentId: string, action: LifecycleAction, machines: string[]) =>
    call<Confirmation>('request_lifecycle_confirmation', { environmentId, action, machines }),
  environmentStatus: (environmentId: string) =>
    call<EnvironmentStatus | null>('get_environment_status', { environmentId }),
  environmentStatuses: () => call<EnvironmentStatus[]>('list_environment_statuses'),
  list: (environmentId: string | null = null, limit = 200) =>
    call<Operation[]>('list_operations', { environmentId, limit }),
  get: (id: string) => call<Operation>('get_operation', { id }),
  log: (id: string) => call<LogLine[]>('read_operation_log', { id }),
  cancel: (id: string) => call<boolean>('cancel_operation', { id }),
}

export async function onOperationEvent(
  handler: (event: OperationEvent) => void,
): Promise<UnlistenFn> {
  if (!hasBackend()) return () => undefined
  return listen<OperationEvent>(OPERATION_EVENT, (e) => handler(e.payload))
}
