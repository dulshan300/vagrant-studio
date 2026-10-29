import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { hasBackend } from './app'
import { BackendUnavailableError } from './errors'
import type { BoxAddRequest, BoxRef, BoxStore, OutdatedBox } from '@/types/boxes'
import type { Confirmation, ConfirmationProof, Operation } from '@/types/operations'

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasBackend()) return Promise.reject(new BackendUnavailableError())
  return invoke<T>(command, args)
}

export const boxesApi = {
  list: () => call<BoxStore>('list_boxes'),
  checkUpdates: () => call<OutdatedBox[]>('check_box_updates'),
  add: (request: BoxAddRequest) => call<Operation>('add_box', { request }),
  update: (target: BoxRef) => call<Operation>('update_box', { target }),
  requestRemoval: (target: BoxRef) => call<Confirmation>('request_box_removal', { target }),
  remove: (target: BoxRef, confirmation: ConfirmationProof) =>
    call<Operation>('remove_box', { target, confirmation }),
}

/** Native picker for a `.box` file. Resolves to null when cancelled. */
export async function pickBoxFile(): Promise<string | null> {
  if (!hasBackend()) throw new BackendUnavailableError()
  const result = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'Vagrant box', extensions: ['box'] }],
  })
  return typeof result === 'string' ? result : null
}
