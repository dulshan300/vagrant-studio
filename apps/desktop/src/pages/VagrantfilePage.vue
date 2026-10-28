<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRoute } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { AlertTriangle, ChevronLeft, Code, History, LayoutList, Save } from '@lucide/vue'
import CodeEditor from '@/components/editor/CodeEditor.vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import SaveVagrantfileDialog from '@/components/vagrantfile/SaveVagrantfileDialog.vue'
import VagrantfileBackupsDialog from '@/components/vagrantfile/VagrantfileBackupsDialog.vue'
import VisualEditor from '@/components/vagrantfile/VisualEditor.vue'
import { cloneModel, errorsByPath, sameModel, starterModel } from '@/lib/vagrantfile'
import { cn } from '@/lib/utils'
import { hasBackend } from '@/services/app'
import { errorMessage } from '@/services/errors'
import { projectsApi } from '@/services/projects'
import { vagrantfileApi } from '@/services/vagrantfile'
import { useOperationsStore } from '@/stores/operations'
import type { Environment, Project } from '@/types/projects'
import type {
  Analysis,
  FieldError,
  SaveVagrantfileOutcome,
  VagrantConfig,
  VagrantfileBackup,
  VagrantfileDocument,
} from '@/types/vagrantfile'

const route = useRoute()
const operations = useOperationsStore()

const project = ref<Project | null>(null)
const environment = ref<Environment | null>(null)
const doc = ref<VagrantfileDocument | null>(null)
const loadError = ref<string | null>(null)

/** The text that Save writes. Visual edits regenerate it; raw edits change it directly. */
const draft = ref('')
const draftAnalysis = ref<Analysis | null>(null)
const tab = ref<'visual' | 'raw'>('visual')
/** The form model; null when the draft is outside the supported subset. */
const model = ref<VagrantConfig | null>(null)
/** The model that `draft` currently represents, and its problems. */
let baseline: VagrantConfig | null = null
let baselineErrors: FieldError[] = []
const modelErrors = ref<FieldError[]>([])
const generating = ref(false)

const stale = ref(false)
const notice = ref<string | null>(null)
const actionError = ref<string | null>(null)
const showSave = ref(false)
const showBackups = ref(false)
const rawEditor = ref<InstanceType<typeof CodeEditor>>()

const dirty = computed(() => {
  if (!doc.value) return false
  return doc.value.exists ? draft.value !== doc.value.content : draft.value !== ''
})
const errorMap = computed(() => errorsByPath(modelErrors.value))
const canSave = computed(
  () =>
    dirty.value &&
    !generating.value &&
    !(tab.value === 'visual' && (modelErrors.value.length > 0 || !model.value)),
)

const fileState = computed(() => {
  const a = doc.value?.analysis
  if (!doc.value?.exists || !a) return null
  if (a.managed && !a.modifiedOutside) return 'Written by the visual editor'
  if (a.managed) return 'Written by the visual editor, edited by hand since'
  if (a.model) return 'Hand-written; the visual editor can show it'
  return 'Hand-written; raw editing only'
})

async function load() {
  const projectId = route.params.id as string
  const environmentId = route.params.environmentId as string
  loadError.value = null
  try {
    const details = await projectsApi.get(projectId)
    project.value = details.project
    environment.value = details.environments.find((e) => e.id === environmentId) ?? null
    if (!environment.value) throw new Error('This environment no longer exists.')
    await loadDocument()
  } catch (e) {
    loadError.value = errorMessage(e)
  }
}

async function loadDocument() {
  const fresh = await vagrantfileApi.get(environment.value!.id)
  doc.value = fresh
  stale.value = false
  notice.value = null
  actionError.value = null
  if (fresh.exists) {
    draft.value = fresh.content
    draftAnalysis.value = fresh.analysis
    showModel(fresh.analysis)
    tab.value = fresh.analysis.model ? 'visual' : 'raw'
  } else {
    draft.value = ''
    draftAnalysis.value = null
    model.value = starterModel(environment.value!.defaultProvider)
    baseline = null
    tab.value = 'visual'
    await regenerate()
  }
}

