import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSettingsStore } from './settings'
import type { ApplicationSettings } from '@/app/api/client'

beforeEach(() => setActivePinia(createPinia()))

describe('settings store', () => {
  it('saves typed settings and refreshes advertised LAN state', async () => {
    const settings: ApplicationSettings = {
      bind_address: '0.0.0.0', port: 8080, allowed_networks: ['192.168.0.0/16'],
      download_directory: 'downloads', concurrent_downloads: 2,
      open_browser_on_start: false, ytdlp_auto_update: true,
    }
    const network = { bind_address: '0.0.0.0', port: 8080, urls: ['http://127.0.0.1:8080'], authentication: false, restart_required_after_bind_change: true }
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(settings), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(network), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    const store = useSettingsStore()
    await store.save(settings)
    expect(fetchMock.mock.calls[0][0]).toBe('/api/settings')
    expect(fetchMock.mock.calls[0][1].method).toBe('PUT')
    expect(store.saved).toBe(true)
    expect(store.network?.authentication).toBe(false)
    vi.unstubAllGlobals()
  })
})
