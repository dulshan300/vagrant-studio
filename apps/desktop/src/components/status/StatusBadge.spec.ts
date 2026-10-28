import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import StatusBadge from './StatusBadge.vue'
import { MACHINE_STATES } from './machine-state'

describe('StatusBadge', () => {
  it('renders a text label for every state so color is never the only signal', () => {
    for (const state of MACHINE_STATES) {
      const wrapper = mount(StatusBadge, { props: { state } })
      expect(wrapper.text().length).toBeGreaterThan(0)
      expect(wrapper.attributes('data-state')).toBe(state)
    }
  })

  it('pulses only transitional states', () => {
    const pulsing = MACHINE_STATES.filter((state) =>
      mount(StatusBadge, { props: { state } }).find('.animate-pulse').exists(),
    )
    expect(pulsing.sort()).toEqual(['provisioning', 'starting', 'stopping'])
  })
})
