<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { getChunks, unifiedMergeView } from '@codemirror/merge'
import { EditorState } from '@codemirror/state'
import { EditorView, lineNumbers } from '@codemirror/view'
import { lineSeparator, vagrantfileLanguage } from '@/lib/codemirror'

/** Read-only unified diff: removed lines inline above the lines that replace them. */
const props = defineProps<{ original: string; modified: string }>()
const emit = defineEmits<{ changes: [count: number] }>()

const host = ref<HTMLDivElement>()
let view: EditorView | null = null

function render() {
  view?.destroy()
  view = new EditorView({
    parent: host.value!,
    state: EditorState.create({
      doc: props.modified,
      extensions: [
        lineNumbers(),
        vagrantfileLanguage(),
        EditorState.readOnly.of(true),
        EditorView.editable.of(false),
        EditorState.lineSeparator.of(lineSeparator(props.modified)),
        EditorView.contentAttributes.of({ 'aria-label': 'Changes to the Vagrantfile' }),
        unifiedMergeView({
          original: props.original,
          mergeControls: false,
          highlightChanges: false,
          collapseUnchanged: { margin: 3, minSize: 6 },
        }),
      ],
    }),
  })
  emit('changes', getChunks(view.state)?.chunks.length ?? 0)
}

onMounted(render)
onBeforeUnmount(() => view?.destroy())
watch(() => [props.original, props.modified], render)
</script>

<template>
  <div ref="host" class="max-h-[50vh] overflow-auto rounded-md border" />
</template>
