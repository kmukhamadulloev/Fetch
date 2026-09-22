import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { useDownloadsStore } from '@/stores/downloads'
import { useLibraryStore } from '@/stores/library'
import { useRuntimeStore } from '@/stores/runtime'
import { useTelegramStore } from '@/stores/telegram'
import type { TelegramStatus } from '@/app/api/client'

const channelName = 'fetch.realtime.v1'
const leaseKey = 'fetch.realtime.primary'
const leaseDurationMs = 6_000
const heartbeatMs = 2_000
const electionDelayMs = 80

const downloadEvents = new Set([
  'download.created',
  'download.progress',
  'download.postprocessing',
  'download.completed',
  'download.failed',
  'download.stopped',
])
const runtimeEvents = new Set([
  'runtime.installing',
  'runtime.updating',
  'runtime.ready',
  'runtime.failed',
  'runtime.missing',
])
const libraryEvents = new Set(['library.completed', 'library.progress', 'library.progress-cleared', 'library.metadata'])
const telegramEvents = new Set(['telegram.status'])
const eventNames = [...downloadEvents, ...runtimeEvents, ...libraryEvents, ...telegramEvents]

type RealtimeRole = 'electing' | 'primary' | 'secondary'
type ConnectionState = 'connecting' | 'connected' | 'reconnecting' | 'offline'
type Lease = { tabId: string; expiresAt: number }
type ChannelMessage =
  | { type: 'event'; sender: string; name: string; data: string }
  | { type: 'state'; sender: string; state: ConnectionState; error: string | null }
  | { type: 'takeover'; sender: string }
  | { type: 'released'; sender: string }

