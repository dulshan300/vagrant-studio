<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import {
  ChevronDown,
  ChevronUp,
  CircleCheck,
  CircleDashed,
  CircleX,
  Copy,
  FileText,
  Search,
  Square,
  TriangleAlert,
  X,
} from '@lucide/vue'
import {
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from 'reka-ui'
import { Button } from '@/components/ui/button'
import OperationStatusBadge from './OperationStatusBadge.vue'
import { duration, isActive, operationLabel } from '@/lib/operations'
import { highlight, matchingLines, step, viewLines, type LogFilter } from '@/lib/log-view'
import { provisionerLabel } from '@/lib/provisioning'
import { errorMessage } from '@/services/errors'
import { operationsApi } from '@/services/operations'
import { useOperationsStore } from '@/stores/operations'
import { cn } from '@/lib/utils'
import type { ProvisionerRun } from '@/types/operations'

const store = useOperationsStore()
const open = computed({
  get: () => store.selectedId !== null,
  set: (v) => {
    if (!v) store.close()
  },
})
const op = computed(() => store.selected)
const lines = computed(() => (op.value ? (store.logs[op.value.id] ?? []) : []))
const provisioned = computed(() => (op.value ? (store.provisioning[op.value.id] ?? []) : []))
const STATUS_TEXT = { succeeded: 'Succeeded', failed: 'Failed', unfinished: 'Interrupted' }

function outputText(run: ProvisionerRun): string {
  const text = run.output.join('\n') || '(no output)'
  return run.truncated ? `…\n${text}` : text
}
const cancelError = ref<string | null>(null)
const logEl = ref<HTMLElement | null>(null)
const follow = ref(true)

// Output view: filter, search and problem navigation. Positions index `shown`.
const logFilter = ref<LogFilter>('all')
const query = ref('')
const current = ref<number | null>(null)
const searchEl = ref<HTMLInputElement | null>(null)
const shown = computed(() => viewLines(lines.value, logFilter.value))
const matches = computed(() => matchingLines(shown.value, query.value))
const problems = computed(() => shown.value.flatMap((l, i) => (l.severity ? [i] : [])))
const matchNumber = computed(() =>
  current.value === null ? 0 : matches.value.indexOf(current.value) + 1,
)
const LOG_FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'problems', label: 'Problems' },
  { value: 'stderr', label: 'stderr' },
] as const

watch(
  () => store.selectedId,
  () => {
    logFilter.value = 'all'
    query.value = ''
    current.value = null
    follow.value = true
    actionMessage.value = null
  },
)
watch([query, logFilter], async () => {
  current.value = null
  if (query.value.trim()) {
    void goTo(step(matches.value, null, true))
  } else if (logFilter.value === 'all') {
    // Back to the plain view: follow the output again.
    follow.value = true
    await nextTick()
    logEl.value?.scrollTo({ top: logEl.value.scrollHeight })
  }
})

async function goTo(position: number | null) {
  if (position === null) return
  current.value = position
  follow.value = false
  await nextTick()
  logEl.value?.querySelector(`[data-pos="${position}"]`)?.scrollIntoView({ block: 'center' })
}
function nextMatch(forward = true) {
  void goTo(step(matches.value, current.value, forward))
}
function nextProblem(forward = true) {
  void goTo(step(problems.value, current.value, forward))
}
function onSearchKey(event: KeyboardEvent) {
  if (event.key === 'Enter') {
    event.preventDefault()
    nextMatch(!event.shiftKey)
  } else if (event.key === 'Escape' && query.value) {
    // Clear the search before Escape closes the drawer.
    event.preventDefault()
    event.stopPropagation()
    query.value = ''
  }
}
function onDrawerKey(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 'f') {
    event.preventDefault()
    searchEl.value?.focus()
    searchEl.value?.select()
  }
}

const actionMessage = ref<{ text: string; error: boolean } | null>(null)
async function copyOutput() {
  try {
    await navigator.clipboard.writeText(shown.value.map((l) => l.text).join('\n'))
    actionMessage.value = {
      text: `Copied ${shown.value.length} ${shown.value.length === 1 ? 'line' : 'lines'}.`,
      error: false,
    }
  } catch (e) {
    actionMessage.value = { text: `Could not copy: ${errorMessage(e)}`, error: true }
  }
}
async function revealLog() {
  if (!op.value) return
  actionMessage.value = null
  try {
    await operationsApi.revealLog(op.value.id)
  } catch (e) {
    actionMessage.value = { text: errorMessage(e), error: true }
  }
}

// Re-render the running duration every second.
const now = ref(Date.now())
const timer = setInterval(() => (now.value = Date.now()), 1000)
onBeforeUnmount(() => clearInterval(timer))

