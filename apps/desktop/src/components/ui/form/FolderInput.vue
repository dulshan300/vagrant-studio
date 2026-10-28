<script setup lang="ts">
import { FolderOpen } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import TextInput from './TextInput.vue'
import { pickFolder } from '@/services/projects'

const model = defineModel<string>({ required: true })
defineProps<{ id: string; invalid?: boolean }>()
const emit = defineEmits<{ picked: [path: string] }>()

async function browse() {
  const path = await pickFolder(model.value || null)
  if (path) {
    model.value = path
    emit('picked', path)
  }
}
</script>

<template>
  <div class="flex gap-2">
    <TextInput :id="id" v-model="model" :invalid="invalid" class="font-mono text-xs" />
    <Button variant="secondary" type="button" @click="browse">
      <FolderOpen aria-hidden="true" />
      Browse…
    </Button>
  </div>
</template>
