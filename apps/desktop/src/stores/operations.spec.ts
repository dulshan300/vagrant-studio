import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useOperationsStore } from './operations'
import type { Operation } from '@/types/operations'

const op = (status: Operation['status']): Operation => ({
  id: 'op1',
  projectId: null,
  environmentId: 'env1',
  machineName: null,
  targetLabel: 'Web · default',
  operationType: 'status',
  status,
  commandLine: 'vagrant status --machine-readable',
  workingDir: '/p',
  startedAt: null,
  finishedAt: null,
  exitCode: null,
  logPath: null,
  errorSummary: null,
  createdAt: '2026-10-28T12:00:00Z',
})

describe('operations store events', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('tracks an operation from queued to completed with live output and status', () => {
    const store = useOperationsStore()
    const base = { timestamp: '', operationId: 'op1', environmentId: 'env1', machineName: null }
    store.handle({ ...base, type: 'operation.queued', payload: { operation: op('queued') } })
    store.handle({ ...base, type: 'operation.started', payload: { operation: op('running') } })
    store.handle({ ...base, type: 'operation.stdout', payload: { line: 'hello' } })
    store.handle({ ...base, type: 'operation.stderr', payload: { line: 'oops' } })
    store.handle({
      ...base,
      type: 'environment.status_updated',
      payload: {
        status: {
          environmentId: 'env1',
          operationId: 'op1',
          observedAt: '',
          machines: [],
          error: null,
        },
      },
    })
    store.handle({ ...base, type: 'operation.completed', payload: { operation: op('succeeded') } })

    expect(store.operations).toHaveLength(1)
    expect(store.operations[0]!.status).toBe('succeeded')
    expect(store.logs.op1).toEqual([
      { kind: 'out', text: 'hello' },
      { kind: 'err', text: 'oops' },
    ])
    expect(store.statuses.env1?.operationId).toBe('op1')
    expect(store.latestFor('env1')?.id).toBe('op1')
    expect(store.active).toHaveLength(0)
  })

  it('marks cancellation requests without overriding finished operations', () => {
    const store = useOperationsStore()
    const base = { timestamp: '', operationId: 'op1', environmentId: 'env1', machineName: null }
    store.handle({ ...base, type: 'operation.started', payload: { operation: op('running') } })
    store.handle({ ...base, type: 'operation.cancel_requested', payload: {} })
    expect(store.operations[0]!.status).toBe('cancel_requested')
  })

  it('lists active operations per environment, oldest first', () => {
    const store = useOperationsStore()
    const base = { timestamp: '', environmentId: 'env1', machineName: null }
    for (const [id, env, status] of [
      ['a', 'env1', 'running'],
      ['b', 'env2', 'queued'],
      ['c', 'env1', 'queued'],
      ['d', 'env1', 'succeeded'],
    ] as const) {
      store.handle({
        ...base,
        type: 'operation.queued',
        operationId: id,
        payload: { operation: { ...op(status), id, environmentId: env } },
      })
    }
    expect(store.activeFor('env1').map((o) => o.id)).toEqual(['a', 'c'])
  })
})
