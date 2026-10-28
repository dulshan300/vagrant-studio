<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Compartment, EditorState } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import { editingExtensions, lineSeparator, vagrantfileLanguage } from '@/lib/codemirror'

/** Raw Vagrantfile editor (CodeMirror 6): Ruby highlighting, Ctrl/⌘+F search and replace. */
const model = defineModel<string>({ required: true })
const props = withDefaults(defineProps<{ label: string; readonly?: boolean }>(), {
  readonly: false,
})

const host = ref<HTMLDivElement>()
let view: EditorView | null = null
const readOnly = new Compartment()
const separator = new Compartment()

const readOnlyExtension = (on: boolean) => [
  EditorState.readOnly.of(on),
  EditorView.editable.of(!on),
]

onMounted(() => {
  view = new EditorView({
    parent: host.value!,
    state: EditorState.create({
      doc: model.value,
      extensions: [
        editingExtensions(),
        vagrantfileLanguage(),
        readOnly.of(readOnlyExtension(props.readonly)),
        separator.of(EditorState.lineSeparator.of(lineSeparator(model.value))),
        EditorView.contentAttributes.of({ 'aria-label': props.label }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) model.value = update.state.doc.toString()
        }),
      ],
    }),
  })
})

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})

// Replacements from outside (reload, visual edits, a restored backup).
watch(model, (text) => {
  if (!view || text === view.state.doc.toString()) return
  view.dispatch({
    effects: separator.reconfigure(EditorState.lineSeparator.of(lineSeparator(text))),
  })
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } })
})

watch(
  () => props.readonly,
  (on) => view?.dispatch({ effects: readOnly.reconfigure(readOnlyExtension(on)) }),
)

/** Scrolls to a 1-based line and puts the cursor there. */
function revealLine(line: number) {
  if (!view) return
  const target = view.state.doc.line(Math.max(1, Math.min(line, view.state.doc.lines)))
  view.dispatch({
    selection: { anchor: target.from },
    effects: EditorView.scrollIntoView(target.from, { y: 'center' }),
  })
  view.focus()
}

defineExpose({ revealLine })
</script>

<template>
  <div ref="host" class="h-full min-h-0 overflow-hidden" />
</template>
