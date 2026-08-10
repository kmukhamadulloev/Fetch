import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useStatusStore } from './status'

beforeEach(() => setActivePinia(createPinia()))
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('status store', () => {
  it('maps the real relative status endpoint response', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ version: '0.1.0', server: 'ready', runtime_ready: false, storage_ready: true }), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    const store = useStatusStore()
    await store.refresh()
    expect(fetchMock).toHaveBeenCalledWith('/api/status', expect.any(Object))
    expect(store.value?.storage_ready).toBe(true)
    expect(store.error).toBeNull()
  })

  it('stops waiting and exposes an actionable error when the server hangs', async () => {
    vi.useFakeTimers()
    vi.stubGlobal('fetch', vi.fn((_path: string, init?: RequestInit) => new Promise((_resolve, reject) => {
      init?.signal?.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')))
    })))
    const store = useStatusStore()
    const refresh = store.refresh()
    await vi.advanceTimersByTimeAsync(10_001)
    await refresh
    expect(store.loading).toBe(false)
    expect(store.error).toBe('Fetch server did not respond within 10 seconds')
  })
})
