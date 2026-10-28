<script setup lang="ts">
import { onMounted } from 'vue'
import { Monitor, Moon, Sun } from '@lucide/vue'
import { RadioGroupItem, RadioGroupRoot } from 'reka-ui'
import PageHeader from '@/components/layout/PageHeader.vue'
import { Card, CardHeader } from '@/components/ui/card'
import { describeHost } from '@/components/host/host'
import { isThemePreference, type ThemePreference } from '@/app/theme'
import { useAppStore } from '@/stores/app'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const app = useAppStore()
onMounted(() => {
  if (!app.loaded) void app.load()
})

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
