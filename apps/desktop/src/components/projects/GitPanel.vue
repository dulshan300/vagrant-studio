<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { GitBranch, GitCommitHorizontal, History, LoaderCircle, RefreshCw } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardHeader } from '@/components/ui/card'
import {
  branchLabel,
  changeLabel,
  displayPath,
  folderInRepo,
  stageState,
  type StageState,
} from '@/lib/git'
import { relativeTime } from '@/lib/projects'
import { errorMessage } from '@/services/errors'
import { gitApi } from '@/services/git'
import type { CommitSummary, FileChange, GitStatus } from '@/types/git'

const props = defineProps<{
  projectId: string
  projectRoot: string
  status: GitStatus | null
  loading: boolean
}>()
const emit = defineEmits<{ update: [status: GitStatus]; refresh: [] }>()

const history = ref<CommitSummary[]>([])
const message = ref('')
const busy = ref(false)
const error = ref<string | null>(null)
const notice = ref<string | null>(null)

const repo = computed(() => (props.status?.state === 'repository' ? props.status : null))
const prefix = computed(() => folderInRepo(repo.value?.root ?? null, props.projectRoot))
const files = computed(() => repo.value?.files ?? [])
const stagedFiles = computed(() => files.value.filter((f) => stageState(f) !== 'unstaged'))
const unstagedFiles = computed(() => files.value.filter((f) => stageState(f) !== 'staged'))

async function loadHistory() {
  if (!repo.value) {
    history.value = []
    return
  }
  try {
    history.value = await gitApi.log(props.projectId)
  } catch (e) {
    error.value = errorMessage(e)
  }
}
// Reload history when the commit changes (after a commit, or after a refresh).
watch(
  () => (repo.value ? `${repo.value.commit}` : null),
  () => void loadHistory(),
  { immediate: true },
)

async function act(action: () => Promise<GitStatus>) {
  busy.value = true
  error.value = null
  notice.value = null
  try {
    emit('update', await action())
  } catch (e) {
    error.value = errorMessage(e)
  } finally {
    busy.value = false
  }
}

const stage = (paths: string[]) => act(() => gitApi.stage(props.projectId, paths))
const unstage = (paths: string[]) => act(() => gitApi.unstage(props.projectId, paths))

function toggle(f: FileChange, state: StageState) {
  // A partly staged file is staged fully on the first click.
  void (state === 'staged' ? unstage([f.path]) : stage([f.path]))
}

async function commit() {
  await act(async () => {
    const status = await gitApi.commit(props.projectId, message.value)
    message.value = ''
    return status
  })
}

async function initialize() {
  if (
    !window.confirm(
      `Create a Git repository in ${props.projectRoot}?\n\nThis adds a .git folder and, unless one exists, a .gitignore that leaves out .vagrant/ (machine state and SSH keys), logs and boxes. Nothing is committed.`,
    )
  )
    return
  await act(async () => {
    const result = await gitApi.init(props.projectId)
    notice.value = result.gitignoreCreated
      ? 'Repository created with a .gitignore for Vagrant.'
      : 'Repository created. The existing .gitignore was kept.'
    return result.status
  })
}
</script>

