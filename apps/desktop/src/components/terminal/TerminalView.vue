<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { useTerminalsStore } from '@/stores/terminals'

/** One xterm.js terminal bound to a session tab. Stays mounted while the tab exists. */
const props = defineProps<{ tabKey: string; active: boolean }>()

const store = useTerminalsStore()
const el = ref<HTMLElement | null>(null)
let term: Terminal | null = null
let fit: FitAddon | null = null
let observer: ResizeObserver | null = null
let lastSize = ''

function refit() {
  if (!term || !fit || !props.active || !el.value?.offsetParent) return
  fit.fit()
  const size = `${term.cols}x${term.rows}`
  if (size !== lastSize) {
    lastSize = size
    store.resize(props.tabKey, { cols: term.cols, rows: term.rows })
  }
}

onMounted(() => {
  term = new Terminal({
    cursorBlink: true,
    fontFamily: 'ui-monospace, "Cascadia Mono", Consolas, "DejaVu Sans Mono", monospace',
    fontSize: 13,
    scrollback: 5000,
    theme: { background: '#0f1115', foreground: '#d6d9df', cursor: '#d6d9df' },
  })
  fit = new FitAddon()
  term.loadAddon(fit)
  term.open(el.value!)
  term.onData((data) => store.input(props.tabKey, data))
  store.attach(props.tabKey, (data) => term?.write(data))
  observer = new ResizeObserver(() => refit())
  observer.observe(el.value!)
  refit()
  if (props.active) term.focus()
})

watch(
  () => props.active,
  (active) => {
    if (!active) return
    requestAnimationFrame(() => {
      refit()
      term?.focus()
    })
  },
)

onBeforeUnmount(() => {
  store.detach(props.tabKey)
  observer?.disconnect()
  term?.dispose()
  term = null
})
</script>

<template>
  <div ref="el" class="h-full w-full overflow-hidden bg-[#0f1115] px-2 py-1" />
</template>