function showModel(analysis: Analysis) {
  model.value = analysis.model ? cloneModel(analysis.model) : null
  baseline = analysis.model ? cloneModel(analysis.model) : null
  baselineErrors = analysis.errors
  modelErrors.value = analysis.errors
}

// --- visual edits → generated draft ---------------------------------------

let generateTimer: ReturnType<typeof setTimeout> | undefined
let generateSeq = 0

async function regenerate() {
  clearTimeout(generateTimer)
  if (!model.value || !doc.value) return
  const snapshot = cloneModel(model.value)
  const seq = ++generateSeq
  generating.value = true
  try {
    const result = await vagrantfileApi.generate(snapshot, doc.value.analysis.lineEnding)
    if (seq !== generateSeq) return
    modelErrors.value = result.errors
    if (result.content !== null) {
      draft.value = result.content
      baseline = snapshot
      baselineErrors = []
    }
  } catch (e) {
    actionError.value = errorMessage(e)
  } finally {
    if (seq === generateSeq) generating.value = false
  }
}

watch(
  model,
  (m) => {
    if (tab.value !== 'visual' || !m) return
    if (baseline && sameModel(m, baseline)) {
      clearTimeout(generateTimer)
      generateSeq++
      generating.value = false
      modelErrors.value = baselineErrors
      return
    }
    generating.value = true
    clearTimeout(generateTimer)
    generateTimer = setTimeout(() => void regenerate(), 200)
  },
  { deep: true },
)

// --- draft → analysis (status line, save checks) ----------------------------

let analyzeTimer: ReturnType<typeof setTimeout> | undefined
let analyzeSeq = 0

async function analyzeDraft(): Promise<Analysis | null> {
  clearTimeout(analyzeTimer)
  const seq = ++analyzeSeq
  try {
    const a = await vagrantfileApi.analyze(draft.value)
    if (seq === analyzeSeq) draftAnalysis.value = a
    return a
  } catch (e) {
    actionError.value = errorMessage(e)
    return null
  }
}

watch(draft, () => {
  clearTimeout(analyzeTimer)
  analyzeTimer = setTimeout(() => void analyzeDraft(), 300)
})

// --- tabs -------------------------------------------------------------------

async function toVisual() {
  if (tab.value === 'visual') return
  const a = await analyzeDraft()
  if (!a) return
  showModel(a)
  tab.value = 'visual'
}

async function toRaw() {
  if (tab.value === 'raw') return
  if (modelErrors.value.length) {
    actionError.value = 'Fix the problems marked in the form before switching to raw editing.'
    return
  }
  if (generating.value) await regenerate()
  actionError.value = null
  tab.value = 'raw'
}

async function goToLine(line: number) {
  await toRaw()
  requestAnimationFrame(() => rawEditor.value?.revealLine(line))
}

// --- saving -------------------------------------------------------------------

async function openSave() {
  if (!canSave.value) return
  if (generating.value) await regenerate()
  if (!(await analyzeDraft())) return
  showSave.value = true
}

const replacesUnmanaged = computed(
  () => !!doc.value?.exists && !doc.value.analysis.managed && !!draftAnalysis.value?.managed,
)
const saveWarnings = computed(() =>
  doc.value?.exists && draftAnalysis.value?.managed ? doc.value.analysis.warnings : [],
)

async function onSaved(outcome: SaveVagrantfileOutcome, validate: boolean) {
  const wasMissing = !doc.value?.exists
  doc.value = outcome.document
  draft.value = outcome.document.content
  draftAnalysis.value = outcome.document.analysis
  if (tab.value === 'visual') showModel(outcome.document.analysis)
  notice.value = outcome.backup
    ? 'Saved. The previous version was backed up.'
    : wasMissing
      ? 'Vagrantfile created.'
      : 'Saved.'
  if (wasMissing) {
    const details = await projectsApi.get(project.value!.id).catch(() => null)
    environment.value = details?.environments.find((e) => e.id === environment.value!.id) ?? null
  }
  if (validate) {
    try {
      const op = await operations.validate(outcome.document.environmentId)
      await operations.open(op.id)
    } catch (e) {
      actionError.value = `Saved, but validation didn't start: ${errorMessage(e)}`
    }
  }
}

