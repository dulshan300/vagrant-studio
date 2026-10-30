<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Monitor, Moon, Sun } from '@lucide/vue'
import { RadioGroupItem, RadioGroupRoot } from 'reka-ui'
import PageHeader from '@/components/layout/PageHeader.vue'
import { Card, CardHeader } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import FolderInput from '@/components/ui/form/FolderInput.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import ProviderSelect from '@/components/providers/ProviderSelect.vue'
import { useProviderChoices } from '@/composables/useProviderChoices'
import { useFormErrors } from '@/composables/useFormErrors'
import { hasBackend } from '@/services/app'
import { editorApi } from '@/services/editor'
import { settingsApi } from '@/services/projects'
import { describeHost } from '@/components/host/host'
import { isThemePreference, type ThemePreference } from '@/app/theme'
import { useAppStore } from '@/stores/app'
import { useSettingsStore } from '@/stores/settings'
import type { DetectedEditor } from '@/types/app'

const settings = useSettingsStore()
const app = useAppStore()
const location = ref('')
const savedLocation = ref('')
const vagrantPath = ref('')
const savedVagrantPath = ref('')
const defaultProvider = ref<string | null>(null)
const savedDefaultProvider = ref<string | null>(null)
const providers = useProviderChoices()
const editors = ref<DetectedEditor[]>([])
/** '' = automatic, a detected editor's path, or 'custom'. */
const editorChoice = ref('')
const customEditor = ref('')
const savedEditorPath = ref<string | null>(null)
const editorPath = computed(() =>
  editorChoice.value === 'custom' ? customEditor.value.trim() || null : editorChoice.value || null,
)

function showEditor(path: string | null) {
  savedEditorPath.value = path
  if (!path) editorChoice.value = ''
  else if (editors.value.some((e) => e.path === path)) editorChoice.value = path
  else {
    editorChoice.value = 'custom'
    customEditor.value = path
  }
}
const locationErrors = useFormErrors()

onMounted(async () => {
  if (!app.loaded) void app.load()
  if (!hasBackend()) return
  try {
    const settings = await settingsApi.get()
    savedLocation.value = settings.defaultProjectLocation ?? ''
    location.value = savedLocation.value
    savedVagrantPath.value = settings.vagrantPath ?? ''
    vagrantPath.value = savedVagrantPath.value
    savedDefaultProvider.value = settings.defaultProvider
    defaultProvider.value = settings.defaultProvider
    editors.value = await editorApi.list().catch(() => [])
    showEditor(settings.editorPath)
  } catch (e) {
    locationErrors.set(e)
  }
})

async function saveLocation() {
  locationErrors.clear()
  try {
    const saved = await settingsApi.save({
      defaultProjectLocation: location.value.trim() || null,
      vagrantPath: vagrantPath.value.trim() || null,
      defaultProvider: defaultProvider.value,
      editorPath: editorPath.value,
    })
    savedLocation.value = saved.defaultProjectLocation ?? ''
    location.value = savedLocation.value
    savedVagrantPath.value = saved.vagrantPath ?? ''
    vagrantPath.value = savedVagrantPath.value
    savedDefaultProvider.value = saved.defaultProvider
    defaultProvider.value = saved.defaultProvider
    providers.preferred.value = saved.defaultProvider
    showEditor(saved.editorPath)
  } catch (e) {
    locationErrors.set(e)
  }
}

const themeOptions: { value: ThemePreference; label: string; icon: typeof Sun }[] = [
  { value: 'light', label: 'Light', icon: Sun },
  { value: 'dark', label: 'Dark', icon: Moon },
  { value: 'system', label: 'System', icon: Monitor },
]

function onThemeChange(value: unknown) {
  if (isThemePreference(value)) settings.setThemePreference(value)
}
</script>

