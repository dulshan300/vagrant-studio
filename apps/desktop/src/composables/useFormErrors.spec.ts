import { describe, expect, it } from 'vitest'
import { useFormErrors } from './useFormErrors'
import { errorMessage } from '@/services/errors'

describe('useFormErrors', () => {
  it('maps validation errors to their field', () => {
    const errors = useFormErrors()
    errors.set({ kind: 'validation', message: 'Name is required.', field: 'name', existing: null })
    expect(errors.field.value).toEqual({ name: 'Name is required.' })
    expect(errors.general.value).toBeNull()
  })

  it('shows other errors as a general message', () => {
    const errors = useFormErrors()
    errors.set({ kind: 'conflict', message: 'Already registered.', field: null, existing: null })
    expect(errors.general.value).toBe('Already registered.')
    errors.set(new Error('boom'))
    expect(errors.general.value).toBe('boom')
    errors.clear()
    expect(errors.general.value).toBeNull()
  })

  it('formats unknown errors safely', () => {
    expect(errorMessage(42)).toBe('Something went wrong.')
    expect(errorMessage('plain')).toBe('plain')
  })
})
