<script setup lang="ts">
import { computed } from 'vue'
import { ArrowDown, ArrowUp, Plus, Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import FormField from '@/components/ui/form/FormField.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
import {
  PROVIDER_LABELS,
  move,
  newProvisioner,
  optionalNumber,
  optionalText,
} from '@/lib/vagrantfile'
import type {
  ForwardedPort,
  MachineSettings,
  ProviderKind,
  ProvisionRun,
  ShellProvisioner,
} from '@/types/vagrantfile'

const settings = defineModel<MachineSettings>('settings', { required: true })
const props = defineProps<{
  /** Field path prefix for errors, e.g. `defaults` or `machines.0.settings`. */
  path: string
  /** Unique prefix for input ids. */
  idPrefix: string
  errors: Record<string, string>
  /** Values from the shared settings, shown as placeholders for a machine. */
  inherited?: MachineSettings | null
  newForwardedPort: () => ForwardedPort
}>()

const err = (field: string) => props.errors[`${props.path}.${field}`] ?? null
const id = (field: string) => `${props.idPrefix}-${field}`

const providerKinds = Object.keys(PROVIDER_LABELS) as ProviderKind[]
const unusedProvider = computed(() =>
  providerKinds.find((p) => !settings.value.providers.some((s) => s.provider === p)),
)

const numberInput = 'w-full rounded-md border bg-surface px-2 py-1.5 text-sm'
const selectInput = 'rounded-md border bg-surface px-2 py-1.5 text-sm'
const sectionTitle = 'flex items-center justify-between gap-2 text-sm font-medium'

function invalid(field: string) {
  return err(field) ? 'border-status-failed' : ''
}

function setSource(p: ShellProvisioner, kind: 'inline' | 'path') {
  if (p.source.kind === kind) return
  p.source = kind === 'inline' ? { kind, script: '' } : { kind, path: '' }
}

function parseRun(value: string): ProvisionRun | null {
  return value === 'once' || value === 'always' || value === 'never' ? value : null
}

function parsePrivileged(value: string): boolean | null {
  return value === '' ? null : value === 'true'
}
</script>

<template>
  <div class="space-y-6">
    <div class="grid gap-4 sm:grid-cols-2">
      <FormField label="Box" :for="id('box')" :error="err('box')">
        <TextInput
          :id="id('box')"
          :model-value="settings.box ?? ''"
          :placeholder="inherited?.box ? `Shared: ${inherited.box}` : 'bento/ubuntu-24.04'"
          :invalid="!!err('box')"
          spellcheck="false"
          @update:model-value="settings.box = optionalText($event)"
        />
      </FormField>
      <FormField label="Hostname" :for="id('hostname')" :error="err('hostname')">
        <TextInput
          :id="id('hostname')"
          :model-value="settings.hostname ?? ''"
          :placeholder="inherited?.hostname ? `Shared: ${inherited.hostname}` : 'Not set'"
          :invalid="!!err('hostname')"
          spellcheck="false"
          @update:model-value="settings.hostname = optionalText($event)"
        />
      </FormField>
    </div>

    <section class="space-y-2">
      <h3 :class="sectionTitle">
        CPU and memory
        <Button
          v-if="unusedProvider"
          variant="ghost"
          size="sm"
          @click="settings.providers.push({ provider: unusedProvider, cpus: null, memoryMb: null })"
        >
          <Plus /> {{ PROVIDER_LABELS[unusedProvider] }}
        </Button>
      </h3>
      <p v-if="!settings.providers.length" class="text-xs text-muted">
        Not set: the provider's defaults apply{{
          inherited?.providers.length ? ' (or the shared settings)' : ''
        }}.
      </p>
      <div
        v-for="(p, i) in settings.providers"
        :key="p.provider"
        class="grid grid-cols-[minmax(0,1fr)_6rem_8rem_auto] items-end gap-2"
      >
        <div class="pb-2 text-sm">{{ PROVIDER_LABELS[p.provider] }}</div>
        <FormField label="CPUs" :for="id(`p${i}-cpus`)" :error="err(`providers.${i}.cpus`)">
          <input
            :id="id(`p${i}-cpus`)"
            type="number"
            min="1"
            :value="p.cpus ?? ''"
            placeholder="Default"
            :class="[numberInput, invalid(`providers.${i}.cpus`)]"
            @input="p.cpus = optionalNumber(($event.target as HTMLInputElement).value)"
          />
        </FormField>
        <FormField
          label="Memory (MB)"
          :for="id(`p${i}-mem`)"
          :error="err(`providers.${i}.memoryMb`)"
        >
          <input
            :id="id(`p${i}-mem`)"
            type="number"
            min="64"
            step="256"
            :value="p.memoryMb ?? ''"
            placeholder="Default"
            :class="[numberInput, invalid(`providers.${i}.memoryMb`)]"
            @input="p.memoryMb = optionalNumber(($event.target as HTMLInputElement).value)"
          />
        </FormField>
        <Button
          variant="ghost"
          size="icon"
          class="size-8 text-muted"
          :aria-label="`Remove ${PROVIDER_LABELS[p.provider]} settings`"
          @click="settings.providers.splice(i, 1)"
        >
          <Trash2 />
        </Button>
      </div>
    </section>

    <section class="space-y-2">
      <h3 :class="sectionTitle">
        Private networks
        <Button variant="ghost" size="sm" @click="settings.privateNetworks.push({ ip: '' })">
          <Plus /> Add
        </Button>
      </h3>
      <div
        v-for="(n, i) in settings.privateNetworks"
        :key="i"
        class="grid grid-cols-[9rem_minmax(0,1fr)_auto] items-start gap-2"
      >
        <select
          :value="n.ip === null ? 'dhcp' : 'static'"
          :class="selectInput"
          :aria-label="`Network ${i + 1} addressing`"
          @change="n.ip = ($event.target as HTMLSelectElement).value === 'dhcp' ? null : ''"
        >
          <option value="static">Static IP</option>
          <option value="dhcp">DHCP</option>
        </select>
        <div v-if="n.ip !== null" class="space-y-1">
          <TextInput
            v-model="n.ip"
            placeholder="192.168.56.10"
            :aria-label="`Network ${i + 1} IP address`"
            :invalid="!!err(`privateNetworks.${i}.ip`)"
            spellcheck="false"
          />
          <p v-if="err(`privateNetworks.${i}.ip`)" class="text-xs text-status-failed" role="alert">
            {{ err(`privateNetworks.${i}.ip`) }}
          </p>
        </div>
        <p v-else class="py-2 text-xs text-muted">The provider assigns an address.</p>
        <Button
          variant="ghost"
          size="icon"
          class="size-8 text-muted"
          :aria-label="`Remove network ${i + 1}`"
          @click="settings.privateNetworks.splice(i, 1)"
        >
          <Trash2 />
        </Button>
      </div>
    </section>

    <section class="space-y-2">
      <h3 :class="sectionTitle">
        Forwarded ports
        <Button
          variant="ghost"
          size="sm"
          @click="settings.forwardedPorts.push(props.newForwardedPort())"
        >
          <Plus /> Add
        </Button>
      </h3>
      <div
        v-if="settings.forwardedPorts.length"
        class="grid grid-cols-[5.5rem_5.5rem_minmax(0,1fr)_5.5rem_auto] gap-2 text-xs text-muted"
      >
        <span>Guest port</span><span>Host port</span><span>Host IP</span><span>Protocol</span>
      </div>
      <div v-for="(p, i) in settings.forwardedPorts" :key="i" class="space-y-1">
        <div class="grid grid-cols-[5.5rem_5.5rem_minmax(0,1fr)_5.5rem_auto] items-center gap-2">
          <input
            type="number"
            min="1"
            max="65535"
            :value="p.guest || ''"
            :aria-label="`Port ${i + 1} guest port`"
            :class="[numberInput, invalid(`forwardedPorts.${i}.guest`)]"
            @input="p.guest = optionalNumber(($event.target as HTMLInputElement).value) ?? 0"
          />
          <input
            type="number"
            min="1"
            max="65535"
            :value="p.host || ''"
            :aria-label="`Port ${i + 1} host port`"
            :class="[numberInput, invalid(`forwardedPorts.${i}.host`)]"
            @input="p.host = optionalNumber(($event.target as HTMLInputElement).value) ?? 0"
          />
          <TextInput
            :model-value="p.hostIp ?? ''"
            placeholder="All interfaces"
            :aria-label="`Port ${i + 1} host IP`"
            :invalid="!!err(`forwardedPorts.${i}.hostIp`)"
            spellcheck="false"
            @update:model-value="p.hostIp = optionalText($event)"
          />
          <select
            :value="p.protocol ?? 'tcp'"
            :class="selectInput"
            :aria-label="`Port ${i + 1} protocol`"
            @change="
              p.protocol = ($event.target as HTMLSelectElement).value === 'udp' ? 'udp' : null
            "
          >
            <option value="tcp">TCP</option>
            <option value="udp">UDP</option>
          </select>
          <Button
            variant="ghost"
            size="icon"
            class="size-8 text-muted"
            :aria-label="`Remove port ${i + 1}`"
            @click="settings.forwardedPorts.splice(i, 1)"
          >
            <Trash2 />
          </Button>
        </div>
        <p
          v-for="field in ['guest', 'host', 'hostIp']"
          v-show="err(`forwardedPorts.${i}.${field}`)"
          :key="field"
          class="text-xs text-status-failed"
          role="alert"
        >
          {{ err(`forwardedPorts.${i}.${field}`) }}
        </p>
      </div>
    </section>

    <section class="space-y-2">
      <h3 :class="sectionTitle">
        Synced folders
        <Button
          variant="ghost"
          size="sm"
          @click="settings.syncedFolders.push({ hostPath: '.', guestPath: '', disabled: false })"
        >
          <Plus /> Add
        </Button>
      </h3>
      <p v-if="!settings.syncedFolders.length" class="text-xs text-muted">
        Vagrant's default applies: the environment folder is synced to /vagrant.
      </p>
      <div v-for="(f, i) in settings.syncedFolders" :key="i" class="space-y-1">
        <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto_auto] items-center gap-2">
          <TextInput
            v-model="f.hostPath"
            placeholder="Host folder, e.g. ."
            :aria-label="`Folder ${i + 1} host path`"
            :invalid="!!err(`syncedFolders.${i}.hostPath`)"
            spellcheck="false"
          />
          <TextInput
            v-model="f.guestPath"
            placeholder="Guest folder, e.g. /vagrant"
            :aria-label="`Folder ${i + 1} guest path`"
            :invalid="!!err(`syncedFolders.${i}.guestPath`)"
            spellcheck="false"
          />
          <label class="flex items-center gap-1.5 text-xs whitespace-nowrap">
            <input v-model="f.disabled" type="checkbox" /> Disabled
          </label>
          <Button
            variant="ghost"
            size="icon"
            class="size-8 text-muted"
            :aria-label="`Remove folder ${i + 1}`"
            @click="settings.syncedFolders.splice(i, 1)"
          >
            <Trash2 />
          </Button>
        </div>
        <p
          v-for="field in ['hostPath', 'guestPath']"
          v-show="err(`syncedFolders.${i}.${field}`)"
          :key="field"
          class="text-xs text-status-failed"
          role="alert"
        >
          {{ err(`syncedFolders.${i}.${field}`) }}
        </p>
      </div>
    </section>

    <section class="space-y-2">
      <h3 :class="sectionTitle">
        Shell provisioners
        <Button variant="ghost" size="sm" @click="settings.provisioners.push(newProvisioner())">
          <Plus /> Add
        </Button>
      </h3>
      <div
        v-for="(p, i) in settings.provisioners"
        :key="i"
        class="space-y-2 rounded-md border bg-surface-muted/40 p-3"
      >
        <div class="flex flex-wrap items-center gap-2">
          <select
            :value="p.source.kind"
            :class="selectInput"
            :aria-label="`Provisioner ${i + 1} source`"
            @change="
              setSource(
                p,
                ($event.target as HTMLSelectElement).value === 'path' ? 'path' : 'inline',
              )
            "
          >
            <option value="inline">Inline script</option>
            <option value="path">Script file</option>
          </select>
          <select
            :value="p.run ?? ''"
            :class="selectInput"
            :aria-label="`Provisioner ${i + 1} runs`"
            @change="p.run = parseRun(($event.target as HTMLSelectElement).value)"
          >
            <option value="">Run once (default)</option>
            <option value="once">Run once (set explicitly)</option>
            <option value="always">Run on every up and reload</option>
            <option value="never">Run only with --provision-with</option>
          </select>
          <select
            :value="p.privileged === null ? '' : String(p.privileged)"
            :class="selectInput"
            :aria-label="`Provisioner ${i + 1} privileges`"
            @change="p.privileged = parsePrivileged(($event.target as HTMLSelectElement).value)"
          >
            <option value="">As root (default)</option>
            <option value="true">As root (set explicitly)</option>
            <option value="false">As the SSH user</option>
          </select>
          <span class="flex-1" />
          <Button
            variant="ghost"
            size="icon"
            class="size-8 text-muted"
            :aria-label="`Move provisioner ${i + 1} up`"
            :disabled="i === 0"
            @click="move(settings.provisioners, i, -1)"
          >
            <ArrowUp />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-8 text-muted"
            :aria-label="`Move provisioner ${i + 1} down`"
            :disabled="i === settings.provisioners.length - 1"
            @click="move(settings.provisioners, i, 1)"
          >
            <ArrowDown />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-8 text-muted"
            :aria-label="`Remove provisioner ${i + 1}`"
            @click="settings.provisioners.splice(i, 1)"
          >
            <Trash2 />
          </Button>
        </div>
        <template v-if="p.source.kind === 'inline'">
          <TextInput
            v-model="p.source.script"
            multiline
            class="min-h-28 font-mono text-xs"
            :aria-label="`Provisioner ${i + 1} script`"
            :invalid="!!err(`provisioners.${i}.source.script`)"
            spellcheck="false"
          />
          <p
            v-if="err(`provisioners.${i}.source.script`)"
            class="text-xs text-status-failed"
            role="alert"
          >
            {{ err(`provisioners.${i}.source.script`) }}
          </p>
        </template>
        <template v-else>
          <TextInput
            v-model="p.source.path"
            placeholder="scripts/setup.sh (relative to the Vagrantfile)"
            :aria-label="`Provisioner ${i + 1} script file`"
            :invalid="!!err(`provisioners.${i}.source.path`)"
            spellcheck="false"
          />
          <p
            v-if="err(`provisioners.${i}.source.path`)"
            class="text-xs text-status-failed"
            role="alert"
          >
            {{ err(`provisioners.${i}.source.path`) }}
          </p>
        </template>
      </div>
    </section>
  </div>
</template>
