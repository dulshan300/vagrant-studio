<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { CircleAlert, GitBranch, LoaderCircle } from '@lucide/vue'
import { branchLabel, changeCount, changeSummary } from '@/lib/git'
import { hasBackend } from '@/services/app'
import { errorMessage } from '@/services/errors'
import { gitApi } from '@/services/git'
import type { GitStatus } from '@/types/git'

const props = defineProps<{ projectId: string }>()

const status = ref<GitStatus | null>(null)
const loading = ref(false)

async function load() {
  if (!hasBackend()) return
  loading.value = true
  try {
    status.value = await gitApi.status(props.projectId)
  } catch (e) {
    status.value = { state: 'error', message: errorMessage(e) }
  } finally {
    loading.value = false
  }
}
watch(() => props.projectId, load, { immediate: true })

const repo = computed(() => (status.value?.state === 'repository' ? status.value : null))
const tooltip = computed(() => {
  const s = repo.value
  if (!s) return ''
  const lines = [
    `Branch: ${branchLabel(s)}${s.commit && s.branch ? ` (${s.commit})` : ''}`,
    s.upstream
      ? `Upstream: ${s.upstream}, ${s.ahead ?? 0} ahead, ${s.behind ?? 0} behind`
      : 'No upstream branch',
    `In this folder: ${changeSummary(s)}`,
    'Click to check again.',
  ]
  return lines.join('\n')
})
</script>

<template>
  <!-- Nothing to show when Git isn't installed or the folder isn't a repository. -->
  <button
    v-if="repo"
    type="button"
    class="flex min-w-0 items-center gap-1.5 hover:text-foreground"
    :title="tooltip"
    :aria-label="`Git: ${branchLabel(repo)}, ${changeSummary(repo)}. Check again`"
    @click="load"
  >
    <LoaderCircle v-if="loading" class="size-3.5 shrink-0 animate-spin" aria-hidden="true" />
    <GitBranch v-else class="size-3.5 shrink-0" aria-hidden="true" />
    <span class="truncate font-mono">{{ branchLabel(repo) }}</span>
    <span v-if="repo.ahead || repo.behind" class="tabular-nums">
      <template v-if="repo.ahead">↑{{ repo.ahead }}</template>
      <template v-if="repo.behind"> ↓{{ repo.behind }}</template>
    </span>
    <span
      :class="[
        'rounded-full px-1.5 tabular-nums',
        repo.conflicted
          ? 'bg-status-failed/15 text-status-failed'
          : changeCount(repo)
            ? 'bg-status-transition/15 text-status-transition'
            : 'bg-surface-muted',
      ]"
    >
      {{ changeCount(repo) ? `${changeCount(repo)} changed` : 'clean' }}
    </span>
  </button>
  <span
    v-else-if="status?.state === 'error'"
    class="flex items-center gap-1.5 text-status-failed"
    :title="status.message"
  >
    <CircleAlert class="size-3.5 shrink-0" aria-hidden="true" /> Git status unavailable
  </span>
</template>
