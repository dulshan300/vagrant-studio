import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import {
  HighlightStyle,
  StreamLanguage,
  bracketMatching,
  indentOnInput,
  syntaxHighlighting,
} from '@codemirror/language'
import { ruby } from '@codemirror/legacy-modes/mode/ruby'
import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search'
import { EditorState, type Extension } from '@codemirror/state'
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from '@codemirror/view'
import { tags } from '@lezer/highlight'

/** Colors come from the theme tokens in styles/main.css, so light and dark both work. */
const highlightStyle = HighlightStyle.define([
  { tag: [tags.keyword, tags.bool, tags.null], color: 'var(--syntax-keyword)' },
  { tag: [tags.string, tags.special(tags.string), tags.regexp], color: 'var(--syntax-string)' },
  { tag: [tags.number, tags.atom], color: 'var(--syntax-number)' },
  { tag: tags.comment, color: 'var(--syntax-comment)', fontStyle: 'italic' },
  {
    tag: [tags.variableName, tags.definition(tags.variableName), tags.propertyName],
    color: 'var(--syntax-name)',
  },
])

const theme = EditorView.theme({
  '&': {
    height: '100%',
    fontSize: '13px',
    backgroundColor: 'var(--surface)',
    color: 'var(--foreground)',
  },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.55' },
  '.cm-content': { caretColor: 'var(--foreground)' },
  '.cm-cursor': { borderLeftColor: 'var(--foreground)' },
  '.cm-gutters': {
    backgroundColor: 'var(--surface-muted)',
    color: 'var(--muted)',
    borderRight: '1px solid var(--border)',
  },
  '.cm-activeLine': {
    backgroundColor: 'color-mix(in oklch, var(--surface-muted) 60%, transparent)',
  },
  '.cm-activeLineGutter': { backgroundColor: 'var(--surface-muted)', color: 'var(--foreground)' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'color-mix(in oklch, var(--accent) 25%, transparent) !important',
  },
  '.cm-searchMatch': {
    backgroundColor: 'color-mix(in oklch, var(--status-transition) 35%, transparent)',
  },
  '.cm-panels': { backgroundColor: 'var(--surface-muted)', color: 'var(--foreground)' },
  '.cm-panels-bottom': { borderTop: '1px solid var(--border)' },
  '.cm-panel input, .cm-panel button': { fontSize: '12px' },
  '.cm-textfield': {
    backgroundColor: 'var(--surface)',
    border: '1px solid var(--border)',
    borderRadius: '4px',
  },
  '.cm-button': {
    backgroundImage: 'none',
    backgroundColor: 'var(--surface)',
    border: '1px solid var(--border)',
    borderRadius: '4px',
  },
  '.cm-changedLine, .cm-insertedLine': { backgroundColor: 'var(--diff-added) !important' },
  '.cm-deletedChunk': { backgroundColor: 'var(--diff-removed) !important' },
  '.cm-changedText, ins.cm-insertedLine': { background: 'none' },
  '.cm-deletedChunk del': { textDecoration: 'line-through' },
  '.cm-collapsedLines': {
    backgroundColor: 'var(--surface-muted)',
    color: 'var(--muted)',
    backgroundImage: 'none',
  },
})

/** Ruby highlighting and the theme, shared by the editor and the diff view. */
export function vagrantfileLanguage(): Extension {
  return [StreamLanguage.define(ruby), syntaxHighlighting(highlightStyle), theme]
}

/** Editing features for the raw editor: history, search/replace (Ctrl/⌘+F), line numbers. */
export function editingExtensions(): Extension {
  return [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightActiveLine(),
    history(),
    drawSelection(),
    indentOnInput(),
    bracketMatching(),
    highlightSelectionMatches(),
    search({ top: false }),
    EditorState.tabSize.of(2),
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
  ]
}

/** CodeMirror keeps one separator per document; CRLF files must stay CRLF. */
export function lineSeparator(text: string): '\r\n' | '\n' {
  return text.includes('\r\n') ? '\r\n' : '\n'
}
