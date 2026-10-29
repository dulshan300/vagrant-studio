<script setup lang="ts">
import { DropdownMenuContent, DropdownMenuPortal } from 'reka-ui'
</script>

<template>
  <DropdownMenuPortal>
    <DropdownMenuContent
      align="end"
      :side-offset="4"
      class="z-50 min-w-44 rounded-md border bg-surface p-1 shadow-lg focus:outline-none"
    >
      <slot />
    </DropdownMenuContent>
  </DropdownMenuPortal>
</template>
