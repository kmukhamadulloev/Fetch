import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useProcessesStore } from './processes'
import { useDownloadsStore } from './downloads'
import { useLibraryStore } from './library'
import { useRealtimeStore } from './realtime'
import { useTelegramStore } from './telegram'
import { useStatusStore } from './status'
import { useRuntimeStore } from './runtime'
import { useSettingsStore } from './settings'
import { useProxyStore } from './proxy'
import { flushPromises } from '@vue/test-utils'
import type { CompletedFile, DownloadJob, TelegramIntegration } from '@/app/api/client'

const fixtureJob: DownloadJob = {
  id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310',
  url: 'https://example.test/media',
  title: 'Media',
  mode: 'video',
  embed_metadata: false,
  embed_thumbnail: false,
  subtitles: false,
  status: 'downloading',
  progress_percent: 42,
  downloaded_bytes: 42,
  total_bytes: 100,
  speed_bytes_per_second: 10,
  eta_seconds: 5,
  error_code: null,
  error_message: null,
  created_at: '2026-08-09T00:00:00Z',
  updated_at: '2026-08-09T00:00:00Z',
}
const fixtureFile: CompletedFile = {
  id: 'af2bf705-8425-4178-9c5c-805e62db4f64',
  job_id: fixtureJob.id,
  playlist: null,
  filename: 'media.mp4',
  thumbnail_available: true,
  size_bytes: 100,
  mime_type: 'video/mp4',
  title: 'Media',
  browser_playable: true,
  playback: null,
  created_at: '2026-08-09T00:00:00Z',
}
const fixtureTelegram: TelegramIntegration = {
  settings: {
    enabled: true,
    use_proxy: true,
    send_completed_media: true,
    upload_limit_mb: 25,
    allowed_user_ids: [123],
    notify_queued: false,
    notify_completed: true,
    notify_failed: true,
    privacy_acknowledged: true,
  },
  status: {
    state: 'connecting', failed_attempts: 0, retry_at: null,
    token_configured: true,
    token_source: 'native',
    bot_username: null,
    last_success_at: null,
    error: null,
  },
}

class TestBroadcastChannel extends EventTarget {
  static instances: TestBroadcastChannel[] = []
  postMessage = vi.fn()
  close = vi.fn()

  constructor() {
    super()
    TestBroadcastChannel.instances.push(this)
  }
}

class TestEventSource extends EventTarget {
  static instances: TestEventSource[] = []
  onopen: ((event: Event) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  close = vi.fn()

  constructor(public readonly url: string) {
    super()
    TestEventSource.instances.push(this)
  }
}

beforeEach(() => {
  setActivePinia(createPinia())
  const values = new Map<string, string>()
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      removeItem: (key: string) => values.delete(key),
      clear: () => values.clear(),
    },
  })
  TestEventSource.instances = []
  TestBroadcastChannel.instances = []
  vi.useFakeTimers()
  vi.stubGlobal('BroadcastChannel', TestBroadcastChannel)
  vi.stubGlobal('EventSource', TestEventSource)
  for (const store of [useStatusStore(), useRuntimeStore(), useSettingsStore(), useTelegramStore(), useProxyStore()]) {
    vi.spyOn(store, 'refresh').mockResolvedValue()
  }
  useSettingsStore().network = { local_client: false, bind_address: '127.0.0.1', port: 8080, urls: [], authentication: false, restart_required_after_bind_change: false }
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

