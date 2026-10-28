<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { Square, X } from '@lucide/vue'
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
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import { cn } from '@/lib/utils'

const store = useOperationsStore()
const open = computed({
  get: () => store.selectedId !== null,
  set: (v) => {
    if (!v) store.close()
  },
})
const op = computed(() => store.selected)
const lines = computed(() => (op.value ? (store.logs[op.value.id] ?? []) : []))
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
