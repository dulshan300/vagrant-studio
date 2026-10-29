import type { Component } from 'vue'
import { Pause, Play, RotateCw, Square, StepForward, Trash2, Wrench } from '@lucide/vue'
import type { LifecycleAction } from '@/types/operations'

export interface ActionMeta {
  label: string
  icon: Component
  /** What the action does, shown as a tooltip. */
  hint: string
}

export const ACTION_META: Record<LifecycleAction, ActionMeta> = {
  up: { label: 'Start', icon: Play, hint: 'vagrant up: create or boot the machine' },
  halt: { label: 'Stop', icon: Square, hint: 'vagrant halt: shut the machine down' },
  reload: {
    label: 'Reload',
    icon: RotateCw,
    hint: 'vagrant reload: restart and apply Vagrantfile changes',
  },
  suspend: {
    label: 'Suspend',
    icon: Pause,
    hint: 'vagrant suspend: save the machine state to disk',
  },
  resume: {
    label: 'Resume',
    icon: StepForward,
    hint: 'vagrant resume: continue a suspended machine',
  },
  provision: {
    label: 'Provision',
    icon: Wrench,
    hint: 'vagrant provision: run all or chosen provisioners (can start or restart first)',
  },
  destroy: {
    label: 'Destroy',
    icon: Trash2,
    hint: 'vagrant destroy: delete the machine and its disks',
  },
}
