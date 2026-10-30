<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { ArrowDown, ArrowUp, ListTodo, Plus, Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardHeader } from '@/components/ui/card'
import { hasBackend } from '@/services/app'
import { errorMessage } from '@/services/errors'
import { tasksApi } from '@/services/workspace'
import type { ProjectTask } from '@/types/workspace'

const props = defineProps<{ projectId: string }>()

const tasks = ref<ProjectTask[]>([])
const newTitle = ref('')
const editing = ref<string | null>(null)
const editTitle = ref('')
const error = ref<string | null>(null)
const editInput = ref<HTMLInputElement[]>([])

const open = computed(() => tasks.value.filter((t) => !t.done))
const done = computed(() => tasks.value.filter((t) => t.done))

async function attempt(action: () => Promise<void>) {
  error.value = null
  try {
    await action()
  } catch (e) {
    error.value = errorMessage(e)
  }
}

async function load() {
  if (!hasBackend()) return
  await attempt(async () => {
    tasks.value = await tasksApi.list(props.projectId)
  })
}
watch(() => props.projectId, load, { immediate: true })

function replace(task: ProjectTask) {
  tasks.value = tasks.value.map((t) => (t.id === task.id ? task : t))
}

const add = () =>
  attempt(async () => {
    if (!newTitle.value.trim()) return
    tasks.value.push(await tasksApi.create(props.projectId, newTitle.value))
    newTitle.value = ''
  })

const toggle = (task: ProjectTask) =>
  attempt(async () =>
    replace(await tasksApi.update(props.projectId, task.id, { done: !task.done })),
  )

async function startEdit(task: ProjectTask) {
  editing.value = task.id
  editTitle.value = task.title
  await nextTick()
  editInput.value[0]?.focus()
}

const saveEdit = (task: ProjectTask) =>
  attempt(async () => {
    if (editing.value !== task.id) return
    editing.value = null
    if (editTitle.value.trim() === task.title) return
    replace(await tasksApi.update(props.projectId, task.id, { title: editTitle.value }))
  })

const remove = (task: ProjectTask) =>
  attempt(async () => {
    await tasksApi.remove(props.projectId, task.id)
    tasks.value = tasks.value.filter((t) => t.id !== task.id)
  })

/** Moves an open task up or down among the open tasks; done tasks keep their place after them. */
const move = (task: ProjectTask, by: -1 | 1) =>
  attempt(async () => {
    const order = open.value.map((t) => t.id)
    const from = order.indexOf(task.id)
    const to = from + by
    if (to < 0 || to >= order.length) return
    order.splice(to, 0, ...order.splice(from, 1))
    tasks.value = await tasksApi.reorder(props.projectId, [
      ...order,
      ...done.value.map((t) => t.id),
    ])
  })
</script>

<template>
  <Card>
    <CardHeader
      title="Tasks"
      :description="
        tasks.length
          ? `${open.length} open · ${done.length} done`
          : 'A to-do list for this project.'
      "
    />
    <form class="flex gap-2 border-b px-5 py-3" @submit.prevent="add">
      <label for="new-task" class="sr-only">New task</label>
      <input
        id="new-task"
        v-model="newTitle"
        type="text"
        maxlength="200"
        placeholder="Add a task…"
        class="min-w-0 flex-1 rounded-md border bg-surface px-3 py-1.5 text-sm placeholder:text-muted/70"
      />
      <Button type="submit" variant="secondary" size="sm" :disabled="!newTitle.trim()">
        <Plus /> Add
      </Button>
    </form>

    <p v-if="!tasks.length" class="flex items-center gap-2 px-5 py-4 text-xs text-muted">
      <ListTodo class="size-4" aria-hidden="true" /> No tasks yet.
    </p>
    <ul v-else class="divide-y">
      <li
        v-for="(task, i) in open"
        :key="task.id"
        class="group flex items-center gap-2 px-5 py-1.5"
      >
        <input
          type="checkbox"
          :checked="false"
          :aria-label="`Mark “${task.title}” done`"
          @change="toggle(task)"
        />
        <input
          v-if="editing === task.id"
          ref="editInput"
          v-model="editTitle"
          type="text"
          maxlength="200"
          :aria-label="`Rename “${task.title}”`"
          class="min-w-0 flex-1 rounded border bg-surface px-2 py-0.5 text-sm"
          @keydown.enter.prevent="saveEdit(task)"
          @keydown.esc="editing = null"
          @blur="saveEdit(task)"
        />
        <button
          v-else
          type="button"
          class="min-w-0 flex-1 truncate py-0.5 text-left text-sm"
          title="Click to rename"
          @click="startEdit(task)"
        >
          {{ task.title }}
        </button>
        <span
          class="flex items-center opacity-0 group-focus-within:opacity-100 group-hover:opacity-100"
        >
          <Button
            variant="ghost"
            size="icon"
            class="size-6 [&_svg]:size-3.5"
            :disabled="i === 0"
            :aria-label="`Move “${task.title}” up`"
            @click="move(task, -1)"
          >
            <ArrowUp />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-6 [&_svg]:size-3.5"
            :disabled="i === open.length - 1"
            :aria-label="`Move “${task.title}” down`"
            @click="move(task, 1)"
          >
            <ArrowDown />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-6 text-muted [&_svg]:size-3.5"
            :aria-label="`Delete “${task.title}”`"
            @click="remove(task)"
          >
            <Trash2 />
          </Button>
        </span>
      </li>
    </ul>

    <details v-if="done.length" class="border-t px-5 py-2">
      <summary class="cursor-pointer text-xs text-muted select-none">
        Done ({{ done.length }})
      </summary>
      <ul class="mt-1">
        <li v-for="task in done" :key="task.id" class="group flex items-center gap-2 py-1">
          <input
            type="checkbox"
            :checked="true"
            :aria-label="`Reopen “${task.title}”`"
            @change="toggle(task)"
          />
          <span class="min-w-0 flex-1 truncate text-sm text-muted line-through">{{
            task.title
          }}</span>
          <Button
            variant="ghost"
            size="icon"
            class="size-6 text-muted opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 [&_svg]:size-3.5"
            :aria-label="`Delete “${task.title}”`"
            @click="remove(task)"
          >
            <Trash2 />
          </Button>
        </li>
      </ul>
    </details>

    <p v-if="error" class="border-t px-5 py-2 text-xs text-status-failed" role="alert">
      {{ error }}
    </p>
  </Card>
</template>
