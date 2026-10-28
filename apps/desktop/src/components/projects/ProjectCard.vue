<script setup lang="ts">
import { Archive, Layers, Star } from '@lucide/vue'
import { relativeTime } from '@/lib/projects'
import { cn } from '@/lib/utils'
import type { Project } from '@/types/projects'

defineProps<{ project: Project }>()
defineEmits<{ toggleFavorite: [] }>()
</script>

<template>
  <article
    class="group relative flex flex-col rounded-lg border bg-surface p-4 shadow-xs transition-colors hover:border-accent/50"
  >
    <div class="flex items-start justify-between gap-3">
      <RouterLink
        :to="{ name: 'project', params: { id: project.id } }"
        class="min-w-0 font-semibold after:absolute after:inset-0 after:content-['']"
      >
        <span class="block truncate">{{ project.name }}</span>
      </RouterLink>
      <button
        type="button"
        class="relative z-10 -m-1 rounded p-1 text-muted hover:text-foreground"
        :aria-label="project.favorite ? 'Remove from favorites' : 'Add to favorites'"
        :aria-pressed="project.favorite"
        @click="$emit('toggleFavorite')"
      >
        <Star
          :class="cn('size-4', project.favorite && 'fill-status-transition text-status-transition')"
          aria-hidden="true"
        />
      </button>
    </div>
    <p class="mt-1 line-clamp-2 min-h-[2lh] text-xs text-muted">
      {{ project.description || 'No description' }}
    </p>
    <code class="mt-3 truncate font-mono text-[11px] text-muted" :title="project.rootPath">{{
      project.rootPath
    }}</code>
    <div class="mt-3 flex flex-wrap items-center gap-1.5">
      <span
        v-if="project.status === 'archived'"
        class="inline-flex items-center gap-1 rounded bg-surface-muted px-1.5 py-0.5 text-[11px]"
      >
        <Archive class="size-3" aria-hidden="true" /> Archived
      </span>
      <span
        v-for="tag in project.tags"
        :key="tag"
        class="rounded bg-surface-muted px-1.5 py-0.5 text-[11px]"
        >{{ tag }}</span
      >
    </div>
    <footer class="mt-auto flex items-center justify-between pt-4 text-[11px] text-muted">
      <span class="inline-flex items-center gap-1">
        <Layers class="size-3" aria-hidden="true" />
        {{ project.environmentCount }}
        {{ project.environmentCount === 1 ? 'environment' : 'environments' }}
      </span>
      <span>{{
        project.lastOpenedAt
          ? `Opened ${relativeTime(project.lastOpenedAt)}`
          : `Added ${relativeTime(project.createdAt)}`
      }}</span>
    </footer>
  </article>
</template>
