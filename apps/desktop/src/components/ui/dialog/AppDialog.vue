<script setup lang="ts">
import { X } from '@lucide/vue'
import {
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from 'reka-ui'
import { cn } from '@/lib/utils'

const open = defineModel<boolean>('open', { required: true })
const props = withDefaults(
  defineProps<{ title: string; description?: string; size?: 'md' | 'lg' }>(),
  { description: undefined, size: 'md' },
)
</script>

<template>
  <DialogRoot v-model:open="open">
    <DialogPortal>
      <DialogOverlay class="fixed inset-0 z-40 bg-black/40" />
      <DialogContent
        :class="
          cn(
            'fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[calc(100vw-2rem)] -translate-x-1/2 -translate-y-1/2 flex-col rounded-lg border bg-surface shadow-lg focus:outline-none',
            props.size === 'lg' ? 'max-w-2xl' : 'max-w-lg',
          )
        "
      >
        <header class="flex items-start justify-between gap-4 border-b px-5 py-4">
          <div>
            <DialogTitle class="font-semibold">{{ title }}</DialogTitle>
            <DialogDescription v-if="description" class="mt-0.5 text-xs text-muted">
              {{ description }}
            </DialogDescription>
          </div>
          <DialogClose
            class="rounded-md p-1 text-muted hover:bg-surface-muted hover:text-foreground"
            aria-label="Close"
          >
            <X class="size-4" />
          </DialogClose>
        </header>
        <div class="overflow-y-auto px-5 py-4">
          <slot />
        </div>
        <footer v-if="$slots.footer" class="flex justify-end gap-2 border-t px-5 py-3">
          <slot name="footer" />
        </footer>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
