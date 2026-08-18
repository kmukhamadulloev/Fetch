import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { reconnectUrl, useSettingsStore } from './settings'
import type { ApplicationSettings } from '@/app/api/client'

beforeEach(() => setActivePinia(createPinia()))

describe('settings store', () => {
  it('saves typed settings and refreshes advertised LAN state', async () => {
    const settings: ApplicationSettings = {
      bind_address: '0.0.0.0', port: 8080, allowed_networks: ['192.168.0.0/16'],
      download_directory: 'downloads', concurrent_downloads: 2,
      open_browser_on_start: false, start_with_system: false, ytdlp_auto_update: true,
      ytdlp_js_runtime: 'auto',
    }
    const network = { bind_address: '0.0.0.0', port: 8080, urls: ['http://127.0.0.1:8080'], authentication: false, restart_required_after_bind_change: false }
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...settings, listener_changed: false }), { status: 200 }))
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

  it('builds the replacement URL for a hot listener change', () => {
    const settings: ApplicationSettings = {
      bind_address: '0.0.0.0', port: 9090, allowed_networks: ['192.168.0.0/16'],
      download_directory: 'downloads', concurrent_downloads: 2,
      open_browser_on_start: false, start_with_system: false, ytdlp_auto_update: true,
      ytdlp_js_runtime: 'auto',
    }
    expect(reconnectUrl(settings, 'http://192.168.1.20:8080/settings#network'))
      .toBe('http://192.168.1.20:9090/settings#network')
    expect(reconnectUrl({ ...settings, bind_address: '::1' }, 'http://127.0.0.1:8080/settings#network'))
      .toBe('http://[::1]:9090/settings#network')
  })
})
