import type { Operation } from '@/types/operations'

/**
 * Advice for failures Vagrant Studio can recognize. Patterns come from the
 * wording in Vagrant 2.4.9's templates/locales/en.yml; unknown failures get
 * no advice rather than a guess.
 */
const HINTS: readonly { pattern: RegExp; hint: string }[] = [
  {
    pattern: /another process is already executing an action|the machine\s+is locked|is locked!/i,
    hint: 'Another Vagrant process is using this machine. Wait for it to finish; if none is running, a crashed one may have left its lock, and restarting your computer clears it.',
  },
  {
    pattern: /would collide with some other application/i,
    hint: 'A forwarded host port is already in use. Stop whatever listens on it or change the port in the Vagrantfile. A suspended machine can’t move its ports: use Reload instead of Resume.',
  },
  {
    pattern: /Timed out while waiting for the machine to boot/i,
    hint: 'The machine started but Vagrant couldn’t reach it over SSH. Try Reload. If it keeps failing, check the box and whether virtualization is enabled on this host.',
  },
  {
    pattern: /timed out while attempting to connect via SSH/i,
    hint: 'Try Reload. If it keeps failing, check the machine’s network settings in the Vagrantfile.',
  },
  {
    pattern:
      /could not be found or could not be accessed in the remote catalog|The box '.*' does not exist/i,
    hint: 'Check the box name on the Boxes page and your network connection.',
  },
  {
    pattern: /doesn't support the provider|no matching provider/i,
    hint: 'This box has no build for the provider in use. Choose another box or provider.',
  },
  {
    pattern: /The specified file .* to save the package as already exists/i,
    hint: 'Export again with a new file name.',
  },
  {
    pattern: /--force. option to replace an existing snapshot/i,
    hint: 'A snapshot with this name already exists. Save under another name, or delete the old one first.',
  },
  {
    pattern: /snapshot name .* was not found/i,
    hint: 'The snapshot no longer exists. Reopen the machine’s snapshots to see the current list.',
  },
  {
    pattern: /error while executing `VBoxManage`/i,
    hint: 'VirtualBox refused the command. Open the Providers page to check its version and host conflicts; the message above has VirtualBox’s own reason.',
  },
]

/** What to try next after a failed or interrupted operation, if known. */
export function recoveryHint(op: Pick<Operation, 'status' | 'errorSummary'>): string | null {
  if (op.status === 'interrupted')
    return 'Vagrant Studio closed while this ran, so the machine may be part-way through. Refresh status before running anything else.'
  if (op.status !== 'failed' || !op.errorSummary) return null
  return HINTS.find((h) => h.pattern.test(op.errorSummary!))?.hint ?? null
}
