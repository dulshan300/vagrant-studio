import { describe, expect, it } from 'vitest'
import { recoveryHint } from './recovery'

const failed = (errorSummary: string | null) => ({ status: 'failed' as const, errorSummary })

describe('recoveryHint', () => {
  it('recognizes Vagrant failures by their wording', () => {
    expect(
      recoveryHint(
        failed(
          'Vagrant cannot forward the specified ports on this VM, since they\nwould collide with some other application that is already listening',
        ),
      ),
    ).toContain('forwarded host port')
    expect(recoveryHint(failed('Timed out while waiting for the machine to boot.'))).toContain(
      'Reload',
    )
    expect(
      recoveryHint(failed('but another process is already executing an action on the machine.')),
    ).toContain('Another Vagrant process')
  })

  it('recognizes snapshot failures from real output', () => {
    expect(
      recoveryHint(
        failed('You must include the `--force` option to replace an existing snapshot.'),
      ),
    ).toContain('already exists')
    expect(
      recoveryHint(
        failed('The snapshot name `nope` was not found for the\nvirtual machine `web`.'),
      ),
    ).toContain('no longer exists')
  })

  it('stays quiet about what it does not recognize', () => {
    expect(recoveryHint(failed('Something new went wrong.'))).toBeNull()
    expect(recoveryHint(failed(null))).toBeNull()
    expect(
      recoveryHint({
        status: 'cancelled',
        errorSummary: 'would collide with some other application',
      }),
    ).toBeNull()
  })

  it('tells the user to check state after an interrupted operation', () => {
    expect(recoveryHint({ status: 'interrupted', errorSummary: null })).toContain('Refresh status')
  })
})
