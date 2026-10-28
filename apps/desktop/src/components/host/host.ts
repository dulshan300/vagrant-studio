import type { HostInfo } from '@/types/app'

const OS_NAMES: Record<HostInfo['os'], string> = {
  windows: 'Windows',
  linux: 'Linux',
  macos: 'macOS',
  other: 'Unknown OS',
}

const ARCH_NAMES: Record<HostInfo['arch'], string> = {
  x86_64: 'x64',
  aarch64: 'ARM64',
  other: 'unknown architecture',
}

export function describeHost(host: HostInfo): string {
  const os = host.os === 'linux' && host.isWsl ? 'Linux (WSL)' : OS_NAMES[host.os]
  return `${os} · ${ARCH_NAMES[host.arch]}`
}
