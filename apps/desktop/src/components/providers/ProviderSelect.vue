<script setup lang="ts">
import type { ProviderChoice } from '@/composables/useProviderChoices'

const model = defineModel<string | null>({ required: true })
const props = defineProps<{ id: string; choices: ProviderChoice[]; allowUnset?: boolean }>()

/** Keep a current value visible even if it isn't one of the known providers. */
const extra = () =>
  model.value && !props.choices.some((c) => c.id === model.value) ? model.value : null
</script>

<template>
  <select
    :id="id"
    :value="model ?? ''"
    class="rounded-md border bg-surface px-2 py-1.5 text-sm"
    @change="model = ($event.target as HTMLSelectElement).value || null"
  >
    <option v-if="allowUnset" value="">Let Vagrant decide</option>
    <option
      v-for="c in choices"
      :key="c.id"
      :value="c.id"
      :disabled="!c.available && c.id !== model"
    >
      {{ c.name
      }}{{
        c.state === 'unavailable'
          ? ' (not installed)'
          : c.state === 'attention'
            ? ' (needs attention)'
            : ''
      }}
    </option>
    <option v-if="extra()" :value="extra()!">{{ extra() }}</option>
  </select>
</template>
