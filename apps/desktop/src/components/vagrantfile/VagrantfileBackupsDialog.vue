<script setup lang="ts">
import { ref, watch } from 'vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import { Button } from '@/components/ui/button'
import { relativeTime } from '@/lib/projects'
import { errorMessage } from '@/services/errors'
import { vagrantfileApi } from '@/services/vagrantfile'
import type { VagrantfileBackup } from '@/types/vagrantfile'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ environmentId: string }>()
const emit = defineEmits<{ restore: [content: string, backup: VagrantfileBackup] }>()

const backups = ref<VagrantfileBackup[] | null>(null)
const error = ref<string | null>(null)

watch(open, async (isOpen) => {
  if (!isOpen) return
  backups.value = null
  error.value = null
  try {
    backups.value = await vagrantfileApi.backups(props.environmentId)
  } catch (e) {
    error.value = errorMessage(e)
  }
})

async function restore(backup: VagrantfileBackup) {
  try {
    emit('restore', await vagrantfileApi.readBackup(props.environmentId, backup.name), backup)
    open.value = false
  } catch (e) {
    error.value = errorMessage(e)
  }
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    title="Vagrantfile backups"
    description="Saved automatically before each change. Loading one puts it in the raw editor; nothing changes on disk until you save."
  >
    <p v-if="backups && !backups.length" class="text-sm text-muted">No backups yet.</p>
    <ul v-else-if="backups" class="divide-y rounded-md border">
      <li
        v-for="b in backups"
        :key="b.name"
        class="flex items-center justify-between gap-3 px-3 py-2"
      >
        <div class="min-w-0">
          <div class="text-sm" :title="b.path">{{ new Date(b.createdAt).toLocaleString() }}</div>
          <div class="text-xs text-muted">
            {{ relativeTime(b.createdAt) }} · {{ (b.size / 1024).toFixed(1) }} KB
          </div>
        </div>
        <Button variant="secondary" size="sm" @click="restore(b)">Load into editor</Button>
      </li>
    </ul>
    <p v-if="error" class="mt-3 text-sm text-status-failed" role="alert">{{ error }}</p>
  </AppDialog>
</template>