/** Takes the newer file on disk as the base while keeping the user's edits. */
async function keepMine() {
  try {
    doc.value = await vagrantfileApi.get(environment.value!.id)
    stale.value = false
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}

function restoreBackup(content: string, backup: VagrantfileBackup) {
  tab.value = 'raw'
  draft.value = content
  notice.value = `Loaded the backup from ${new Date(backup.createdAt).toLocaleString()}. Save to restore it.`
}

// --- unsaved changes ------------------------------------------------------

const showDiscard = ref(false)
let resolveDiscard: ((discard: boolean) => void) | null = null

function confirmDiscard(): Promise<boolean> {
  showDiscard.value = true
  return new Promise((resolve) => (resolveDiscard = resolve))
}

function answerDiscard(discard: boolean) {
  showDiscard.value = false
  resolveDiscard?.(discard)
  resolveDiscard = null
}

watch(showDiscard, (open) => {
  if (!open) answerDiscard(false)
})

onBeforeRouteLeave(async () => !dirty.value || (await confirmDiscard()))
onBeforeRouteUpdate(async (to, from) => {
  if (to.params.environmentId === from.params.environmentId) return true
  if (dirty.value && !(await confirmDiscard())) return false
  return true
})
watch(() => route.params.environmentId, load)

let unlistenClose: (() => void) | null = null

function onKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 's') {
    event.preventDefault()
    void openSave()
  }
}

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  await load()
  if (hasBackend()) {
    const appWindow = getCurrentWindow()
    unlistenClose = await appWindow.onCloseRequested(async (event) => {
      if (!dirty.value) return
      event.preventDefault()
      if (await confirmDiscard()) await appWindow.destroy()
    })
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  unlistenClose?.()
  clearTimeout(generateTimer)
  clearTimeout(analyzeTimer)
})

const tabClass = (active: boolean) =>
  cn(
    'inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm',
    active ? 'bg-surface shadow-sm font-medium' : 'text-muted hover:text-foreground',
  )
</script>

