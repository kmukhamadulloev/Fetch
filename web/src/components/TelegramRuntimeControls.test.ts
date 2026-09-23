import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import TelegramRuntimeControls from './TelegramRuntimeControls.vue'
import { i18n, setLocale } from '@/i18n'
import { useTelegramStore } from '@/stores/telegram'

beforeEach(() => {
  setActivePinia(createPinia())
  setLocale('en')
  vi.useFakeTimers()
  vi.setSystemTime(new Date('2026-09-23T12:00:00Z'))
  useTelegramStore().value = {
    settings: { enabled: true, use_proxy: true, send_completed_media: false, upload_limit_mb: 50, allowed_user_ids: [123], notify_queued: true, notify_completed: true, notify_failed: true, privacy_acknowledged: true },
    status: { state: 'backing_off', failed_attempts: 1, retry_at: '2026-09-23T12:00:10Z', token_configured: true, token_source: 'native', bot_username: null, last_success_at: null, error: 'Telegram is unreachable' },
  }
})
afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks() })

it('counts down live status, shows exhaustion, and sends a restart without changing settings', async () => {
  const telegram = useTelegramStore()
  const fetch = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(JSON.stringify({
    ...telegram.value, status: { ...telegram.value!.status, state: 'connecting', failed_attempts: 0, retry_at: null, error: null },
  }), { status: 200, headers: { 'Content-Type': 'application/json' } }))
  const wrapper = mount(TelegramRuntimeControls, { global: { plugins: [i18n] } })
  expect(wrapper.text()).toContain('Retrying in 10s')
  await vi.advanceTimersByTimeAsync(3000)
  expect(wrapper.text()).toContain('Retrying in 7s')
  telegram.applyStatus({ ...telegram.value!.status, state: 'error', failed_attempts: 3, retry_at: null })
  await flushPromises()
  expect(wrapper.text()).toContain('Telegram couldn’t connect after 3 attempts.')
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(fetch).toHaveBeenCalledWith('/api/telegram/restart', expect.objectContaining({ method: 'POST' }))
  expect(wrapper.text()).toContain('Connecting')
  expect(wrapper.text()).not.toContain('unreachable')
  wrapper.unmount()
  expect(vi.getTimerCount()).toBe(0)
})

it('disables restart while busy, disabled, or missing a token and translates messages', async () => {
  const telegram = useTelegramStore()
  const wrapper = mount(TelegramRuntimeControls, { global: { plugins: [i18n] } })
  for (const reason of ['busy', 'disabled', 'missing']) {
    telegram.saving = reason === 'busy'
    telegram.value!.settings.enabled = reason !== 'disabled'
    telegram.value!.status.token_configured = reason !== 'missing'
    await flushPromises()
    expect(wrapper.get('button').attributes('disabled')).toBeDefined()
  }
  for (const locale of ['ru', 'tg'] as const) {
    setLocale(locale)
    await flushPromises()
    expect(wrapper.text()).not.toContain('Retrying in')
    expect(wrapper.text()).not.toContain('settings.')
  }
  wrapper.unmount()
})
