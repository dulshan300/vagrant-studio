<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { ArrowUpCircle, Package, Plus, RefreshCw, SearchCheck, Trash2 } from '@lucide/vue'
import PageHeader from '@/components/layout/PageHeader.vue'
import EmptyState from '@/components/layout/EmptyState.vue'
import AddBoxDialog from '@/components/boxes/AddBoxDialog.vue'
import RemoveBoxDialog from '@/components/boxes/RemoveBoxDialog.vue'
import BoxUpdateState from '@/components/boxes/BoxUpdateState.vue'
import OperationStatusBadge from '@/components/operations/OperationStatusBadge.vue'
import { Button } from '@/components/ui/button'
import { Card, CardHeader } from '@/components/ui/card'
import {
  BOX_CHANGES,
  activeUsers,
  boxKey,
  boxOperationTarget,
  boxRef,
  sortBoxes,
  updateStateFor,
} from '@/lib/boxes'
import { operationLabel } from '@/lib/operations'
import { boxesApi } from '@/services/boxes'
import { errorMessage } from '@/services/errors'
import { useOperationsStore } from '@/stores/operations'
import type { BoxStore, InstalledBox, OutdatedBox } from '@/types/boxes'

const operations = useOperationsStore()

const store = ref<BoxStore | null>(null)
const loading = ref(false)
const loadError = ref<string | null>(null)
const updates = ref<OutdatedBox[] | null>(null)
const checking = ref(false)
const checkError = ref<string | null>(null)
const actionError = ref<string | null>(null)
const adding = ref(false)
const removing = ref<InstalledBox | null>(null)
const removeOpen = ref(false)

async function load() {
  loading.value = true
  try {
    store.value = await boxesApi.list()
    loadError.value = null
  } catch (e) {
    loadError.value = errorMessage(e)
  } finally {
    loading.value = false
  }
}

async function checkUpdates() {
  checking.value = true
  checkError.value = null
  try {
    updates.value = await boxesApi.checkUpdates()
  } catch (e) {
    checkError.value = errorMessage(e)
  } finally {
    checking.value = false
  }
}

onMounted(load)

const boxes = computed(() => sortBoxes(store.value?.boxes ?? []))
const updateStates = computed(
  () => new Map(boxes.value.map((b) => [boxKey(b), updateStateFor(b, updates.value)])),
)
const boxOperations = computed(() =>
  operations.active.filter((o) => BOX_CHANGES.includes(o.operationType)).reverse(),
)

// The list is re-read whenever a box change finishes, whatever its outcome.
watch(
  () => boxOperations.value.map((o) => o.id),
  (now, before) => {
    if (before?.some((id) => !now.includes(id))) {
      void load()
      if (updates.value) void checkUpdates()
    }
  },
)

function confirmRemove(box: InstalledBox) {
  removing.value = box
  removeOpen.value = true
}

async function update(box: InstalledBox) {
  const target = boxRef(box)
  if (!target) return
  actionError.value = null
  try {
    operations.track(await boxesApi.update(target))
  } catch (e) {
    actionError.value = errorMessage(e)
  }
}

function usersLabel(box: InstalledBox): string {
  const n = activeUsers(box).length
  return n === 0 ? 'Not used' : n === 1 ? '1 machine' : `${n} machines`
}
</script>

