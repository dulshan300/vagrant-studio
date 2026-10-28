import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter, RouterView } from 'vue-router'
import VagrantfilePage from './VagrantfilePage.vue'
import { projectsApi } from '@/services/projects'
import { vagrantfileApi } from '@/services/vagrantfile'
import { starterModel } from '@/lib/vagrantfile'
import type { ProjectDetails } from '@/types/projects'
import type { Analysis, VagrantConfig, VagrantfileDocument } from '@/types/vagrantfile'

vi.mock('@/services/projects', () => ({ projectsApi: { get: vi.fn() } }))
vi.mock('@/services/vagrantfile', () => ({
  vagrantfileApi: {
    get: vi.fn(),
    analyze: vi.fn(),
    generate: vi.fn(),
    save: vi.fn(),
    backups: vi.fn(),
    readBackup: vi.fn(),
  },
}))

// CodeMirror needs layout APIs jsdom lacks; a textarea stands in for it.
const CodeEditorStub = {
  props: ['modelValue', 'label'],
  emits: ['update:modelValue'],
  template:
    '<textarea :aria-label="label" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
  methods: { revealLine: () => undefined },
}

const details: ProjectDetails = {
  project: {
    id: 'p1',
    name: 'Web',
    slug: 'web',
    description: '',
    rootPath: '/p',
    repositoryUrl: null,
    status: 'active',
    favorite: false,
    tags: [],
    createdAt: '',
    updatedAt: '',
    archivedAt: null,
    lastOpenedAt: null,
    environmentCount: 1,
  },
  environments: [
    {
      id: 'e1',
      projectId: 'p1',
      name: 'default',
      rootPath: '/p',
      vagrantfilePath: 'Vagrantfile',
      defaultProvider: 'virtualbox',
      environmentType: 'development',
      createdAt: '',
      updatedAt: '',
    },
  ],
}

function analysis(model: VagrantConfig | null, extra: Partial<Analysis> = {}): Analysis {
  return {
    model,
    unsupported: model ? null : { line: 3, message: '`if` isn’t supported' },
    managed: false,
    modifiedOutside: false,
    warnings: [],
    errors: [],
    lineEnding: 'lf',
    ...extra,
  }
}

function fileDocument(content: string, a: Analysis, exists = true): VagrantfileDocument {
  return {
    environmentId: 'e1',
    path: '/p/Vagrantfile',
    exists,
    content,
    revision: exists ? 'rev1' : null,
    analysis: a,
  }
}

async function mountPage() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/projects/:id', component: { template: '<div>project page</div>' } },
      { path: '/projects/:id/environments/:environmentId/vagrantfile', component: VagrantfilePage },
    ],
  })
  await router.push('/projects/p1/environments/e1/vagrantfile')
  await router.isReady()
  const wrapper = mount(RouterView, {
    global: { plugins: [router, createPinia()], stubs: { CodeEditor: CodeEditorStub } },
    attachTo: document.body,
  })
  await flushPromises()
  return { wrapper, router }
}

const button = (w: ReturnType<typeof mount>, text: string) =>
  w.findAll('button').find((b) => b.text().includes(text))!

const settle = async () => {
  await new Promise((r) => setTimeout(r, 350))
  await flushPromises()
}

