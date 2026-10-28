import { describe, expect, it } from 'vitest'
import { displayLine, duration, isActive, machineState, operationLabel } from './operations'

describe('machineState', () => {
  it('maps provider states and keeps unknown ones unknown', () => {
    expect(machineState('running')).toBe('running')
    expect(machineState('poweroff')).toBe('stopped')
    expect(machineState('not_running')).toBe('stopped')
    expect(machineState('saved')).toBe('suspended')
    expect(machineState('not_created')).toBe('notCreated')
    expect(machineState('gurumeditation')).toBe('unknown')
    expect(machineState(null)).toBe('unknown')
  })
})

describe('displayLine', () => {
  it('shows the message of machine-readable ui lines', () => {
    expect(
      displayLine({
        kind: 'out',
        text: '1793205206,,ui,info,Current machine states:\\n\\nweb   not created (vmware_desktop%!(VAGRANT_COMMA) x)',
      }),
    ).toBe('Current machine states:\n\nweb   not created (vmware_desktop, x)')
    expect(displayLine({ kind: 'out', text: '1,web,ui,error,boom' })).toBe('web: boom')
  })

  it('keeps other lines verbatim', () => {
    expect(displayLine({ kind: 'out', text: '1793205206,web,state,not_created' })).toBe(
      '1793205206,web,state,not_created',
    )
    expect(displayLine({ kind: 'err', text: '==> web: Booting VM...' })).toBe(
      '==> web: Booting VM...',
    )
  })
})

describe('operation helpers', () => {
  it('formats durations', () => {
    const start = '2026-10-28T12:00:00Z'
    expect(duration({ startedAt: null, finishedAt: null })).toBe('—')
    expect(duration({ startedAt: start, finishedAt: '2026-10-28T12:00:42Z' })).toBe('42s')
    expect(duration({ startedAt: start, finishedAt: '2026-10-28T12:03:05Z' })).toBe('3m 5s')
    expect(
      duration({ startedAt: start, finishedAt: null }, Date.parse('2026-10-28T14:10:00Z')),
    ).toBe('2h 10m')
  })

  it('classifies active operations and labels types', () => {
    expect(isActive({ status: 'running' })).toBe(true)
    expect(isActive({ status: 'cancel_requested' })).toBe(true)
    expect(isActive({ status: 'interrupted' })).toBe(false)
    expect(operationLabel('up')).toBe('Start')
    expect(operationLabel('snapshot_save')).toBe('snapshot_save')
  })
})
