import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useDownloadsStore } from './downloads'
import { useRealtimeStore } from './realtime'
import type { DownloadJob } from '@/app/api/client'

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

class TestBroadcastChannel extends EventTarget {
  postMessage = vi.fn()
  close = vi.fn()
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
  vi.useFakeTimers()
  vi.stubGlobal('BroadcastChannel', TestBroadcastChannel)
  vi.stubGlobal('EventSource', TestEventSource)
})

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('realtime coordinator', () => {
  it('owns one SSE connection and dispatches all event types through it', async () => {
    const realtime = useRealtimeStore()
    const downloads = useDownloadsStore()
    realtime.start()
    await vi.advanceTimersByTimeAsync(100)

    expect(realtime.role).toBe('primary')
    expect(TestEventSource.instances).toHaveLength(1)
    const source = TestEventSource.instances[0]
    source.onopen?.(new Event('open'))
    source.dispatchEvent(new MessageEvent('download.progress', { data: JSON.stringify(fixtureJob) }))

    expect(realtime.connection).toBe('connected')
    expect(downloads.jobs[0]?.progress_percent).toBe(42)
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
})
