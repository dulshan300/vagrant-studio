<script setup lang="ts">
defineProps<{ label: string; for: string; hint?: string; error?: string | null }>()
</script>

<template>
  <div class="space-y-1.5">
    <label :for="$props.for" class="block text-xs font-medium">{{ label }}</label>
    <slot />
    <p v-if="error" class="text-xs text-status-failed" role="alert">{{ error }}</p>
    <p v-else-if="hint" class="text-xs text-muted">{{ hint }}</p>
  </div>
</template>