describe('VagrantfilePage', () => {
  let wrapper: ReturnType<typeof mount> | undefined

  beforeEach(() => {
    vi.mocked(projectsApi.get).mockResolvedValue(details)
    vi.mocked(vagrantfileApi.analyze).mockImplementation(async (content) =>
      analysis(content === 'unsupported' ? null : starterModel(null), {
        managed: content.startsWith('# managed'),
      }),
    )
    vi.mocked(vagrantfileApi.generate).mockImplementation(async (model) => ({
      content: `# managed\nhostname=${model.defaults.hostname}\n`,
      errors: [],
    }))
  })

  afterEach(() => {
    wrapper?.unmount()
    vi.clearAllMocks()
  })

  it('opens a supported file in the visual editor and regenerates the text on edits', async () => {
    const model = starterModel('virtualbox')
    vi.mocked(vagrantfileApi.get).mockResolvedValue(fileDocument('original', analysis(model)))
    const page = await mountPage()
    wrapper = page.wrapper

    expect(wrapper.text()).toContain('Hand-written; the visual editor can show it')
    const save = button(wrapper, 'Review and save')
    expect(save.attributes('disabled')).toBeDefined()
    expect(vagrantfileApi.generate).not.toHaveBeenCalled()

    await wrapper.get('#shared-hostname').setValue('devbox')
    await settle()
    expect(vagrantfileApi.generate).toHaveBeenCalledTimes(1)
    expect(wrapper.text()).toContain('Unsaved changes')
    expect(save.attributes('disabled')).toBeUndefined()

    await button(wrapper, 'Raw').trigger('click')
    await flushPromises()
    const raw = wrapper.get('textarea[aria-label="Vagrantfile source"]')
    expect((raw.element as HTMLTextAreaElement).value).toBe('# managed\nhostname=devbox\n')
  })

  it('keeps form problems out of the text and blocks saving', async () => {
    vi.mocked(vagrantfileApi.get).mockResolvedValue(
      fileDocument('original', analysis(starterModel(null))),
    )
    vi.mocked(vagrantfileApi.generate).mockResolvedValue({
      content: null,
      errors: [{ path: 'defaults.hostname', message: 'Use letters, digits and -.' }],
    })
    const page = await mountPage()
    wrapper = page.wrapper
    await wrapper.get('#shared-hostname').setValue('bad_name')
    await settle()
    expect(wrapper.text()).toContain('Use letters, digits and -.')
    expect(button(wrapper, 'Review and save').attributes('disabled')).toBeDefined()

    await button(wrapper, 'Raw').trigger('click')
    expect(wrapper.text()).toContain('Fix the problems marked in the form')
    expect(wrapper.find('textarea[aria-label="Vagrantfile source"]').exists()).toBe(false)
  })

  it('opens unsupported files in the raw editor and explains why visual editing is off', async () => {
    vi.mocked(vagrantfileApi.get).mockResolvedValue(fileDocument('unsupported', analysis(null)))
    const page = await mountPage()
    wrapper = page.wrapper
    expect(wrapper.find('textarea[aria-label="Vagrantfile source"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Hand-written; raw editing only')

    await button(wrapper, 'Visual').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain("The visual editor can't show this Vagrantfile")
    expect(wrapper.text()).toContain('Line 3')
  })

  it('starts a missing Vagrantfile from the environment provider', async () => {
    vi.mocked(vagrantfileApi.get).mockResolvedValue(fileDocument('', analysis(null), false))
    const page = await mountPage()
    wrapper = page.wrapper
    expect(wrapper.text()).toContain('New Vagrantfile')
    const model = vi.mocked(vagrantfileApi.generate).mock.calls[0]![0]
    expect(model.defaults.providers[0]!.provider).toBe('virtualbox')
    expect(button(wrapper, 'Review and create').attributes('disabled')).toBeUndefined()
  })

  it('asks before leaving with unsaved changes', async () => {
    vi.mocked(vagrantfileApi.get).mockResolvedValue(
      fileDocument('original', analysis(starterModel(null))),
    )
    const page = await mountPage()
    wrapper = page.wrapper
    await wrapper.get('#shared-hostname').setValue('devbox')
    await settle()

    const leaving = page.router.push('/projects/p1')
    await flushPromises()
    expect(document.body.textContent).toContain('Discard unsaved changes?')
    const discard = [...document.body.querySelectorAll('button')].find((b) =>
      b.textContent?.includes('Discard changes'),
    )!
    discard.click()
    await leaving
    expect(page.router.currentRoute.value.path).toBe('/projects/p1')
  })
})