describe('realtime coordinator', () => {
  it('owns one SSE connection and dispatches all event types through it', async () => {
    const realtime = useRealtimeStore()
    const downloads = useDownloadsStore()
    const telegram = useTelegramStore()
    telegram.value = fixtureTelegram
    realtime.start()
    await vi.advanceTimersByTimeAsync(100)

    expect(realtime.role).toBe('primary')
    expect(TestEventSource.instances).toHaveLength(1)
    const source = TestEventSource.instances[0]
    source.onopen?.(new Event('open'))
    source.dispatchEvent(new MessageEvent('download.progress', { data: JSON.stringify(fixtureJob) }))
    source.dispatchEvent(new MessageEvent('telegram.status', { data: JSON.stringify({
      ...fixtureTelegram.status,
      state: 'connected',
      bot_username: 'fetch_bot',
    }) }))

    expect(realtime.connection).toBe('connected')
    expect(downloads.jobs[0]?.progress_percent).toBe(42)
    source.dispatchEvent(new MessageEvent('process.updated', { data: JSON.stringify({ id: 'conversion', kind: 'conversion', state: 'running', progress_percent: 30, updated_at: '2026-09-23T00:00:00Z' }) }))
    expect(useProcessesStore().jobs[0]?.progress_percent).toBe(30)
    expect(telegram.value?.status.state).toBe('connected')
    expect(telegram.value?.status.bot_username).toBe('fetch_bot')
    expect(telegram.value?.settings.allowed_user_ids).toEqual([123])
    realtime.stop()
  })

  it('starts secondary and can explicitly take ownership from another tab', async () => {
    window.localStorage.setItem('fetch.realtime.primary', JSON.stringify({ tabId: 'other-tab', expiresAt: Date.now() + 10_000 }))
    const realtime = useRealtimeStore()
    realtime.start()

    expect(realtime.role).toBe('secondary')
    expect(TestEventSource.instances).toHaveLength(0)

    const takeover = realtime.takeOver()
    await vi.advanceTimersByTimeAsync(100)
    await takeover
    expect(realtime.role).toBe('primary')
    expect(TestEventSource.instances).toHaveLength(1)
    realtime.stop()
  })

  it('keeps realtime working when cross-tab broadcast is unavailable', () => {
    vi.stubGlobal('BroadcastChannel', undefined)
    const realtime = useRealtimeStore()
    realtime.start()
    expect(realtime.role).toBe('primary')
    expect(TestEventSource.instances).toHaveLength(1)
    realtime.stop()
  })

  it('refreshes snapshots on first open and reconnect without requesting host data on LAN', async () => {
    const realtime = useRealtimeStore()
    const runtime = useRuntimeStore()
    realtime.start()
    await vi.advanceTimersByTimeAsync(100)
    const source = TestEventSource.instances[0]
    source.onopen?.(new Event('open'))
    await flushPromises()
    expect(runtime.refresh).toHaveBeenCalledOnce()
    expect(useStatusStore().refresh).toHaveBeenCalledOnce()
    expect(useTelegramStore().refresh).not.toHaveBeenCalled()
    expect(useProxyStore().refresh).not.toHaveBeenCalled()
    source.onerror?.(new Event('error'))
    useSettingsStore().network!.local_client = true
    source.onopen?.(new Event('open'))
    await flushPromises()
    expect(runtime.refresh).toHaveBeenCalledTimes(2)
    expect(useTelegramStore().refresh).toHaveBeenCalledOnce()
    expect(useProxyStore().refresh).toHaveBeenCalledOnce()
    realtime.stop()
  })

  it('recovers a failed initial snapshot automatically and cancels retries on stop', async () => {
    const realtime = useRealtimeStore()
    const runtime = useRuntimeStore()
    vi.mocked(runtime.refresh).mockImplementation(async () => { runtime.error = 'Temporary failure' })
    realtime.start()
    await vi.advanceTimersByTimeAsync(100)
    TestEventSource.instances[0].onopen?.(new Event('open'))
    await flushPromises()
    expect(runtime.refresh).toHaveBeenCalledOnce()
    await vi.advanceTimersByTimeAsync(5000)
    expect(runtime.refresh).toHaveBeenCalledTimes(2)
    realtime.stop()
    await vi.advanceTimersByTimeAsync(10000)
    expect(runtime.refresh).toHaveBeenCalledTimes(2)
  })

  it('requests the real primary state instead of assuming an existing lease means connected', async () => {
    window.localStorage.setItem('fetch.realtime.primary', JSON.stringify({ tabId: 'other-tab', expiresAt: Date.now() + 10000 }))
    const realtime = useRealtimeStore()
    realtime.start()
    expect(realtime.connection).toBe('connecting')
    const channel = TestBroadcastChannel.instances[0]
    expect(channel.postMessage).toHaveBeenCalledWith(expect.objectContaining({ type: 'request-state' }))
    const state = (sender: string, state: string) => channel.dispatchEvent(new MessageEvent('message', {
      data: { type: 'state', sender, state, error: null },
    }))
    state('stale-tab', 'connected')
    expect(realtime.connection).toBe('connecting')
    state('other-tab', 'reconnecting')
    expect(realtime.connection).toBe('reconnecting')
    state('other-tab', 'connected')
    await flushPromises()
    expect(realtime.connection).toBe('connected')
    expect(useRuntimeStore().refresh).toHaveBeenCalledOnce()
    state('other-tab', 'connected')
    await flushPromises()
    expect(useRuntimeStore().refresh).toHaveBeenCalledOnce()
    realtime.stop()
  })

  it('answers late-joining tabs and ignores callbacks from a replaced stream', async () => {
    const realtime = useRealtimeStore()
    realtime.start()
    await vi.advanceTimersByTimeAsync(100)
    const old = TestEventSource.instances[0]
    old.onerror?.(new Event('error'))
    const channel = TestBroadcastChannel.instances[0]
    channel.postMessage.mockClear()
    channel.dispatchEvent(new MessageEvent('message', { data: { type: 'request-state', sender: 'late-tab' } }))
    expect(channel.postMessage).toHaveBeenCalledWith(expect.objectContaining({ type: 'state', state: 'reconnecting' }))
    realtime.retry()
    const current = TestEventSource.instances[1]
    current.onopen?.(new Event('open'))
    old.onerror?.(new Event('error'))
    expect(realtime.connection).toBe('connected')
    old.dispatchEvent(new MessageEvent('telegram.status', { data: 'invalid' }))
    expect(realtime.connection).toBe('connected')
    realtime.stop()
  })

  it('refreshes when a sleeping page becomes visible again', async () => {
    const realtime = useRealtimeStore()
    realtime.start()
    await vi.advanceTimersByTimeAsync(100)
    TestEventSource.instances[0].onopen?.(new Event('open'))
    await flushPromises()
    vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('visible')
    document.dispatchEvent(new Event('visibilitychange'))
    await flushPromises()
    expect(useRuntimeStore().refresh).toHaveBeenCalledTimes(2)
    realtime.stop()
    document.dispatchEvent(new Event('visibilitychange'))
    await flushPromises()
    expect(useRuntimeStore().refresh).toHaveBeenCalledTimes(2)
  })

  it('merges completed files for direct and relayed library events', async () => {
    const realtime = useRealtimeStore()
    const library = useLibraryStore()
    realtime.start()
    await vi.advanceTimersByTimeAsync(100)

    const completed = { ...fixtureJob, status: 'completed', progress_percent: 100 } satisfies DownloadJob
    TestEventSource.instances[0].dispatchEvent(new MessageEvent('download.completed', { data: JSON.stringify(completed) }))
    TestEventSource.instances[0].dispatchEvent(new MessageEvent('library.completed', { data: JSON.stringify(fixtureFile) }))
    TestEventSource.instances[0].dispatchEvent(new MessageEvent('library.progress', { data: JSON.stringify({ file_id: fixtureFile.id, position_seconds: 50, duration_seconds: 100, completed: false, updated_at: '2026-08-09T01:00:00Z' }) }))
    expect(library.history[0]?.status).toBe('completed')
    expect(library.completed[0]).toEqual(expect.objectContaining({ id: fixtureFile.id }))
    expect(library.completed[0].playback?.position_seconds).toBe(50)
    realtime.stop()

    setActivePinia(createPinia())
    window.localStorage.setItem('fetch.realtime.primary', JSON.stringify({ tabId: 'other-tab', expiresAt: Date.now() + 10_000 }))
    const secondaryRealtime = useRealtimeStore()
    const secondaryLibrary = useLibraryStore()
    secondaryRealtime.start()
    TestBroadcastChannel.instances.at(-1)?.dispatchEvent(new MessageEvent('message', { data: {
      type: 'event', sender: 'other-tab', name: 'library.completed', data: JSON.stringify(fixtureFile),
    } }))
    expect(secondaryLibrary.completed).toEqual([fixtureFile])
    secondaryRealtime.stop()
  })
})
