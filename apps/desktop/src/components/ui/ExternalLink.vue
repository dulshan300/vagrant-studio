<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener'
import { hasBackend } from '@/services/app'

const props = defineProps<{ href: string }>()

// In the desktop shell, links open in the system browser (allowed URLs are listed in
// src-tauri/capabilities/default.json). In a plain browser they behave normally.
function onClick(event: MouseEvent) {
  if (!hasBackend()) return
  event.preventDefault()
  void openUrl(props.href)
}
</script>

<template>
  <a :href="href" target="_blank" rel="noreferrer" @click="onClick"><slot /></a>
</template>
