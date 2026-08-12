import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useProxyStore } from './proxy'

beforeEach(() => setActivePinia(createPinia()))

describe('proxy store', () => {
  it('loads and saves typed host proxy settings', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ mode: 'system', url: null }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ mode: 'custom', url: 'socks5://127.0.0.1:1080' }), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    const store = useProxyStore()

    await store.refresh()
    expect(store.value).toEqual({ mode: 'system', url: null })
    expect(await store.save({ mode: 'custom', url: 'socks5://127.0.0.1:1080' })).toBe(true)
    expect(fetchMock.mock.calls[1][0]).toBe('/api/proxy')
    expect(fetchMock.mock.calls[1][1].method).toBe('PUT')
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({ mode: 'custom', url: 'socks5://127.0.0.1:1080' })
    expect(store.saved).toBe(true)
    vi.unstubAllGlobals()
  })

  it('surfaces a typed validation error without changing the saved value', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: { message: 'authenticated proxies are not supported in this version' } }), { status: 400 })))
    const store = useProxyStore()
    store.value = { mode: 'system', url: null }

    expect(await store.save({ mode: 'custom', url: 'http://user:secret@proxy.test:8080' })).toBe(false)
    expect(store.value).toEqual({ mode: 'system', url: null })
    expect(store.error).toContain('authenticated proxies')
    vi.unstubAllGlobals()
  })
})
