<script setup lang="ts">
import { ref } from 'vue'
import { X } from '@lucide/vue'

const tags = defineModel<string[]>({ required: true })
defineProps<{ id: string; suggestions?: string[] }>()
const draft = ref('')

function commit() {
  const parts = draft.value
    .split(',')
    .map((t) => t.trim().toLowerCase())
    .filter(Boolean)
  for (const tag of parts) if (!tags.value.includes(tag)) tags.value = [...tags.value, tag]
  draft.value = ''
}

function remove(tag: string) {
  tags.value = tags.value.filter((t) => t !== tag)
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' || event.key === ',') {
    event.preventDefault()
    commit()
  } else if (event.key === 'Backspace' && !draft.value && tags.value.length) {
    tags.value = tags.value.slice(0, -1)
  }
}
</script>

<template>
  <div
    class="flex flex-wrap items-center gap-1.5 rounded-md border bg-surface px-2 py-1.5 focus-within:outline-2 focus-within:outline-ring"
  >
    <span
      v-for="tag in tags"
      :key="tag"
      class="inline-flex items-center gap-1 rounded bg-surface-muted px-1.5 py-0.5 text-xs"
    >
      {{ tag }}
      <button
        type="button"
        class="text-muted hover:text-foreground"
        :aria-label="`Remove tag ${tag}`"
        @click="remove(tag)"
      >
        <X class="size-3" />
      </button>
    </span>
    <input
      :id="id"
      v-model="draft"
      type="text"
      class="min-w-24 flex-1 bg-transparent px-1 py-0.5 text-sm outline-none placeholder:text-muted/70"
      placeholder="Add tag…"
      :list="suggestions?.length ? `${id}-suggestions` : undefined"
      @keydown="onKeydown"
      @blur="commit"
    />
    <datalist v-if="suggestions?.length" :id="`${id}-suggestions`">
      <option v-for="s in suggestions" :key="s" :value="s" />
    </datalist>
  </div>
</template>