<template>
  <Card>
    <CardHeader
      title="Version control"
      :description="
        repo
          ? `${branchLabel(repo)}${repo.upstream ? ` · tracking ${repo.upstream}` : ''}${
              repo.ahead ? ` · ${repo.ahead} to push` : ''
            }${repo.behind ? ` · ${repo.behind} to pull` : ''}`
          : 'Git for this project folder.'
      "
    >
      <template #actions>
        <Button
          variant="ghost"
          size="sm"
          :disabled="loading || busy"
          aria-label="Refresh Git status"
          @click="emit('refresh')"
        >
          <RefreshCw :class="loading ? 'animate-spin' : ''" /> Refresh
        </Button>
      </template>
    </CardHeader>

    <p v-if="!status" class="flex items-center gap-2 px-5 py-4 text-xs text-muted">
      <LoaderCircle class="size-4 animate-spin" aria-hidden="true" /> Checking Git…
    </p>
    <p v-else-if="status.state === 'unavailable'" class="px-5 py-4 text-xs text-muted">
      Git isn't installed, or wasn't found. Install Git (on Windows, Git for Windows) to use version
      control here.
    </p>
    <div
      v-else-if="status.state === 'notRepository'"
      class="flex flex-wrap items-center justify-between gap-3 px-5 py-4"
    >
      <p class="text-xs text-muted">
        This folder isn't a Git repository. Initializing adds a .gitignore for Vagrant; nothing is
        committed.
      </p>
      <Button variant="secondary" size="sm" :disabled="busy" @click="initialize">
        <GitBranch /> Initialize Git
      </Button>
    </div>
    <p v-else-if="status.state === 'error'" class="px-5 py-4 text-xs text-status-failed">
      {{ status.message }}
    </p>

    <div v-else-if="repo" class="grid md:grid-cols-[minmax(0,3fr)_minmax(0,2fr)] md:divide-x">
      <section aria-label="Changes" class="min-w-0">
        <div class="flex items-center justify-between gap-2 border-b px-5 py-2 text-xs">
          <span class="font-medium">
            Changes <span class="text-muted">({{ files.length }})</span>
          </span>
          <span v-if="files.length" class="flex gap-1">
            <Button
              variant="ghost"
              size="sm"
              class="h-6 px-2"
              :disabled="busy || !unstagedFiles.length"
              @click="stage(unstagedFiles.map((f) => f.path))"
            >
              Stage all
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 px-2"
              :disabled="busy || !stagedFiles.length"
              @click="unstage(stagedFiles.map((f) => f.path))"
            >
              Unstage all
            </Button>
          </span>
        </div>
        <p v-if="!files.length" class="px-5 py-4 text-xs text-muted">No changes in this folder.</p>
        <ul v-else class="max-h-72 divide-y overflow-y-auto">
          <li
            v-for="f in files"
            :key="f.path"
            class="flex items-center gap-2 px-5 py-1.5"
            :title="`${changeLabel(f).text}${f.origPath ? ` from ${f.origPath}` : ''}: ${f.path}`"
          >
            <input
              type="checkbox"
              :checked="stageState(f) === 'staged'"
              :indeterminate="stageState(f) === 'partly'"
              :disabled="busy"
              :aria-label="`${stageState(f) === 'staged' ? 'Unstage' : 'Stage'} ${f.path}`"
              @change="toggle(f, stageState(f))"
            />
            <span
              :class="[
                'w-4 shrink-0 text-center font-mono text-[11px] font-semibold',
                f.kind === 'conflicted'
                  ? 'text-status-failed'
                  : f.kind === 'untracked'
                    ? 'text-status-running'
                    : 'text-status-transition',
              ]"
              aria-hidden="true"
              >{{ changeLabel(f).letter }}</span
            >
            <span class="min-w-0 flex-1 truncate font-mono text-xs">{{
              displayPath(f.path, prefix)
            }}</span>
          </li>
        </ul>

        <form class="space-y-2 border-t px-5 py-3" @submit.prevent="commit">
          <label for="commit-message" class="sr-only">Commit message</label>
          <textarea
            id="commit-message"
            v-model="message"
            rows="2"
            maxlength="10000"
            placeholder="Commit message"
            class="w-full rounded-md border bg-surface px-3 py-2 text-sm placeholder:text-muted/70"
            @keydown.ctrl.enter.prevent="commit"
          />
          <div class="flex flex-wrap items-center justify-between gap-2">
            <p class="text-[11px] text-muted">
              <template v-if="prefix && repo.root">
                Repository root: {{ repo.root }}. A commit includes everything staged there.
              </template>
              <template v-else
                >Ctrl+Enter to commit. Your Git name, email and hooks apply.</template
              >
            </p>
            <Button type="submit" size="sm" :disabled="busy || !repo.staged || !message.trim()">
              <GitCommitHorizontal />
              {{
                repo.staged
                  ? `Commit ${repo.staged} ${repo.staged === 1 ? 'change' : 'changes'}`
                  : 'Commit'
              }}
            </Button>
          </div>
        </form>
      </section>

      <section aria-label="History" class="min-w-0 border-t md:border-t-0">
        <div class="flex items-center gap-1.5 border-b px-5 py-2 text-xs font-medium">
          <History class="size-3.5" aria-hidden="true" /> History
        </div>
        <p v-if="!history.length" class="px-5 py-4 text-xs text-muted">No commits yet.</p>
        <ul v-else class="max-h-80 divide-y overflow-y-auto">
          <li v-for="c in history" :key="c.id" class="px-5 py-2">
            <div class="truncate text-sm" :title="c.subject">{{ c.subject }}</div>
            <div class="flex gap-2 text-[11px] text-muted">
              <span class="font-mono">{{ c.id }}</span>
              <span class="truncate">{{ c.author }}</span>
              <span class="ml-auto shrink-0">{{ relativeTime(c.date) }}</span>
            </div>
          </li>
        </ul>
      </section>
    </div>

    <p v-if="notice" class="border-t px-5 py-2 text-xs text-muted" role="status">{{ notice }}</p>
    <p v-if="error" class="border-t px-5 py-2 text-xs text-status-failed" role="alert">
      {{ error }}
    </p>
  </Card>
</template>
