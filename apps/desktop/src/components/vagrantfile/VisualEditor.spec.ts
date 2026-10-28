import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import VisualEditor from './VisualEditor.vue'
import { starterModel } from '@/lib/vagrantfile'
import type { VagrantConfig } from '@/types/vagrantfile'

function mountEditor(model: VagrantConfig, errors: Record<string, string> = {}) {
  return mount(VisualEditor, {
    props: {
      modelValue: model,
      'onUpdate:modelValue': () => undefined,
      errors,
    },
  })
}

describe('VisualEditor', () => {
  it('edits the shared settings of a single-machine file in place', async () => {
    const model = starterModel('virtualbox')
    const wrapper = mountEditor(model)
    expect(wrapper.text()).toContain('Machine “default”')

    await wrapper.get('#shared-hostname').setValue('dev')
    expect(model.defaults.hostname).toBe('dev')
    await wrapper.get('#shared-box').setValue('  ')
    expect(model.defaults.box).toBeNull()
    await wrapper.get('#shared-p0-cpus').setValue('4')
    expect(model.defaults.providers[0]!.cpus).toBe(4)
  })

  it('adds a machine and selects it, turning the rest into shared settings', async () => {
    const model = starterModel(null)
    const wrapper = mountEditor(model)
    const add = wrapper.findAll('button').find((b) => b.text().includes('Add machine'))!
    await add.trigger('click')
    expect(model.machines.map((m) => m.name)).toEqual(['machine1'])
    expect(wrapper.text()).toContain('Shared settings')
    expect((wrapper.get('#m0-name').element as HTMLInputElement).value).toBe('machine1')
    // The box placeholder shows the inherited value.
    expect(wrapper.get('#m0-box').attributes('placeholder')).toBe('Shared: bento/ubuntu-24.04')

    await wrapper.get('#m0-name').setValue('web')
    expect(model.machines[0]!.name).toBe('web')
  })

  it('shows field errors next to inputs and marks machines with problems', async () => {
    const model = starterModel(null)
    model.machines.push({
      name: 'db',
      primary: false,
      autostart: true,
      settings: { ...starterModel(null).defaults, box: null },
    })
    const wrapper = mountEditor(model, { 'machines.0.name': 'Bad name.' })
    expect(wrapper.find('[aria-label="has problems"]').exists()).toBe(true)
    const machineButton = wrapper.findAll('nav button').find((b) => b.text().includes('db'))!
    await machineButton.trigger('click')
    expect(wrapper.text()).toContain('Bad name.')
    expect(wrapper.get('#m0-name').attributes('aria-invalid')).toBe('true')
  })
})