<template>
  <PageHeader title="Settings" description="Application preferences and environment details." />

  <div class="space-y-5">
    <Card>
      <CardHeader title="Appearance" description="Stored on this computer only." />
      <div class="px-5 py-4">
        <RadioGroupRoot
          :model-value="settings.themePreference"
          aria-label="Theme"
          orientation="horizontal"
          class="inline-flex rounded-lg border bg-surface-muted p-1"
          @update:model-value="onThemeChange"
        >
          <RadioGroupItem
            v-for="option in themeOptions"
            :key="option.value"
            :value="option.value"
            class="flex items-center gap-2 rounded-md px-3 py-1.5 text-muted transition-colors hover:text-foreground data-[state=checked]:bg-surface data-[state=checked]:text-foreground data-[state=checked]:shadow-xs"
          >
            <component :is="option.icon" class="size-4" aria-hidden="true" />
            {{ option.label }}
          </RadioGroupItem>
        </RadioGroupRoot>
      </div>
    </Card>

    <Card v-if="hasBackend()">
      <CardHeader
        title="Projects"
        description="Stored in the application database on this computer."
      />
      <form class="space-y-1.5 px-5 py-4" @submit.prevent="saveLocation">
        <label for="settings-location" class="block text-xs font-medium"
          >Default location for new projects</label
        >
        <div class="flex gap-2">
          <div class="flex-1"><FolderInput id="settings-location" v-model="location" /></div>
        </div>
        <label for="settings-vagrant" class="mt-4 block text-xs font-medium"
          >Vagrant executable</label
        >
        <TextInput
          id="settings-vagrant"
          v-model="vagrantPath"
          class="font-mono text-xs"
          placeholder="Leave empty to find Vagrant automatically"
        />
        <p
          v-if="locationErrors.field.value.vagrantPath"
          class="text-xs text-status-failed"
          role="alert"
        >
          {{ locationErrors.field.value.vagrantPath }}
        </p>
        <label for="settings-provider" class="mt-4 block text-xs font-medium"
          >Default provider</label
        >
        <ProviderSelect
          id="settings-provider"
          v-model="defaultProvider"
          :choices="providers.choices.value"
          allow-unset
        />
        <p class="text-xs text-muted">
          Pre-selected when you create a project or add an environment. Each environment can use a
          different provider.
        </p>
        <p
          v-if="locationErrors.field.value.provider"
          class="text-xs text-status-failed"
          role="alert"
        >
          {{ locationErrors.field.value.provider }}
        </p>
        <label for="settings-editor" class="mt-4 block text-xs font-medium">Code editor</label>
        <select
          id="settings-editor"
          v-model="editorChoice"
          class="rounded-md border bg-surface px-2 py-1.5 text-sm"
        >
          <option value="">
            Automatic{{ editors.length ? ` (${editors[0]!.name})` : ' (none detected)' }}
          </option>
          <option v-for="e in editors" :key="e.path" :value="e.path">{{ e.name }}</option>
          <option value="custom">Other program…</option>
        </select>
        <TextInput
          v-if="editorChoice === 'custom'"
          id="settings-editor-path"
          v-model="customEditor"
          aria-label="Editor program"
          class="font-mono text-xs"
          placeholder="Full path to the editor's program, e.g. C:\Program Files\…\editor.exe"
        />
        <p class="text-xs text-muted">
          Used by Open in editor on project pages. The folder is passed as the only argument.
        </p>
        <p
          v-if="locationErrors.field.value.editorPath"
          class="text-xs text-status-failed"
          role="alert"
        >
          {{ locationErrors.field.value.editorPath }}
        </p>
        <div class="flex justify-end pt-2">
          <Button
            type="submit"
            :disabled="
              location === savedLocation &&
              vagrantPath === savedVagrantPath &&
              defaultProvider === savedDefaultProvider &&
              editorPath === savedEditorPath
            "
          >
            Save
          </Button>
        </div>
        <p
          v-if="locationErrors.field.value.defaultProjectLocation || locationErrors.general.value"
          class="text-xs text-status-failed"
          role="alert"
        >
          {{ locationErrors.field.value.defaultProjectLocation ?? locationErrors.general.value }}
        </p>
      </form>
    </Card>

    <Card>
      <CardHeader title="About" />
      <dl class="grid grid-cols-[auto_1fr] gap-x-8 gap-y-3 px-5 py-4">
        <template v-if="app.info">
          <dt class="text-muted">Version</dt>
          <dd class="tabular-nums">{{ app.info.appVersion }}</dd>
          <dt class="text-muted">Tauri</dt>
          <dd class="tabular-nums">{{ app.info.tauriVersion }}</dd>
          <dt class="text-muted">Host</dt>
          <dd>{{ describeHost(app.info.host) }}</dd>
          <dt class="text-muted">Data folder</dt>
          <dd>
            <code v-if="app.info.dataDir" class="font-mono text-xs break-all">{{
              app.info.dataDir
            }}</code>
            <span v-else class="text-muted">Unavailable</span>
          </dd>
        </template>
        <template v-else>
          <dt class="text-muted">Backend</dt>
          <dd class="text-muted">
            {{ app.error ?? (app.loaded ? 'Running outside the desktop shell' : 'Loading…') }}
          </dd>
        </template>
      </dl>
    </Card>
  </div>
</template>
