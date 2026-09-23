import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { useTelegramStore } from './telegram'
import type { TelegramIntegration } from '@/app/api/client'

beforeEach(() => setActivePinia(createPinia()))
afterEach(() => vi.unstubAllGlobals())

it('retains a status event that arrives before the first settings snapshot', async () => {
  let resolve!: (response: Response) => void
  vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>((done) => { resolve = done })))
  const telegram = useTelegramStore()
  const snapshot: TelegramIntegration = {
    settings: { enabled: true, use_proxy: false, send_completed_media: false, upload_limit_mb: 50, allowed_user_ids: [123], notify_queued: true, notify_completed: true, notify_failed: true, privacy_acknowledged: true },
    status: { state: 'connecting', failed_attempts: 0, retry_at: null, token_configured: true, token_source: 'native', bot_username: null, last_success_at: null, error: null },
  }
  const refresh = telegram.refresh()
  telegram.applyStatus({ ...snapshot.status, state: 'connected' })
  resolve(new Response(JSON.stringify(snapshot)))
  await refresh
  expect(telegram.value?.status.state).toBe('connected')
  expect(telegram.value?.settings.allowed_user_ids).toEqual([123])
})
