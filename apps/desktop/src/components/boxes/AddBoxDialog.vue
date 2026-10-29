<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { FileUp } from '@lucide/vue'
import AppDialog from '@/components/ui/dialog/AppDialog.vue'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import { Button } from '@/components/ui/button'
import ProviderSelect from '@/components/providers/ProviderSelect.vue'
import { useProviderChoices } from '@/composables/useProviderChoices'
import { boxesApi, pickBoxFile } from '@/services/boxes'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { BoxSource } from '@/types/boxes'
import type { Operation } from '@/types/operations'

const open = defineModel<boolean>('open', { required: true })
const emit = defineEmits<{ started: [operation: Operation] }>()
const providers = useProviderChoices()
const operations = useOperationsStore()

const kind = ref<BoxSource['kind']>('catalog')
const source = ref('')
const provider = ref<string | null>(null)
const version = ref('')
const name = ref('')
const busy = ref(false)
const error = ref<string | null>(null)

watch(open, (isOpen) => {
  if (!isOpen) return
  kind.value = 'catalog'
  source.value = ''
  provider.value = providers.defaultProvider.value
  version.value = ''
  name.value = ''
  error.value = null
})

const SOURCES = [
  { value: 'catalog', label: 'Catalog name' },
  { value: 'url', label: 'URL' },
  { value: 'file', label: 'Local file' },
] as const

// A catalog box usually offers several providers, and Vagrant would ask which one.
const needsProvider = computed(() => kind.value === 'catalog')
const canSubmit = computed(
  () =>
    !busy.value &&
    !!source.value.trim() &&
    (!needsProvider.value || !!provider.value) &&
    (kind.value !== 'file' || !!name.value.trim()),
)

async function browse() {
  try {
    const path = await pickBoxFile()
    if (path) source.value = path
  } catch (e) {
    error.value = errorMessage(e)
  }
}

async function submit() {
  busy.value = true
  error.value = null
  try {
    const op = await boxesApi.add({
      source: { kind: kind.value, value: source.value.trim() },
      provider: provider.value,
      version: kind.value === 'file' ? null : version.value.trim() || null,
      architecture: null,
      name: kind.value === 'catalog' ? null : name.value.trim() || null,
    })
    operations.track(op)
    emit('started', op)
    open.value = false
  } catch (e) {
    error.value = errorMessage(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <AppDialog
    v-model:open="open"
    title="Add box"
    description="Downloads a box into Vagrant's box store. It runs as an operation you can follow and cancel."
  >
    <form id="add-box" class="space-y-4" @submit.prevent="submit">
      <fieldset class="space-y-1.5">
        <legend class="text-xs font-medium">Source</legend>
        <div class="flex flex-wrap gap-1.5">
          <label
            v-for="s in SOURCES"
            :key="s.value"
            class="flex cursor-pointer items-center gap-1.5 rounded-md border px-2.5 py-1 text-sm has-[:checked]:border-foreground has-[:checked]:bg-surface-muted"
          >
            <input v-model="kind" type="radio" name="box-source" :value="s.value" class="sr-only" />
            {{ s.label }}
          </label>
        </div>
      </fieldset>

      <FormField
        v-if="kind === 'catalog'"
        label="Box"
        for="ab-source"
        hint="Its name in the HashiCorp Vagrant catalog, for example bento/ubuntu-24.04."
      >
        <TextInput
          id="ab-source"
          v-model="source"
          placeholder="owner/box"
          autocomplete="off"
          spellcheck="false"
        />
      </FormField>
      <FormField
        v-else-if="kind === 'url'"
        label="URL"
        for="ab-source"
        hint="A .box file or catalog metadata over https. URLs with a user name or password are refused."
      >
        <TextInput
          id="ab-source"
          v-model="source"
          placeholder="https://"
          autocomplete="off"
          spellcheck="false"
        />
      </FormField>
      <FormField v-else label="File" for="ab-source">
        <div class="flex gap-2">
          <TextInput id="ab-source" v-model="source" class="font-mono text-xs" />
          <Button variant="secondary" type="button" @click="browse">
            <FileUp aria-hidden="true" /> Browse…
          </Button>
        </div>
      </FormField>

      <div class="grid gap-4 sm:grid-cols-2">
        <FormField
          label="Provider"
          for="ab-provider"
          :hint="needsProvider ? undefined : 'Optional: Vagrant reads it from the box.'"
        >
          <ProviderSelect
            id="ab-provider"
            v-model="provider"
            :choices="providers.choices.value"
            :allow-unset="!needsProvider"
          />
        </FormField>
        <FormField
          v-if="kind !== 'file'"
          label="Version"
          for="ab-version"
          hint="Optional; the newest version when empty."
        >
          <TextInput id="ab-version" v-model="version" autocomplete="off" spellcheck="false" />
        </FormField>
      </div>

      <FormField
        v-if="kind !== 'catalog'"
        label="Name"
        for="ab-name"
        :hint="
          kind === 'file'
            ? 'Required. Vagrantfiles refer to the box by this name.'
            : 'Needed for a .box file URL; catalog metadata names the box itself.'
        "
      >
        <TextInput
          id="ab-name"
          v-model="name"
          placeholder="owner/box"
          autocomplete="off"
          spellcheck="false"
        />
      </FormField>

      <p v-if="error" class="text-sm text-status-failed" role="alert">{{ error }}</p>
    </form>
    <template #footer>
      <Button variant="secondary" @click="open = false">Cancel</Button>
      <Button type="submit" form="add-box" :disabled="!canSubmit">Add box</Button>
    </template>
  </AppDialog>
</template>
