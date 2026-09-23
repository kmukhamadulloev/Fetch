import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { useRuntimeStore } from './runtime'
import type { RuntimeComponent } from '@/app/api/client'

beforeEach(() => setActivePinia(createPinia()))
afterEach(() => vi.unstubAllGlobals())

it('shares overlapping reads and preserves newer events when a REST snapshot finishes later', async () => {
  let resolve!: (response: Response) => void
  const pending = new Promise<Response>((done) => { resolve = done })
  vi.stubGlobal('fetch', vi.fn((path: string) => path === '/api/runtime'
    ? pending : Promise.resolve(new Response('[]'))))
  const runtime = useRuntimeStore()
  const refresh = runtime.refresh()
  const overlapping = runtime.refresh()
  expect(fetch).toHaveBeenCalledTimes(2)
  const ready: RuntimeComponent = { name: 'yt-dlp', version: '1', status: 'ready', error: null, progress_percent: null }
  runtime.applyEvent(ready)
  resolve(new Response(JSON.stringify([
    { ...ready, status: 'missing' }, { ...ready, name: 'ffmpeg' },
  ])))
  await Promise.all([refresh, overlapping])
  expect(runtime.ytdlp?.status).toBe('ready')
  expect(runtime.components.find((item) => item.name === 'ffmpeg')?.status).toBe('ready')
})
