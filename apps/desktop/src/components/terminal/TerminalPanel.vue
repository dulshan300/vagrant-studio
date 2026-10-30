<script setup lang="ts">
import { defineAsyncComponent, onBeforeUnmount, onMounted, ref } from 'vue'
import { ChevronDown, LoaderCircle, SquareTerminal, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useTerminalsStore } from '@/stores/terminals'

// xterm.js is only loaded once a terminal is opened.
const TerminalView = defineAsyncComponent(() => import('./TerminalView.vue'))

const store = useTerminalsStore()

const MIN_HEIGHT = 160
const height = ref(320)
function maxHeight() {
  return Math.max(MIN_HEIGHT, Math.round(window.innerHeight * 0.75))
}

let dragStart: { y: number; height: number } | null = null
function startDrag(event: PointerEvent) {
  dragStart = { y: event.clientY, height: height.value }
  ;(event.target as HTMLElement).setPointerCapture(event.pointerId)
}
function drag(event: PointerEvent) {
  if (!dragStart) return
  const next = dragStart.height + dragStart.y - event.clientY
  height.value = Math.min(maxHeight(), Math.max(MIN_HEIGHT, next))
}
function endDrag() {
  dragStart = null
}
function resizeBy(delta: number) {
  height.value = Math.min(maxHeight(), Math.max(MIN_HEIGHT, height.value + delta))
}

/** Ctrl+` shows or hides the panel when sessions exist. */
function onKey(event: KeyboardEvent) {
  if (event.ctrlKey && !event.altKey && !event.metaKey && event.code === 'Backquote') {
    if (!store.tabs.length) return
    event.preventDefault()
    store.panelOpen = !store.panelOpen
  }
}
onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

function stateText(state: string, exitCode: number | null, error: string | null) {
  if (error) return 'Could not start'
  if (state === 'starting') return 'Starting'
  if (state === 'open') return 'Connected'
  return exitCode === null ? 'Closed' : `Ended (exit code ${exitCode})`
}
</script>

<template>
  <section
    v-if="store.tabs.length"
    v-show="store.panelOpen"
    class="flex shrink-0 flex-col border-t bg-surface"
    :style="{ height: `${height}px` }"
    aria-label="Terminal panel"
  >
    <div
      class="h-1 shrink-0 cursor-row-resize hover:bg-accent/40 focus-visible:bg-accent/40 focus-visible:outline-none"
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize the terminal panel"
      tabindex="0"
      @pointerdown="startDrag"
      @pointermove="drag"
      @pointerup="endDrag"
      @pointercancel="endDrag"
      @keydown.up.prevent="resizeBy(24)"
      @keydown.down.prevent="resizeBy(-24)"
    />
    <div class="flex items-center gap-1 border-b px-2">
      <div role="tablist" aria-label="Terminal sessions" class="flex min-w-0 flex-1 gap-0.5">
        <div
          v-for="t in store.tabs"
          :key="t.key"
          :class="
            cn(
              'group flex min-w-0 items-center gap-1.5 border-b-2 border-transparent py-1.5 pr-1 pl-2.5 text-xs text-muted',
              t.key === store.activeKey && 'border-accent text-foreground',
            )
          "
        >
          <button
            role="tab"
            :aria-selected="t.key === store.activeKey"
            class="flex min-w-0 items-center gap-1.5 hover:text-foreground"
            :title="`${t.context} · ${stateText(t.state, t.exitCode, t.error)}`"
            @click="store.activeKey = t.key"
          >
            <LoaderCircle
              v-if="t.state === 'starting'"
              class="size-3 shrink-0 animate-spin"
              aria-hidden="true"
            />
            <span
              v-else
              :class="
                cn(
                  'size-2 shrink-0 rounded-full',
                  t.state === 'open' ? 'bg-status-running' : 'bg-status-stopped',
                )
              "
              aria-hidden="true"
            />
            <span class="truncate font-mono">{{ t.title }}</span>
            <span class="hidden truncate text-muted md:inline">{{ t.context }}</span>
          </button>
          <button
            class="rounded p-0.5 text-muted hover:bg-surface-muted hover:text-foreground"
            :aria-label="`Close ${t.title}`"
            :title="t.state === 'ended' ? 'Close tab' : 'End the session and close the tab'"
            @click="store.close(t.key)"
          >
            <X class="size-3" />
          </button>
        </div>
      </div>
      <Button
        variant="ghost"
        size="icon"
        class="size-7"
        title="Hide the terminal panel (Ctrl+`). Sessions keep running."
        aria-label="Hide the terminal panel"
        @click="store.panelOpen = false"
      >
        <ChevronDown />
      </Button>
    </div>
    <div class="relative min-h-0 flex-1 bg-[#0f1115]">
      <template v-for="t in store.tabs" :key="t.key">
        <div v-show="t.key === store.activeKey" class="absolute inset-0">
          <p
            v-if="t.error"
            class="flex items-start gap-2 px-4 py-3 text-sm text-[#ff8f8f]"
            role="alert"
          >
            <SquareTerminal class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
            {{ t.error }}
          </p>
          <TerminalView v-else :tab-key="t.key" :active="t.key === store.activeKey" />
        </div>
      </template>
    </div>
  </section>
</template>
