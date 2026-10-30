import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTerminalsStore } from './terminals'
import { terminalApi, type TerminalEvent } from '@/services/terminal'

vi.mock('@/services/terminal', () => ({
  terminalApi: {
    startSsh: vi.fn(),
    write: vi.fn(),
    resize: vi.fn(),
    close: vi.fn(),
  },
}))

/** Starts a session whose events the test sends; resolves the start when told to. */
function pendingStart() {
  let emit: (event: TerminalEvent) => void = () => undefined
  let resolve: () => void = () => undefined
  vi.mocked(terminalApi.startSsh).mockImplementation(
    (_env, _machine, _size, onEvent) =>
      new Promise((done) => {
        emit = onEvent
        resolve = () => done({ id: 's1', title: 'ssh web' })
      }),
  )
  return { emit: (e: TerminalEvent) => emit(e), resolve: () => resolve() }
}

describe('terminals store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    for (const fn of Object.values(terminalApi)) vi.mocked(fn).mockResolvedValue(undefined as never)
  })
  afterEach(() => vi.clearAllMocks())

  it('buffers output until the view attaches and keeps early input for the session', async () => {
    const store = useTerminalsStore()
    const start = pendingStart()
    const opened = store.openSsh('e1', 'web', 'Web · default')
    expect(store.tabs[0]).toMatchObject({ state: 'starting', title: 'ssh web' })
    expect(store.panelOpen).toBe(true)

    // ConPTY's cursor query arrives before the start resolves; the answer waits.
    start.emit({ kind: 'output', data: '\x1b[6n' })
    const key = store.tabs[0]!.key
    store.input(key, '\x1b[1;1R')
    expect(terminalApi.write).not.toHaveBeenCalled()

    start.resolve()
    await opened
    expect(store.tabs[0]).toMatchObject({ sessionId: 's1', state: 'open' })
    expect(terminalApi.write).toHaveBeenCalledWith('s1', '\x1b[1;1R')

    start.emit({ kind: 'output', data: 'Welcome' })
    const written: string[] = []
    store.attach(key, (d) => written.push(d))
    start.emit({ kind: 'output', data: '$ ' })
    expect(written).toEqual(['\x1b[6nWelcome', '$ '])

    store.resize(key, { cols: 120, rows: 40 })
    expect(terminalApi.resize).toHaveBeenCalledWith('s1', { cols: 120, rows: 40 })
  })

  it('marks the tab ended on exit and stops sending input', async () => {
    const store = useTerminalsStore()
    const start = pendingStart()
    const opened = store.openSsh('e1', 'web', 'ctx')
    start.resolve()
    await opened
    const key = store.tabs[0]!.key
    const written: string[] = []
    store.attach(key, (d) => written.push(d))

    start.emit({ kind: 'exit', code: 130 })
    expect(store.tabs[0]).toMatchObject({ state: 'ended', exitCode: 130 })
    expect(written.join('')).toContain('exit code 130')
    store.input(key, 'x')
    expect(terminalApi.write).not.toHaveBeenCalled()

    store.close(key)
    expect(terminalApi.close).not.toHaveBeenCalled()
    expect(store.tabs).toEqual([])
    expect(store.panelOpen).toBe(false)
  })

  it('closing a running tab ends its session and selects a neighbour', async () => {
    const store = useTerminalsStore()
    vi.mocked(terminalApi.startSsh)
      .mockResolvedValueOnce({ id: 'a', title: 'ssh web' })
      .mockResolvedValueOnce({ id: 'b', title: 'ssh db' })
    await store.openSsh('e1', 'web', 'ctx')
    await store.openSsh('e1', 'db', 'ctx')
    expect(store.active?.sessionId).toBe('b')

    store.close(store.tabs[1]!.key)
    expect(terminalApi.close).toHaveBeenCalledWith('b')
    expect(store.active?.sessionId).toBe('a')
    expect(store.panelOpen).toBe(true)
  })

  it('keeps a tab with the reason when the session could not start', async () => {
    const store = useTerminalsStore()
    vi.mocked(terminalApi.startSsh).mockRejectedValueOnce({
      kind: 'validation',
      message: 'web isn’t running.',
    })
    await expect(store.openSsh('e1', 'web', 'ctx')).rejects.toBeTruthy()
    expect(store.tabs[0]).toMatchObject({ state: 'ended', error: 'web isn’t running.' })
  })
})
