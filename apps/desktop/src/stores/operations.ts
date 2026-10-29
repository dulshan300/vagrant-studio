import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { operationsApi, onOperationEvent } from '@/services/operations'
import { errorMessage } from '@/services/errors'
import { isActive } from '@/lib/operations'
import type {
  ConfirmationProof,
  EnvironmentStatus,
  HostReadiness,
  LifecycleAction,
  LogLine,
  Operation,
  OperationEvent,
} from '@/types/operations'

const MAX_LOG_LINES = 5000

/**
 * Live view of operations. History comes from the backend database; events
 * only update what is on screen.
 */
export const useOperationsStore = defineStore('operations', () => {
  const operations = ref<Operation[]>([])
  const loaded = ref(false)
  const error = ref<string | null>(null)
  const logs = ref<Record<string, LogLine[]>>({})
  const statuses = ref<Record<string, EnvironmentStatus>>({})
  const readiness = ref<HostReadiness | null>(null)
  const readinessLoading = ref(false)
  const readinessError = ref<string | null>(null)
  const selectedId = ref<string | null>(null)
  let subscribed = false

  const active = computed(() => operations.value.filter(isActive))
  const selected = computed(() => operations.value.find((o) => o.id === selectedId.value) ?? null)

  function upsert(op: Operation) {
    const i = operations.value.findIndex((o) => o.id === op.id)
    if (i === -1) operations.value.unshift(op)
    else operations.value.splice(i, 1, op)
  }

  function appendLog(id: string, line: LogLine) {
    const lines = logs.value[id]
    if (!lines) return
    lines.push(line)
    if (lines.length > MAX_LOG_LINES) lines.splice(0, lines.length - MAX_LOG_LINES)
  }

  function handle(event: OperationEvent) {
    const { operation, line, status } = event.payload
    if (operation) upsert(operation)
    if (event.type === 'operation.queued' && operation) logs.value[operation.id] ??= []
    if (event.operationId && line !== undefined) {
      appendLog(event.operationId, {
        kind: event.type === 'operation.stderr' ? 'err' : 'out',
        text: line,
      })
    }
    if (event.type === 'operation.cancel_requested' && event.operationId) {
      const op = operations.value.find((o) => o.id === event.operationId)
      if (op && !op.finishedAt) op.status = 'cancel_requested'
    }
    if (status) statuses.value[status.environmentId] = status
  }

  async function init() {
    if (!subscribed) {
      subscribed = true
      await onOperationEvent(handle)
    }
    await load()
  }

  async function load() {
    try {
      operations.value = await operationsApi.list(null, 200)
      error.value = null
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loaded.value = true
    }
  }

  async function loadStatuses() {
    for (const s of await operationsApi.environmentStatuses()) statuses.value[s.environmentId] = s
  }

  async function loadStatus(environmentId: string) {
    const s = await operationsApi.environmentStatus(environmentId)
    if (s) statuses.value[environmentId] = s
  }

  async function checkHost() {
    readinessLoading.value = true
    try {
      readiness.value = await operationsApi.hostReadiness()
      readinessError.value = null
    } catch (e) {
      readinessError.value = errorMessage(e)
    } finally {
      readinessLoading.value = false
    }
  }

  async function refreshStatus(environmentId: string) {
    const op = await operationsApi.refreshStatus(environmentId)
    upsert(op)
    logs.value[op.id] ??= []
    return op
  }

  async function validate(environmentId: string) {
    const op = await operationsApi.validate(environmentId)
    upsert(op)
    logs.value[op.id] ??= []
    return op
  }

  async function runLifecycle(
    environmentId: string,
    action: LifecycleAction,
    machines: string[],
    confirmation: ConfirmationProof | null = null,
  ) {
    const op = await operationsApi.runLifecycle(environmentId, action, machines, confirmation)
    upsert(op)
    logs.value[op.id] ??= []
    return op
  }

  /** Follows an operation started outside this store (e.g. a box change). */
  function track(op: Operation) {
    upsert(op)
    logs.value[op.id] ??= []
    return op
  }

  async function cancel(id: string) {
    return operationsApi.cancel(id)
  }

  /** Opens the output drawer; loads the stored log for operations not followed live. */
  async function open(id: string) {
    selectedId.value = id
    if (!operations.value.some((o) => o.id === id)) upsert(await operationsApi.get(id))
    if (!logs.value[id]?.length) logs.value[id] = await operationsApi.log(id)
  }

  function close() {
    selectedId.value = null
  }

  function latestFor(environmentId: string): Operation | undefined {
    return operations.value.find((o) => o.environmentId === environmentId)
  }

  /** Queued and running operations for an environment, oldest first. */
  function activeFor(environmentId: string): Operation[] {
    return active.value.filter((o) => o.environmentId === environmentId).reverse()
  }

  return {
    operations,
    loaded,
    error,
    logs,
    statuses,
    readiness,
    readinessLoading,
    readinessError,
    selectedId,
    active,
    selected,
    handle,
    init,
    load,
    loadStatuses,
    loadStatus,
    checkHost,
    refreshStatus,
    validate,
    runLifecycle,
    track,
    cancel,
    open,
    close,
    latestFor,
    activeFor,
  }
})
