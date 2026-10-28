import { describe, expect, it } from 'vitest'
import { describeHost } from './host'

describe('describeHost', () => {
  it('labels native and WSL hosts distinctly', () => {
    expect(describeHost({ os: 'windows', arch: 'x86_64', isWsl: false })).toBe('Windows · x64')
    expect(describeHost({ os: 'linux', arch: 'x86_64', isWsl: false })).toBe('Linux · x64')
    expect(describeHost({ os: 'linux', arch: 'x86_64', isWsl: true })).toBe('Linux (WSL) · x64')
  })
})
