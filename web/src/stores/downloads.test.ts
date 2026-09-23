import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useDownloadsStore } from './downloads'
import type { DownloadJob } from '@/app/api/client'

const job = (status: DownloadJob['status'], progress = 0): DownloadJob => ({
  id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310',
  url: 'https://example.test/media',
  title: 'Media',
  mode: 'video',
  embed_metadata: false,
  embed_thumbnail: false,
  subtitles: false,
  status,
  progress_percent: progress,
  downloaded_bytes: progress,
  total_bytes: 100,
  speed_bytes_per_second: 10,
  eta_seconds: 5,
  error_code: null,
  error_message: null,
  created_at: '2026-08-09T00:00:00Z',
  updated_at: '2026-08-09T00:00:00Z',
})

beforeEach(() => setActivePinia(createPinia()))

describe('downloads store', () => {
  it('maps relative API data and merges relayed progress', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify([job('queued')]), { status: 200 })))
    const store = useDownloadsStore()
    await store.refresh()
    store.applyEvent(job('downloading', 25))
    expect(store.jobs).toHaveLength(1)
    expect(store.jobs[0].status).toBe('downloading')
    expect(store.jobs[0].progress_percent).toBe(25)
    vi.unstubAllGlobals()
  })
})

it('preserves progress received while a reconnect snapshot is pending', async () => {
  let respond!: (response: Response) => void
  vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(resolve => { respond = resolve })))
  const store = useDownloadsStore()
  const pending = store.refresh()
  store.applyEvent(job('downloading', 60))
  respond(new Response(JSON.stringify([job('queued')])))
  await pending
  expect(store.jobs[0].progress_percent).toBe(60)
  vi.unstubAllGlobals()
})
