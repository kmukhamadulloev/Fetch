import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, expect, it, vi } from 'vitest'
import RuntimeSummary from './RuntimeSummary.vue'
import { i18n, setLocale } from '@/i18n'
import { useSettingsStore } from '@/stores/settings'
import { useTelegramStore } from '@/stores/telegram'
import { useProxyStore } from '@/stores/proxy'
import { useRealtimeStore } from '@/stores/realtime'
import { useRuntimeStore } from '@/stores/runtime'

beforeEach(() => {
  setActivePinia(createPinia())
  setLocale('en')
  vi.spyOn(useTelegramStore(), 'refresh').mockResolvedValue()
  vi.spyOn(useProxyStore(), 'refresh').mockResolvedValue()
  useSettingsStore().network = { local_client: true, bind_address: '127.0.0.1', port: 8080, urls: [], authentication: false, restart_required_after_bind_change: false }
})

it('loads host integrations, follows live status, and never presents stale health as ready', async () => {
  const telegram = useTelegramStore()
  telegram.value = {
    settings: { enabled: true, use_proxy: false, send_completed_media: false, upload_limit_mb: 50, allowed_user_ids: [], notify_queued: true, notify_completed: true, notify_failed: true, privacy_acknowledged: true },
    status: { state: 'connecting', token_configured: true, token_source: 'native', bot_username: null, last_success_at: null, error: null },
  }
  useProxyStore().value = { mode: 'custom', url: 'http://private-proxy:8080' }
  useRealtimeStore().connection = 'connected'
  useRuntimeStore().components = ['yt-dlp', 'ffmpeg', 'ffprobe'].map((name) => ({ name, status: 'ready', version: '1', progress_percent: null, error: null })) as ReturnType<typeof useRuntimeStore>['components']
  const wrapper = mount(RuntimeSummary, { global: { plugins: [i18n] } })
  await flushPromises()
  expect(telegram.refresh).toHaveBeenCalledOnce()
  expect(useProxyStore().refresh).toHaveBeenCalledOnce()
  expect(wrapper.get('[data-summary-row="backend"]').text()).toContain('Connected')
  expect(wrapper.get('[data-summary-row="runtime"]').text()).toContain('Ready')
  expect(wrapper.get('[data-summary-row="telegram"]').text()).toContain('Connecting')
  expect(wrapper.get('[data-summary-row="proxy"]').text()).toContain('Configured')
  expect(wrapper.html()).not.toContain('private-proxy')
  telegram.applyStatus({ ...telegram.value!.status, state: 'connected' })
  await flushPromises()
  expect(wrapper.get('[data-summary-row="telegram"]').text()).toContain('Connected')
  useRealtimeStore().connection = 'reconnecting'
  await flushPromises()
  expect(wrapper.get('[data-summary-row="backend"]').text()).toContain('Reconnecting')
  expect(wrapper.get('[data-summary-row="telegram"]').text()).toContain('Unknown')
  expect(wrapper.get('[data-summary-row="runtime"]').text()).toContain('Unknown')
  useRealtimeStore().connection = 'offline'
  await flushPromises()
  expect(wrapper.get('[data-summary-row="backend"]').text()).toContain('Offline')
})

it('hides unused integrations and makes no host-only requests for LAN clients', async () => {
  useSettingsStore().network!.local_client = false
  const wrapper = mount(RuntimeSummary, { global: { plugins: [i18n] } })
  await flushPromises()
  expect(useTelegramStore().refresh).not.toHaveBeenCalled()
  expect(useProxyStore().refresh).not.toHaveBeenCalled()
  expect(wrapper.findAll('[data-summary-row]')).toHaveLength(2)
})