watch(
  () => lines.value.length,
  async () => {
    if (!follow.value || query.value.trim() || logFilter.value !== 'all') return
    await nextTick()
    logEl.value?.scrollTo({ top: logEl.value.scrollHeight })
  },
)

function onScroll() {
  const el = logEl.value
  if (el) follow.value = el.scrollHeight - el.scrollTop - el.clientHeight < 40
}

async function cancel() {
  if (!op.value) return
  cancelError.value = null
  try {
    await store.cancel(op.value.id)
  } catch (e) {
    cancelError.value = errorMessage(e)
  }
}
</script>

<template>
  <DialogRoot v-model:open="open">
    <DialogPortal>
      <DialogOverlay class="fixed inset-0 z-40 bg-black/30" />
      <DialogContent
        class="fixed inset-y-0 right-0 z-50 flex w-full max-w-3xl flex-col border-l bg-surface shadow-xl focus:outline-none"
        @keydown="onDrawerKey"
      >
        <template v-if="op">
          <header class="border-b px-5 py-4">
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0">
                <DialogTitle class="flex items-center gap-2 font-semibold">
                  {{ operationLabel(op.operationType) }}
                  <OperationStatusBadge :status="op.status" />
                </DialogTitle>
                <DialogDescription class="mt-0.5 truncate text-xs text-muted">
                  {{ op.targetLabel || 'Host'
                  }}<template v-if="op.machineName"> · {{ op.machineName }}</template>
                </DialogDescription>
              </div>
              <div class="flex items-center gap-2">
                <Button
                  v-if="isActive(op)"
                  variant="secondary"
                  size="sm"
                  :disabled="op.status === 'cancel_requested'"
                  @click="cancel"
                >
                  <Square /> {{ op.status === 'cancel_requested' ? 'Cancelling…' : 'Cancel' }}
                </Button>
                <DialogClose
                  class="rounded-md p-1 text-muted hover:bg-surface-muted hover:text-foreground"
                  aria-label="Close"
                >
                  <X class="size-4" />
                </DialogClose>
              </div>
            </div>
            <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs">
              <dt class="text-muted">Command</dt>
              <dd>
                <code class="font-mono">{{ op.commandLine }}</code>
              </dd>
              <template v-if="op.workingDir">
                <dt class="text-muted">Folder</dt>
                <dd>
                  <code class="font-mono break-all">{{ op.workingDir }}</code>
                </dd>
              </template>
              <dt class="text-muted">Duration</dt>
              <dd class="tabular-nums">
                {{ duration(op, now)
                }}<template v-if="op.exitCode !== null"> · exit code {{ op.exitCode }}</template>
              </dd>
            </dl>
            <p
              v-if="op.errorSummary"
              :class="
                cn(
                  'mt-3 rounded-md px-3 py-2 text-xs whitespace-pre-wrap',
                  op.status === 'cancelled'
                    ? 'bg-surface-muted'
                    : 'bg-status-failed/10 text-status-failed',
                )
              "
            >
              {{ op.errorSummary }}
            </p>
            <p v-if="cancelError" class="mt-2 text-xs text-status-failed">{{ cancelError }}</p>
          </header>

          <section
            v-if="provisioned.length"
            class="max-h-[40%] overflow-auto border-b px-5 py-3"
            aria-label="Provisioners"
          >
            <h3 class="mb-2 text-xs font-medium">Provisioners</h3>
            <div v-for="m in provisioned" :key="m.machine" class="mb-2 last:mb-0">
              <p v-if="provisioned.length > 1" class="mb-1 font-mono text-[11px] text-muted">
                {{ m.machine }}
              </p>
              <details
                v-for="(run, i) in m.runs"
                :key="i"
                class="group rounded-md text-xs"
                :open="run.status !== 'succeeded'"
              >
                <summary
                  class="flex cursor-pointer items-center gap-2 rounded-md px-1.5 py-1 hover:bg-surface-muted"
                >
                  <CircleCheck
                    v-if="run.status === 'succeeded'"
                    class="size-3.5 shrink-0 text-status-running"
                    aria-hidden="true"
                  />
                  <CircleX
                    v-else-if="run.status === 'failed'"
                    class="size-3.5 shrink-0 text-status-failed"
                    aria-hidden="true"
                  />
                  <CircleDashed v-else class="size-3.5 shrink-0 text-muted" aria-hidden="true" />
                  <span>{{ provisionerLabel(run) }}</span>
                  <span class="text-muted">{{ STATUS_TEXT[run.status] }}</span>
                </summary>
                <pre
                  class="mt-1 ml-6 overflow-x-auto rounded bg-surface-muted px-2 py-1.5 font-mono text-[11px] whitespace-pre-wrap"
                  >{{ outputText(run) }}</pre>
              </details>
            </div>
          </section>

          <div
            class="flex flex-wrap items-center gap-2 border-b px-4 py-2"
            role="toolbar"
            aria-label="Output tools"
          >
            <div class="relative">
              <Search
                class="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted"
                aria-hidden="true"
              />
              <input
                ref="searchEl"
                v-model="query"
                type="search"
                placeholder="Search output"
                aria-label="Search output"
                title="Enter: next match · Shift+Enter: previous · Ctrl/⌘+F: focus"
                class="w-48 rounded-md border bg-surface py-1 pr-2 pl-7 text-xs"
                @keydown="onSearchKey"
              />
            </div>
            <template v-if="query.trim()">
              <span class="text-xs text-muted tabular-nums" aria-live="polite">
                {{ matches.length ? `${matchNumber} of ${matches.length}` : 'No matches' }}
              </span>
              <Button
                variant="ghost"
                size="icon"
                class="size-7"
                aria-label="Previous match"
                :disabled="!matches.length"
                @click="nextMatch(false)"
              >
                <ChevronUp />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                class="size-7"
                aria-label="Next match"
                :disabled="!matches.length"
                @click="nextMatch()"
              >
                <ChevronDown />
              </Button>
            </template>
            <div
              role="tablist"
              aria-label="Lines to show"
              class="inline-flex rounded-md border bg-surface-muted p-0.5"
            >
              <button
                v-for="f in LOG_FILTERS"
                :key="f.value"
                role="tab"
                :aria-selected="logFilter === f.value"
                :class="
                  cn(
                    'rounded px-2 py-0.5 text-xs text-muted hover:text-foreground',
                    logFilter === f.value && 'bg-surface text-foreground shadow-xs',
                  )
                "
                :title="
                  f.value === 'problems'
                    ? 'stderr lines and lines that mention errors or warnings'
                    : f.value === 'stderr'
                      ? 'Only what Vagrant wrote to stderr. Guest output arrives on stdout.'
                      : undefined
                "
                @click="logFilter = f.value"
              >
                {{ f.label }}
              </button>
            </div>
            <Button
              variant="ghost"
              size="sm"
              class="h-7 px-2"
              :disabled="!problems.length"
              :title="`Jump to the next line that mentions an error or warning (${problems.length})`"
              @click="nextProblem()"
            >
              <TriangleAlert /> Next problem
            </Button>
            <span class="ml-auto flex items-center gap-1">
              <Button
                variant="ghost"
                size="icon"
                class="size-7"
                title="Copy the lines shown"
                aria-label="Copy output"
                :disabled="!shown.length"
                @click="copyOutput"
              >
                <Copy />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                class="size-7"
                title="Show the log file in the file manager"
                aria-label="Show log file"
                :disabled="!op.logPath"
                @click="revealLog"
              >
                <FileText />
              </Button>
            </span>
            <p
              v-if="actionMessage"
              :class="
                cn('w-full text-xs', actionMessage.error ? 'text-status-failed' : 'text-muted')
              "
              role="status"
            >
              {{ actionMessage.text }}
            </p>
          </div>

          <div
            ref="logEl"
            class="flex-1 overflow-auto bg-[#0f1115] px-4 py-3 font-mono text-[12px] leading-5 text-[#d6d9df]"
            role="log"
            aria-live="polite"
            aria-label="Command output"
            @scroll="onScroll"
          >
            <p v-if="!lines.length" class="text-[#8b919c]">
              {{ isActive(op) ? 'Waiting for output…' : 'No output was recorded.' }}
            </p>
            <p v-else-if="!shown.length" class="text-[#8b919c]">No lines match this filter.</p>
            <div
              v-for="(line, i) in shown"
              :key="line.index"
              :data-pos="i"
              :class="
                cn(
                  'break-words whitespace-pre-wrap',
                  line.kind === 'err' && 'text-[#ff8f8f]',
                  line.kind === 'meta' && 'text-[#8b919c]',
                  line.kind === 'out' && line.severity === 'warning' && 'text-[#f2c66d]',
                  i === current && 'bg-white/10 outline outline-1 outline-white/30',
                )
              "
            >
              <template v-if="query.trim()"
                ><template v-for="(seg, j) in highlight(line.text, query)" :key="j"
                  ><mark v-if="seg.match" class="rounded-sm bg-[#f2c66d] text-[#0f1115]">{{
                    seg.text
                  }}</mark
                  ><template v-else>{{ seg.text }}</template></template
                ></template
              >
              <template v-else>{{ line.text || ' ' }}</template>
            </div>
          </div>
        </template>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