<template>
  <div>
    <RouterLink
      :to="`/projects/${route.params.id}`"
      class="mb-4 inline-flex items-center gap-1 text-sm text-muted hover:text-foreground"
    >
      <ChevronLeft class="size-4" /> {{ project?.name ?? 'Project' }}
    </RouterLink>

    <p v-if="loadError" class="text-status-failed" role="alert">{{ loadError }}</p>

    <template v-else-if="doc && environment">
      <header class="flex flex-wrap items-end justify-between gap-4 pb-4">
        <div class="min-w-0">
          <h1 class="text-xl font-semibold tracking-tight">
            {{ doc.exists ? 'Vagrantfile' : 'New Vagrantfile' }}
            <span class="font-normal text-muted">· {{ environment.name }}</span>
          </h1>
          <code class="mt-1 block truncate font-mono text-xs text-muted" :title="doc.path">{{
            doc.path
          }}</code>
          <p class="mt-1 flex flex-wrap items-center gap-2 text-xs">
            <span v-if="fileState" class="rounded bg-surface-muted px-1.5 py-0.5">{{
              fileState
            }}</span>
            <span
              v-if="dirty"
              class="rounded bg-status-transition/15 px-1.5 py-0.5 text-status-transition"
              >Unsaved changes</span
            >
          </p>
        </div>
        <div class="flex items-center gap-2">
          <Button variant="ghost" size="sm" @click="showBackups = true">
            <History /> Backups
          </Button>
          <Button :disabled="!canSave" title="Ctrl/⌘+S" @click="openSave">
            <Save /> {{ doc.exists ? 'Review and save' : 'Review and create' }}
          </Button>
        </div>
      </header>

      <div
        v-if="stale"
        class="mb-4 flex flex-wrap items-center gap-3 rounded-md border border-status-transition/50 bg-status-transition/10 px-4 py-3 text-sm"
        role="alert"
      >
        <AlertTriangle class="size-4 text-status-transition" aria-hidden="true" />
        <span class="flex-1">
          The Vagrantfile changed on disk after you opened it, so nothing was saved.
        </span>
        <Button variant="secondary" size="sm" @click="loadDocument">
          Reload, discarding my changes
        </Button>
        <Button size="sm" @click="keepMine">Keep my changes and compare again</Button>
      </div>
      <p v-if="notice" class="mb-4 text-sm text-status-running" role="status">{{ notice }}</p>
      <p v-if="actionError" class="mb-4 text-sm text-status-failed" role="alert">
        {{ actionError }}
      </p>

      <div class="mb-3 inline-flex rounded-lg bg-surface-muted p-1" role="tablist">
        <button
          type="button"
          role="tab"
          :aria-selected="tab === 'visual'"
          :class="tabClass(tab === 'visual')"
          @click="toVisual"
        >
          <LayoutList class="size-4" /> Visual
        </button>
        <button
          type="button"
          role="tab"
          :aria-selected="tab === 'raw'"
          :class="tabClass(tab === 'raw')"
          @click="toRaw"
        >
          <Code class="size-4" /> Raw
        </button>
      </div>

      <Card v-if="tab === 'visual'" class="p-5">
        <VisualEditor v-if="model" v-model="model" :errors="errorMap" />
        <div v-else class="max-w-prose space-y-3 text-sm">
          <h2 class="font-medium">The visual editor can't show this Vagrantfile</h2>
          <p>
            Vagrantfiles are Ruby, and the visual editor only handles a limited set of settings so
            that it never rewrites code it doesn't understand. Raw editing and all machine actions
            still work.
          </p>
          <p v-if="draftAnalysis?.unsupported" class="rounded-md bg-surface-muted p-3">
            <button
              type="button"
              class="font-medium underline-offset-2 hover:underline"
              @click="goToLine(draftAnalysis.unsupported.line)"
            >
              Line {{ draftAnalysis.unsupported.line }}</button
            >: {{ draftAnalysis.unsupported.message }}
          </p>
          <Button variant="secondary" size="sm" @click="toRaw">Open the raw editor</Button>
        </div>
        <ul
          v-if="model && modelErrors.some((e) => e.path === '')"
          class="mt-4 text-sm text-status-failed"
        >
          <li v-for="e in modelErrors.filter((e) => e.path === '')" :key="e.message" role="alert">
            {{ e.message }}
          </li>
        </ul>
      </Card>

      <template v-else>
        <Card class="h-[62vh] overflow-hidden">
          <CodeEditor ref="rawEditor" v-model="draft" label="Vagrantfile source" />
        </Card>
        <p class="mt-2 text-xs text-muted">
          <template v-if="draftAnalysis?.model">The visual editor can show this text.</template>
          <template v-else-if="draftAnalysis?.unsupported">
            Visual editing unavailable:
            <button
              type="button"
              class="underline underline-offset-2 hover:text-foreground"
              @click="rawEditor?.revealLine(draftAnalysis.unsupported.line)"
            >
              line {{ draftAnalysis.unsupported.line }}</button
            >: {{ draftAnalysis.unsupported.message }}
          </template>
          · Ctrl/⌘+F to search and replace.
        </p>
      </template>

      <SaveVagrantfileDialog
        v-model:open="showSave"
        :environment-id="environment.id"
        :path="doc.path"
        :original="doc.content"
        :proposed="draft"
        :exists="doc.exists"
        :base-revision="doc.revision"
        :replaces-unmanaged="replacesUnmanaged"
        :warnings="saveWarnings"
        @saved="onSaved"
        @stale="stale = true"
      />
      <VagrantfileBackupsDialog
        v-model:open="showBackups"
        :environment-id="environment.id"
        @restore="restoreBackup"
      />
    </template>

    <AppDialog v-model:open="showDiscard" title="Discard unsaved changes?">
      <p class="text-sm">Your changes to the Vagrantfile haven't been saved and will be lost.</p>
      <template #footer>
        <Button variant="secondary" @click="answerDiscard(false)">Keep editing</Button>
        <Button variant="destructive" @click="answerDiscard(true)">Discard changes</Button>
      </template>
    </AppDialog>
  </div>
</template>
