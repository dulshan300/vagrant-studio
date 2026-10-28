import { onBeforeUnmount, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NAV_ITEMS } from '@/app/navigation'

/** Returns the nav index for Ctrl/⌘ + digit, or null when the event isn't a navigation shortcut. */
export function navigationShortcutIndex(
  event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey' | 'shiftKey'>,
): number | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return null
  const digit = Number.parseInt(event.key, 10)
  if (!Number.isInteger(digit) || digit < 1 || digit > NAV_ITEMS.length) return null
  return digit - 1
}

export function useNavigationShortcuts() {
  const router = useRouter()

  function onKeydown(event: KeyboardEvent) {
    const index = navigationShortcutIndex(event)
    const item = index === null ? undefined : NAV_ITEMS[index]
    if (!item) return
    event.preventDefault()
    void router.push(item.path)
  }

  onMounted(() => window.addEventListener('keydown', onKeydown))
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
}
