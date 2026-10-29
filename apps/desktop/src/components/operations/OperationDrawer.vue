<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { CircleCheck, CircleDashed, CircleX, Square, X } from '@lucide/vue'
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
import { displayLine, duration, isActive, operationLabel } from '@/lib/operations'
import { provisionerLabel } from '@/lib/provisioning'
import { errorMessage } from '@/services/errors'
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

// Re-render the running duration every second.
const now = ref(Date.now())
const timer = setInterval(() => (now.value = Date.now()), 1000)
onBeforeUnmount(() => clearInterval(timer))

watch(
  () => lines.value.length,
  async () => {
    if (!follow.value) return
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
            <div
              v-for="(line, i) in lines"
              :key="i"
              :class="
                cn(
                  'break-words whitespace-pre-wrap',
                  line.kind === 'err' && 'text-[#ff8f8f]',
                  line.kind === 'meta' && 'text-[#8b919c]',
                )
              "
            >
              {{ displayLine(line) || ' ' }}
            </div>
          </div>
        </template>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
