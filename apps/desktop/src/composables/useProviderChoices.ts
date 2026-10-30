import { computed, onMounted, ref } from 'vue'
import { hasBackend } from '@/services/app'
import { settingsApi } from '@/services/projects'
import { useOperationsStore } from '@/stores/operations'
import type { ProviderCheck } from '@/types/operations'

/** Providers Vagrant Studio can generate configuration for, in display order. */
export const KNOWN_PROVIDERS: readonly { id: string; name: string }[] = [
  { id: 'vmware_desktop', name: 'VMware Desktop' },
  { id: 'virtualbox', name: 'VirtualBox' },
  { id: 'libvirt', name: 'libvirt' },
]

/** Offered only where the host checks report them; libvirt doesn't exist on Windows. */
const REPORTED_ONLY: readonly string[] = ['libvirt']

export interface ProviderChoice {
  id: string
  name: string
  state: ProviderCheck['state'] | 'unknown'
  /** Selectable: installed (it may still need attention, e.g. a stopped service). */
  available: boolean
  /** Why it isn't ready, from the readiness checks. */
  note: string | null
}

/** Pure part, exported for tests. */
export function buildChoices(checks: readonly ProviderCheck[] | null): ProviderChoice[] {
  const known = KNOWN_PROVIDERS.filter(
    ({ id }) => !REPORTED_ONLY.includes(id) || checks?.some((c) => c.id === id),
  )
  return known.map(({ id, name }) => {
    const check = checks?.find((c) => c.id === id)
    if (!check) return { id, name, state: 'unknown', available: true, note: null }
    const problem = check.findings.find((f) => f.level !== 'ok')
    return {
      id,
      name,
      state: check.state,
      available: check.state !== 'unavailable',
      note: problem?.message ?? null,
    }
  })
}

/** The configured default if usable, else the first ready provider, else the first available. */
export function pickDefault(
  choices: readonly ProviderChoice[],
  preferred: string | null,
): string | null {
  const usable = choices.filter((c) => c.available)
  return (
    usable.find((c) => c.id === preferred)?.id ??
    usable.find((c) => c.state === 'ready')?.id ??
    usable[0]?.id ??
    null
  )
}

export function useProviderChoices() {
  const ops = useOperationsStore()
  const preferred = ref<string | null>(null)

  onMounted(async () => {
    if (!hasBackend()) return
    if (!ops.readiness && !ops.readinessLoading) void ops.checkHost()
    preferred.value = (await settingsApi.get().catch(() => null))?.defaultProvider ?? null
  })

  const choices = computed(() => buildChoices(ops.readiness?.providers ?? null))
  const defaultProvider = computed(() => pickDefault(choices.value, preferred.value))
  const checking = computed(() => ops.readinessLoading && !ops.readiness)

  return { choices, defaultProvider, preferred, checking }
}
