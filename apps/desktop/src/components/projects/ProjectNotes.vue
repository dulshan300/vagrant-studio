<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { NotebookPen, Plus, Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardHeader } from '@/components/ui/card'
import { useFormErrors } from '@/composables/useFormErrors'
import { relativeTime } from '@/lib/projects'
import { hasBackend } from '@/services/app'
import { notesApi } from '@/services/workspace'
import type { ProjectNote } from '@/types/workspace'

const props = defineProps<{ projectId: string }>()

const notes = ref<ProjectNote[]>([])
/** The note being edited: an id, 'new', or null. */
const editing = ref<string | null>(null)
const draft = ref({ title: '', body: '' })
const saving = ref(false)
const errors = useFormErrors()
const titleInput = ref<HTMLInputElement | null>(null)

const shown = computed(() => notes.value.filter((n) => n.id !== editing.value))
const dirty = computed(() => {
  const note = notes.value.find((n) => n.id === editing.value)
  return note
    ? draft.value.title !== note.title || draft.value.body !== note.body
    : !!(draft.value.title.trim() || draft.value.body.trim())
})

async function load() {
  if (!hasBackend()) return
  errors.clear()
  try {
    notes.value = await notesApi.list(props.projectId)
  } catch (e) {
    errors.set(e)
  }
}
watch(() => props.projectId, load, { immediate: true })

async function edit(note: ProjectNote | null) {
  if (editing.value && dirty.value && !window.confirm('Discard the changes to this note?')) return
  errors.clear()
  editing.value = note?.id ?? 'new'
  draft.value = { title: note?.title ?? '', body: note?.body ?? '' }
  await nextTick()
  titleInput.value?.focus()
}

async function save() {
  saving.value = true
  errors.clear()
  try {
    const { title, body } = draft.value
    const saved =
      editing.value === 'new'
        ? await notesApi.create(props.projectId, title, body)
        : await notesApi.update(props.projectId, editing.value!, title, body)
    notes.value = [saved, ...notes.value.filter((n) => n.id !== saved.id)]
    editing.value = null
  } catch (e) {
    errors.set(e)
  } finally {
    saving.value = false
  }
}

async function remove(note: ProjectNote) {
  if (!window.confirm(`Delete the note “${note.title}”?`)) return
  errors.clear()
  try {
    await notesApi.remove(props.projectId, note.id)
    notes.value = notes.value.filter((n) => n.id !== note.id)
    if (editing.value === note.id) editing.value = null
  } catch (e) {
    errors.set(e)
  }
}
</script>

<template>
  <Card>
    <CardHeader title="Notes" description="Plain-text notes kept with this project in the app.">
      <template #actions>
        <Button variant="secondary" size="sm" :disabled="editing === 'new'" @click="edit(null)">
          <Plus /> New note
        </Button>
      </template>
    </CardHeader>

    <form v-if="editing" class="space-y-2 border-b px-5 py-3" @submit.prevent="save">
      <label for="note-title" class="sr-only">Title</label>
      <input
        id="note-title"
        ref="titleInput"
        v-model="draft.title"
        type="text"
        maxlength="200"
        placeholder="Title"
        :aria-invalid="!!errors.field.value.title"
        :class="[
          'w-full rounded-md border bg-surface px-3 py-1.5 text-sm font-medium placeholder:text-muted/70',
          errors.field.value.title && 'border-status-failed',
        ]"
      />
      <p v-if="errors.field.value.title" class="text-xs text-status-failed" role="alert">
        {{ errors.field.value.title }}
      </p>
      <label for="note-body" class="sr-only">Note</label>
      <textarea
        id="note-body"
        v-model="draft.body"
        rows="6"
        placeholder="Write anything: logins, URLs, how to reset the database…"
        class="w-full rounded-md border bg-surface px-3 py-2 font-mono text-xs placeholder:font-sans placeholder:text-muted/70"
      />
      <p v-if="errors.field.value.body" class="text-xs text-status-failed" role="alert">
        {{ errors.field.value.body }}
      </p>
      <div class="flex justify-end gap-2">
        <Button type="button" variant="ghost" size="sm" @click="editing = null">Cancel</Button>
        <Button type="submit" size="sm" :disabled="saving || !draft.title.trim()">
          {{ saving ? 'Saving…' : 'Save note' }}
        </Button>
      </div>
    </form>

    <p
      v-if="!notes.length && !editing"
      class="flex items-center gap-2 px-5 py-4 text-xs text-muted"
    >
      <NotebookPen class="size-4" aria-hidden="true" /> No notes yet.
    </p>
    <ul v-if="shown.length" class="divide-y">
      <li v-for="note in shown" :key="note.id" class="group relative px-5 py-2.5">
        <button
          type="button"
          class="block w-full min-w-0 pr-8 text-left"
          :title="`Edit “${note.title}”`"
          @click="edit(note)"
        >
          <span class="flex items-baseline justify-between gap-3">
            <span class="truncate text-sm font-medium">{{ note.title }}</span>
            <span class="shrink-0 text-[11px] text-muted">{{ relativeTime(note.updatedAt) }}</span>
          </span>
          <span
            v-if="note.body"
            class="mt-0.5 line-clamp-2 text-xs whitespace-pre-wrap text-muted"
            >{{ note.body }}</span
          >
        </button>
        <Button
          variant="ghost"
          size="icon"
          class="absolute top-2 right-3 size-6 text-muted opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 [&_svg]:size-3.5"
          :aria-label="`Delete note “${note.title}”`"
          @click="remove(note)"
        >
          <Trash2 />
        </Button>
      </li>
    </ul>

    <p
      v-if="errors.general.value"
      class="border-t px-5 py-2 text-xs text-status-failed"
      role="alert"
    >
      {{ errors.general.value }}
    </p>
  </Card>
</template>
