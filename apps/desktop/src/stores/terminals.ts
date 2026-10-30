import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { terminalApi, type TerminalEvent, type TerminalSize } from '@/services/terminal'

export type SessionState = 'starting' | 'open' | 'ended'

export interface TerminalTab {
  /** Local key; the backend id arrives once the session has started. */
  key: string
  sessionId: string | null
  title: string
  /** Where it runs, e.g. "Web · default". */
  context: string
  state: SessionState
  /** Exit code once ended; null when closed from the app or unknown. */
  exitCode: number | null
  error: string | null
}

/** Size used until the panel has measured itself. */
export const INITIAL_SIZE: TerminalSize = { cols: 100, rows: 24 }
/** Output kept for a tab whose view isn't attached yet (characters). */
const MAX_BUFFER = 256 * 1024

type Writer = (data: string) => void

/**
 * Embedded terminal sessions (`vagrant ssh` in the terminal panel). Output
 * goes straight to the tab's xterm.js view; it is held in memory only until
 * the view attaches, and never stored.
 */
export const useTerminalsStore = defineStore('terminals', () => {
  const tabs = ref<TerminalTab[]>([])
  const activeKey = ref<string | null>(null)
  const panelOpen = ref(false)
  let nextKey = 1

  // Not reactive: output can arrive faster than Vue should track it.
  const writers = new Map<string, Writer>()
  const buffers = new Map<string, string>()
  /**
   * Input typed (or answered by xterm.js) before the session id is known.
   * ConPTY asks for the cursor position as soon as it starts and waits for
   * the answer, so this must not be dropped.
   */
  const pendingInput = new Map<string, string>()

  const active = computed(() => tabs.value.find((t) => t.key === activeKey.value) ?? null)
  const running = computed(() => tabs.value.filter((t) => t.state !== 'ended').length)

  function tab(key: string) {
    return tabs.value.find((t) => t.key === key)
  }

  function deliver(key: string, data: string) {
    const writer = writers.get(key)
    if (writer) {
      writer(data)
      return
    }
    const buffered = (buffers.get(key) ?? '') + data
    buffers.set(key, buffered.length > MAX_BUFFER ? buffered.slice(-MAX_BUFFER) : buffered)
  }

  function onEvent(key: string, event: TerminalEvent) {
    if (event.kind === 'output') {
      deliver(key, event.data)
      return
    }
    const t = tab(key)
    if (!t) return
    t.state = 'ended'
    t.exitCode = event.code
    const how = event.code === null ? 'closed' : `ended with exit code ${event.code}`
    deliver(key, `\r\n\x1b[2m[session ${how}]\x1b[0m\r\n`)
  }

  /** Opens `vagrant ssh <machine>` in a new tab and shows the panel. */
  async function openSsh(
    environmentId: string,
    machine: string,
    context: string,
    size: TerminalSize = INITIAL_SIZE,
  ) {
    const key = `t${nextKey++}`
    tabs.value.push({
      key,
      sessionId: null,
      title: `ssh ${machine}`,
      context,
      state: 'starting',
      exitCode: null,
      error: null,
    })
    activeKey.value = key
    panelOpen.value = true
    try {
      const session = await terminalApi.startSsh(environmentId, machine, size, (event) =>
        onEvent(key, event),
      )
      const t = tab(key)
      if (!t) {
        // The tab was closed while starting.
        void terminalApi.close(session.id).catch(() => undefined)
        return
      }
      t.sessionId = session.id
      t.title = session.title
      if (t.state === 'starting') t.state = 'open'
      const pending = pendingInput.get(key)
      pendingInput.delete(key)
      if (pending) void terminalApi.write(session.id, pending).catch(() => undefined)
    } catch (e) {
      const t = tab(key)
      if (t) {
        t.state = 'ended'
        t.error =
          e instanceof Error ? e.message : ((e as { message?: string })?.message ?? String(e))
      }
      throw e
    }
  }

  /** Connects a tab's view; buffered output is written first. */
  function attach(key: string, writer: Writer) {
    const buffered = buffers.get(key)
    if (buffered) writer(buffered)
    buffers.delete(key)
    writers.set(key, writer)
  }

  function detach(key: string) {
    writers.delete(key)
  }

  function input(key: string, data: string) {
    const t = tab(key)
    if (!t || t.state === 'ended') return
    if (!t.sessionId) {
      pendingInput.set(key, (pendingInput.get(key) ?? '') + data)
      return
    }
    void terminalApi.write(t.sessionId, data).catch(() => undefined)
  }

  function resize(key: string, size: TerminalSize) {
    const t = tab(key)
    if (t?.sessionId && t.state === 'open')
      void terminalApi.resize(t.sessionId, size).catch(() => undefined)
  }

  /** Ends the session (if still running) and removes its tab. */
  function close(key: string) {
    const index = tabs.value.findIndex((t) => t.key === key)
    if (index === -1) return
    const [t] = tabs.value.splice(index, 1)
    if (t?.sessionId && t.state !== 'ended')
      void terminalApi.close(t.sessionId).catch(() => undefined)
    writers.delete(key)
    buffers.delete(key)
    pendingInput.delete(key)
    if (activeKey.value === key) {
      activeKey.value = tabs.value[Math.min(index, tabs.value.length - 1)]?.key ?? null
    }
    if (!tabs.value.length) panelOpen.value = false
  }

  return {
    tabs,
    activeKey,
    panelOpen,
    active,
    running,
    openSsh,
    attach,
    detach,
    input,
    resize,
    close,
  }
})
