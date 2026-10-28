import { invoke } from '@tauri-apps/api/core'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type {
  Analysis,
  GeneratedVagrantfile,
  LineEnding,
  SaveVagrantfileOutcome,
  SaveVagrantfileRequest,
  VagrantConfig,
  VagrantfileBackup,
  VagrantfileDocument,
} from '@/types/vagrantfile'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const vagrantfileApi = {
  get: (environmentId: string) => call<VagrantfileDocument>('get_vagrantfile', { environmentId }),
  analyze: (content: string) => call<Analysis>('analyze_vagrantfile', { content }),
  generate: (model: VagrantConfig, lineEnding: LineEnding) =>
    call<GeneratedVagrantfile>('generate_vagrantfile', { model, lineEnding }),
  save: (environmentId: string, request: SaveVagrantfileRequest) =>
    call<SaveVagrantfileOutcome>('save_vagrantfile', { environmentId, request }),
  backups: (environmentId: string) =>
    call<VagrantfileBackup[]>('list_vagrantfile_backups', { environmentId }),
  readBackup: (environmentId: string, name: string) =>
    call<string>('read_vagrantfile_backup', { environmentId, name }),
}
