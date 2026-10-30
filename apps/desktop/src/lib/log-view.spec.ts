import { describe, expect, it } from 'vitest'
import { highlight, matchingLines, severityOf, step, viewLines } from './log-view'
import type { LogLine } from '@/types/operations'

const log: LogLine[] = [
  { kind: 'meta', text: '$ vagrant up --no-tty web' },
  { kind: 'out', text: '==> web: Booting VM...' },
  { kind: 'out', text: '    web: Warning: Connection reset. Retrying...' },
  { kind: 'err', text: 'The SSH command responded with a non-zero exit status.' },
  { kind: 'out', text: '    web: bash: line 3: foo: command not found' },
  { kind: 'out', text: '==> web: Error: provisioning failed' },
]

describe('log view', () => {
  it('marks stderr, error and warning wording, never meta lines', () => {
    expect(severityOf({ kind: 'err' }, 'anything')).toBe('error')
    expect(severityOf({ kind: 'out' }, 'Warning: Connection reset')).toBe('warning')
    expect(severityOf({ kind: 'out' }, 'Error: provisioning failed')).toBe('error')
    expect(severityOf({ kind: 'out' }, 'Booting VM...')).toBeNull()
    expect(severityOf({ kind: 'meta' }, 'exit code 1: failed')).toBeNull()
  })

  it('filters lines and keeps their position in the full log', () => {
    expect(viewLines(log, 'all')).toHaveLength(6)
    expect(viewLines(log, 'problems').map((l) => l.index)).toEqual([2, 3, 5])
    expect(viewLines(log, 'stderr').map((l) => l.index)).toEqual([3])
  })

  it('shows machine-readable ui lines as their message', () => {
    const lines = viewLines(
      [{ kind: 'out', text: '1700000000,web,ui,error,Something failed%!(VAGRANT_COMMA) badly' }],
      'problems',
    )
    expect(lines[0]?.text).toBe('web: Something failed, badly')
  })

  it('finds lines containing the query, ignoring case', () => {
    const lines = viewLines(log, 'all')
    expect(matchingLines(lines, 'WEB:')).toEqual([1, 2, 4, 5])
    expect(matchingLines(lines, '  ')).toEqual([])
  })

  it('splits text around every match', () => {
    expect(highlight('Web web WEB', 'web')).toEqual([
      { text: 'Web', match: true },
      { text: ' ', match: false },
      { text: 'web', match: true },
      { text: ' ', match: false },
      { text: 'WEB', match: true },
    ])
    expect(highlight('no match', 'x')).toEqual([{ text: 'no match', match: false }])
    expect(highlight('', 'x')).toEqual([{ text: '', match: false }])
  })

  it('steps through positions and wraps around', () => {
    expect(step([], null, true)).toBeNull()
    expect(step([2, 5, 9], null, true)).toBe(2)
    expect(step([2, 5, 9], null, false)).toBe(9)
    expect(step([2, 5, 9], 5, true)).toBe(9)
    expect(step([2, 5, 9], 9, true)).toBe(2)
    expect(step([2, 5, 9], 2, false)).toBe(9)
    expect(step([2, 5, 9], 6, false)).toBe(5)
  })
})
