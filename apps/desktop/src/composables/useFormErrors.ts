import { ref } from 'vue'
import { errorMessage, isCommandError } from '@/services/errors'

/** Splits backend errors into a per-field message (validation) or a general one. */
export function useFormErrors() {
  const field = ref<Record<string, string>>({})
  const general = ref<string | null>(null)

  function clear() {
    field.value = {}
    general.value = null
  }

  function set(error: unknown) {
    clear()
    if (isCommandError(error) && error.kind === 'validation' && error.field) {
      field.value = { [error.field]: error.message }
    } else {
      general.value = errorMessage(error)
    }
  }

  return { field, general, clear, set }
}
