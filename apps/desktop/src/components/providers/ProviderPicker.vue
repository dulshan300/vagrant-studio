<script setup lang="ts">
import { cn } from '@/lib/utils'
import type { ProviderChoice } from '@/composables/useProviderChoices'

const model = defineModel<string | null>({ required: true })
defineProps<{ choices: ProviderChoice[]; checking?: boolean; name: string }>()

const STATE_LABEL = {
  ready: 'Ready',
  attention: 'Needs attention',
  unavailable: 'Not installed',
  unknown: '',
}
const STATE_DOT = {
  ready: 'bg-status-running',
  attention: 'bg-status-transition',
  unavailable: 'bg-status-stopped',
  unknown: 'bg-status-unknown',
}
</script>

<template>
  <div class="grid gap-2 sm:grid-cols-2" role="radiogroup">
    <label
      v-for="c in choices"
      :key="c.id"
      :class="
        cn(
          'flex items-start gap-3 rounded-md border p-3',
          c.available
            ? 'cursor-pointer has-[:checked]:border-accent has-[:checked]:bg-surface-muted'
            : 'opacity-60',
        )
      "
    >
      <input
        v-model="model"
        type="radio"
        :name="name"
        :value="c.id"
        :disabled="!c.available"
        class="mt-1"
      />
      <span class="min-w-0">
        <span class="flex items-center gap-2 font-medium">
          {{ c.name }}
          <span
            v-if="c.state !== 'unknown'"
            class="flex items-center gap-1 text-[11px] font-normal text-muted"
          >
            <span :class="cn('size-1.5 rounded-full', STATE_DOT[c.state])" aria-hidden="true" />
            {{ STATE_LABEL[c.state] }}
          </span>
          <span v-else-if="checking" class="text-[11px] font-normal text-muted">Checking…</span>
        </span>
        <span v-if="c.note" class="block text-xs text-muted">{{ c.note }}</span>
      </span>
    </label>
  </div>
</template>
