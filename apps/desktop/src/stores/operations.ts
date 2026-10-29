import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { operationsApi, onOperationEvent, type LifecycleOptions } from '@/services/operations'
import { errorMessage } from '@/services/errors'
import { isActive } from '@/lib/operations'
import type {
  EnvironmentStatus,
  HostReadiness,
  LifecycleAction,
  LogLine,
  MachineProvisioning,
  Operation,
  OperationEvent,
} from '@/types/operations'

const MAX_LOG_LINES = 5000
/** Operations whose output is read for provisioner results. */
const PROVISIONING_TYPES = new Set(['up', 'reload', 'provision'])

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
  /** Provisioner results by operation id. */
  const provisioning = ref<Record<string, MachineProvisioning[]>>({})
  /** The latest provisioner results of each machine, by environment id. */
  const latestProvisioning = ref<Record<string, MachineProvisioning[]>>({})
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
    if (event.type === 'operation.provisioning' && event.operationId && event.payload.machines) {
      recordProvisioning(event.operationId, event.environmentId, event.payload.machines)
    }
  }

  function recordProvisioning(
    operationId: string,
    environmentId: string | null,
    machines: MachineProvisioning[],
  ) {
    provisioning.value[operationId] = machines
    const latest = environmentId ? latestProvisioning.value[environmentId] : undefined
    if (!latest) return
    for (const m of machines) {
      const i = latest.findIndex((l) => l.machine === m.machine)
      if (i === -1) latest.push(m)
      else latest.splice(i, 1, m)
    }
  }

  async function loadLatestProvisioning(environmentId: string) {
    latestProvisioning.value[environmentId] = await operationsApi.latestProvisioning(environmentId)
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
    options: LifecycleOptions = {},
  ) {
    const op = await operationsApi.runLifecycle(environmentId, action, machines, options)
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
    if (!provisioning.value[id]) {
      const op = operations.value.find((o) => o.id === id)
      if (op && !isActive(op) && PROVISIONING_TYPES.has(op.operationType))
        provisioning.value[id] = await operationsApi.operationProvisioning(id)
    }
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
    provisioning,
    latestProvisioning,
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
    loadLatestProvisioning,
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
