import { displayLine } from '@/lib/operations'
import type { LogLine } from '@/types/operations'

/** Which lines the output view shows. */
export type LogFilter = 'all' | 'problems' | 'stderr'

export type Severity = 'error' | 'warning'

/** A log line prepared for display, keeping its position in the full log. */
export interface ViewLine {
  index: number
  kind: LogLine['kind']
  text: string
  severity: Severity | null
}

const ERROR_WORDS = /\b(error|errors|failed|failure|fatal|exception|denied|aborted|refused)\b/i
const WARNING_WORDS = /\b(warn|warning|warnings|deprecated|deprecation)\b/i

/**
 * Lines worth jumping to. Vagrant prints guest stderr on stdout, so stdout
 * lines are checked for wording too; stderr lines are always problems.
 * Words like "0 errors" still match, so this is a navigation aid, not a verdict.
 */
export function severityOf(line: Pick<LogLine, 'kind'>, text: string): Severity | null {
  if (line.kind === 'meta') return null
  if (ERROR_WORDS.test(text)) return 'error'
  if (WARNING_WORDS.test(text)) return 'warning'
  return line.kind === 'err' ? 'error' : null
}

export function viewLines(lines: readonly LogLine[], filter: LogFilter): ViewLine[] {
  const out: ViewLine[] = []
  lines.forEach((line, index) => {
    const text = displayLine(line)
    const severity = severityOf(line, text)
    if (filter === 'stderr' && line.kind !== 'err') return
    if (filter === 'problems' && !severity) return
    out.push({ index, kind: line.kind, text, severity })
  })
  return out
}

/** Positions (in `lines`) of lines containing `query`, ignoring case. */
export function matchingLines(lines: readonly Pick<ViewLine, 'text'>[], query: string): number[] {
  const needle = query.trim().toLowerCase()
  if (!needle) return []
  const out: number[] = []
  lines.forEach((l, i) => {
    if (l.text.toLowerCase().includes(needle)) out.push(i)
  })
  return out
}

export interface Segment {
  text: string
  match: boolean
}

/** Splits `text` around every case-insensitive occurrence of `query`. */
export function highlight(text: string, query: string): Segment[] {
  const needle = query.trim().toLowerCase()
  if (!needle) return [{ text, match: false }]
  const lower = text.toLowerCase()
  const out: Segment[] = []
  let from = 0
  for (let at = lower.indexOf(needle); at !== -1; at = lower.indexOf(needle, from)) {
    if (at > from) out.push({ text: text.slice(from, at), match: false })
    out.push({ text: text.slice(at, at + needle.length), match: true })
    from = at + needle.length
  }
  if (from < text.length || !out.length) out.push({ text: text.slice(from), match: false })
  return out
}

/** The next (or previous) position after `current`, wrapping around; null when none. */
export function step(positions: readonly number[], current: number | null, forward: boolean) {
  if (!positions.length) return null
  if (current === null) return forward ? positions[0]! : positions[positions.length - 1]!
  if (forward) return positions.find((p) => p > current) ?? positions[0]!
  return [...positions].reverse().find((p) => p < current) ?? positions[positions.length - 1]!
}