function createTabId() {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

export const useRealtimeStore = defineStore('realtime', () => {
  const downloads = useDownloadsStore()
  const library = useLibraryStore()
  const runtime = useRuntimeStore()
  const telegram = useTelegramStore()
  const tabId = createTabId()
  const role = ref<RealtimeRole>('electing')
  const connection = ref<ConnectionState>('connecting')
  const error = ref<string | null>(null)
  const primaryTabId = ref<string | null>(null)
  const isPrimary = computed(() => role.value === 'primary')

  let started = false
  let source: EventSource | null = null
  let channel: BroadcastChannel | null = null
  let heartbeat: ReturnType<typeof setInterval> | null = null
  let electionSequence = 0

  function readLease(): Lease | null {
    try {
      const value = window.localStorage.getItem(leaseKey)
      if (!value) return null
      const lease = JSON.parse(value) as Partial<Lease>
      if (typeof lease.tabId !== 'string' || typeof lease.expiresAt !== 'number') return null
      return lease as Lease
    } catch {
      return null
    }
  }

  function writeLease() {
    try {
      window.localStorage.setItem(leaseKey, JSON.stringify({ tabId, expiresAt: Date.now() + leaseDurationMs }))
      return true
    } catch {
      return false
    }
  }

  function removeOwnLease() {
    try {
      if (readLease()?.tabId === tabId) window.localStorage.removeItem(leaseKey)
    } catch { /* A private browser context may deny storage access. */ }
  }

  function post(message: ChannelMessage) {
    channel?.postMessage(message)
  }

  function setConnection(state: ConnectionState, message: string | null = null) {
    connection.value = state
    error.value = message
    if (role.value === 'primary') post({ type: 'state', sender: tabId, state, error: message })
  }

  function applyEvent(name: string, data: string) {
    try {
      const payload = JSON.parse(data) as unknown
      if (downloadEvents.has(name)) {
        downloads.applyEvent(payload)
        library.applyDownload(payload)
      }
      else if (runtimeEvents.has(name)) runtime.applyEvent(payload)
      else if (name === 'library.metadata') library.applyMetadata(payload)
      else if (name === 'library.completed') library.applyCompleted(payload)
      else if (name === 'library.progress') library.applyPlayback(payload)
      else if (name === 'library.progress-cleared') library.clearPlayback(payload)
      else if (name === 'telegram.status') telegram.applyStatus(payload as TelegramStatus)
    } catch {
      setConnection('reconnecting', 'Fetch received an invalid realtime update. Reconnecting…')
    }
  }

  function closeSource() {
    source?.close()
    source = null
  }

  function openSource() {
    closeSource()
    setConnection('connecting')
    const next = new EventSource('/api/events')
    source = next
    next.onopen = () => setConnection('connected')
    next.onerror = () => setConnection('reconnecting', 'Realtime updates were interrupted. Fetch is reconnecting…')
    for (const name of eventNames) {
      next.addEventListener(name, (event) => {
        const data = (event as MessageEvent).data as string
        applyEvent(name, data)
        post({ type: 'event', sender: tabId, name, data })
      })
    }
  }

  function becomeSecondary(owner: string, state: ConnectionState = 'connected', message: string | null = null) {
    closeSource()
    role.value = 'secondary'
    primaryTabId.value = owner
    connection.value = state
    error.value = message
  }

  function becomePrimary() {
    role.value = 'primary'
    primaryTabId.value = tabId
    writeLease()
    openSource()
  }

  async function elect(force = false) {
    const sequence = ++electionSequence
    const current = readLease()
    if (!force && current && current.expiresAt > Date.now() && current.tabId !== tabId) {
      becomeSecondary(current.tabId)
      return
    }

    role.value = 'electing'
    primaryTabId.value = null
    if (!writeLease()) {
      // Storage can be disabled in hardened browser contexts. Realtime still
      // works there, but cross-tab coordination is unavailable.
      becomePrimary()
      return
    }
    await new Promise((resolve) => setTimeout(resolve, electionDelayMs))
    if (sequence !== electionSequence || !started) return
    const claimed = readLease()
    if (claimed?.tabId === tabId) becomePrimary()
    else if (claimed) becomeSecondary(claimed.tabId)
    else becomePrimary()
  }

  function checkLeadership() {
    const lease = readLease()
    if (role.value === 'primary') {
      if (lease && lease.tabId !== tabId && lease.expiresAt > Date.now()) {
        becomeSecondary(lease.tabId)
      } else {
        writeLease()
        if (!source) openSource()
      }
      return
    }
    if (!lease || lease.expiresAt <= Date.now()) void elect()
    else if (lease.tabId !== tabId) primaryTabId.value = lease.tabId
  }

  function handleMessage(event: MessageEvent<ChannelMessage>) {
    const message = event.data
    if (!message || message.sender === tabId) return
    if (message.type === 'event' && role.value !== 'primary') applyEvent(message.name, message.data)
    if (message.type === 'state' && role.value !== 'primary') {
      primaryTabId.value = message.sender
      connection.value = message.state
      error.value = message.error
    }
    if (message.type === 'takeover') becomeSecondary(message.sender, 'connecting')
    if (message.type === 'released') setTimeout(checkLeadership, electionDelayMs)
  }

  function handleStorage(event: StorageEvent) {
    if (event.key === leaseKey) checkLeadership()
  }

  function release() {
    if (role.value === 'primary') {
      removeOwnLease()
      post({ type: 'released', sender: tabId })
    }
    closeSource()
  }

  function start() {
    if (started) return
    started = true
    window.addEventListener('pagehide', release)
    window.addEventListener('pageshow', checkLeadership)
    if (typeof BroadcastChannel === 'undefined') {
      // Older embedded browsers cannot relay events between tabs. Preserve
      // realtime correctness there by using an independent stream per tab.
      becomePrimary()
      return
    }
    channel = new BroadcastChannel(channelName)
    channel.addEventListener('message', handleMessage)
    window.addEventListener('storage', handleStorage)
    heartbeat = setInterval(checkLeadership, heartbeatMs)
    void elect()
  }

  function stop() {
    started = false
    electionSequence += 1
    release()
    if (heartbeat) clearInterval(heartbeat)
    heartbeat = null
    window.removeEventListener('storage', handleStorage)
    window.removeEventListener('pagehide', release)
    window.removeEventListener('pageshow', checkLeadership)
    channel?.close()
    channel = null
  }

  async function takeOver() {
    closeSource()
    role.value = 'electing'
    connection.value = 'connecting'
    error.value = null
    writeLease()
    post({ type: 'takeover', sender: tabId })
    await elect(true)
  }

  function retry() {
    if (role.value === 'primary') openSource()
    else checkLeadership()
  }

  return {
    role,
    connection,
    error,
    primaryTabId,
    isPrimary,
    start,
    stop,
    takeOver,
    retry,
  }
})