<template>
  <PageHeader
    title="Boxes"
    description="Base images in Vagrant's box store on this computer. They are shared by every project."
  >
    <template #actions>
      <Button variant="secondary" :disabled="checking || !boxes.length" @click="checkUpdates">
        <SearchCheck :class="checking && 'animate-pulse'" />
        {{ checking ? 'Checking…' : 'Check for updates' }}
      </Button>
      <Button variant="secondary" :disabled="loading" @click="load">
        <RefreshCw :class="loading && 'animate-spin'" /> Refresh
      </Button>
      <Button @click="adding = true"><Plus /> Add box</Button>
    </template>
  </PageHeader>

  <div class="space-y-5">
    <Card v-if="boxOperations.length">
      <CardHeader
        title="Box changes"
        description="Starting machines waits while a box is being added, updated or removed."
      />
      <ul class="divide-y">
        <li
          v-for="op in boxOperations"
          :key="op.id"
          class="flex flex-wrap items-center justify-between gap-3 px-5 py-2.5"
        >
          <span>
            <span class="font-medium">{{ operationLabel(op.operationType) }}</span>
            <span class="ml-2 font-mono text-xs text-muted">{{ boxOperationTarget(op) }}</span>
          </span>
          <span class="flex items-center gap-2">
            <OperationStatusBadge :status="op.status" />
            <Button size="sm" variant="secondary" @click="operations.open(op.id)">Output</Button>
          </span>
        </li>
      </ul>
    </Card>

    <p v-if="checkError" class="text-sm text-status-failed" role="alert">
      The update check failed: {{ checkError }}
    </p>
    <p v-if="actionError" class="text-sm text-status-failed" role="alert">{{ actionError }}</p>
    <p v-if="store?.usageError" class="text-sm text-muted">
      Machine usage is unavailable: {{ store.usageError }}
    </p>

    <Card v-if="loadError">
      <EmptyState :icon="Package" title="Boxes couldn't be listed" :description="loadError" />
    </Card>
    <p v-else-if="!store" class="text-muted">Reading the box store…</p>
    <Card v-else-if="!boxes.length">
      <EmptyState
        :icon="Package"
        title="No boxes installed"
        description="Vagrant downloads a box the first time a machine needs it, or you can add one now."
      >
        <Button @click="adding = true"><Plus /> Add box</Button>
      </EmptyState>
    </Card>
    <Card v-else class="overflow-x-auto">
      <table class="w-full text-left text-sm">
        <thead class="border-b bg-surface-muted text-xs text-muted">
          <tr>
            <th class="px-4 py-2 font-medium">Box</th>
            <th class="px-4 py-2 font-medium">Provider</th>
            <th class="px-4 py-2 font-medium">Version</th>
            <th class="px-4 py-2 font-medium">Used by</th>
            <th class="px-4 py-2 text-right font-medium"><span class="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody class="divide-y">
          <tr v-for="box in boxes" :key="boxKey(box)" class="align-top">
            <td class="px-4 py-2.5">
              <span class="font-mono">{{ box.name }}</span>
              <span
                v-if="box.architecture && box.architecture !== 'n/a'"
                class="ml-1.5 text-xs text-muted"
                >{{ box.architecture }}</span
              >
            </td>
            <td class="px-4 py-2.5 text-muted">{{ box.provider ?? 'unknown' }}</td>
            <td class="px-4 py-2.5">
              <span class="font-mono text-xs">{{ box.version ?? 'unknown' }}</span>
              <BoxUpdateState :state="updateStates.get(boxKey(box))" />
            </td>
            <td class="px-4 py-2.5">
              <details v-if="activeUsers(box).length" class="text-xs">
                <summary class="cursor-pointer">{{ usersLabel(box) }}</summary>
                <ul class="mt-1 space-y-0.5">
                  <li v-for="u in activeUsers(box)" :key="`${u.folder}|${u.machine}`">
                    <span class="font-mono">{{ u.machine }}</span>
                    <RouterLink
                      v-if="u.projectId"
                      :to="{ name: 'project', params: { id: u.projectId } }"
                      class="ml-1 underline-offset-2 hover:underline"
                    >
                      {{ u.projectName }} · {{ u.environmentName }}
                    </RouterLink>
                    <span v-else class="ml-1 font-mono text-muted">{{ u.folder }}</span>
                  </li>
                </ul>
              </details>
              <span v-else class="text-xs text-muted">{{ usersLabel(box) }}</span>
            </td>
            <td class="px-4 py-2.5">
              <div class="flex justify-end gap-1.5">
                <Button
                  v-if="updateStates.get(boxKey(box))?.state === 'outdated'"
                  size="sm"
                  variant="secondary"
                  :disabled="!boxRef(box)"
                  @click="update(box)"
                >
                  <ArrowUpCircle /> Update
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  :disabled="!boxRef(box)"
                  :aria-label="`Remove ${box.name} ${box.version}`"
                  @click="confirmRemove(box)"
                >
                  <Trash2 /> Remove
                </Button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </Card>
    <p v-if="boxes.length" class="text-xs text-muted">
      Updating downloads the newest version next to the installed one. Existing machines keep the
      version they were created with; remove the old version once nothing uses it.
    </p>
  </div>

  <AddBoxDialog v-model:open="adding" />
  <RemoveBoxDialog
    v-if="removing"
    v-model:open="removeOpen"
    :box="removing"
    :usage-error="store?.usageError ?? null"
  />
</template>
