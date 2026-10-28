import { describe, expect, it } from 'vitest'
import {
  cloneModel,
  errorsByPath,
  move,
  newForwardedPort,
  newMachine,
  optionalNumber,
  optionalText,
  sameModel,
  starterModel,
} from './vagrantfile'

describe('starterModel', () => {
  it('uses the default box and the environment provider when the editor supports it', () => {
    const vmware = starterModel('vmware_desktop')
    expect(vmware.defaults.box).toBe('bento/ubuntu-24.04')
    expect(vmware.defaults.providers).toEqual([
      { provider: 'vmware_desktop', cpus: 2, memoryMb: 2048 },
    ])
    expect(starterModel('libvirt').defaults.providers).toEqual([])
    expect(starterModel(null).machines).toEqual([])
  })
})

describe('model helpers', () => {
  it('names new machines without clashing, ignoring case', () => {
    const config = starterModel(null)
    config.machines.push(newMachine(config.machines))
    config.machines.push({ ...newMachine(config.machines), name: 'MACHINE3' })
    expect(config.machines.map((m) => m.name)).toEqual(['machine1', 'MACHINE3'])
    expect(newMachine(config.machines).name).toBe('machine4')
  })

  it('picks a free host port across all machines', () => {
    const config = starterModel(null)
    config.defaults.forwardedPorts.push(newForwardedPort(config))
    const machine = newMachine(config.machines)
    machine.settings.forwardedPorts.push({ guest: 1, host: 8081, hostIp: null, protocol: null })
    config.machines.push(machine)
    expect(config.defaults.forwardedPorts[0]!.host).toBe(8080)
    expect(newForwardedPort(config).host).toBe(8082)
  })

  it('keeps the first error per path', () => {
    expect(
      errorsByPath([
        { path: 'a', message: 'one' },
        { path: 'a', message: 'two' },
        { path: 'b', message: 'three' },
      ]),
    ).toEqual({ a: 'one', b: 'three' })
  })

  it('moves items within bounds only', () => {
    const list = ['a', 'b', 'c']
    move(list, 0, -1)
    expect(list).toEqual(['a', 'b', 'c'])
    move(list, 0, 1)
    expect(list).toEqual(['b', 'a', 'c'])
    move(list, 2, 1)
    expect(list).toEqual(['b', 'a', 'c'])
  })

  it('compares and copies models structurally', () => {
    const a = starterModel('virtualbox')
    const b = cloneModel(a)
    expect(sameModel(a, b)).toBe(true)
    b.defaults.providers[0]!.cpus = 4
    expect(sameModel(a, b)).toBe(false)
    expect(a.defaults.providers[0]!.cpus).toBe(2)
  })

  it('maps empty inputs to unset values', () => {
    expect(optionalText('  ')).toBeNull()
    expect(optionalText('web')).toBe('web')
    expect(optionalNumber('')).toBeNull()
    expect(optionalNumber('2.5')).toBeNull()
    expect(optionalNumber(' 2048 ')).toBe(2048)
  })
})
