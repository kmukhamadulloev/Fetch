import { expect, test, type Page } from '@playwright/test'
import type { CompletedFile, ProxySettings, TelegramIntegration } from '../src/app/api/client'

const readyRuntime = [
  { name: 'yt-dlp', version: '2026.08.09', status: 'ready', progress_percent: null, error: null },
  { name: 'ffmpeg', version: '6.1', status: 'ready', progress_percent: null, error: null },
  { name: 'ffprobe', version: '6.1', status: 'ready', progress_percent: null, error: null },
]
const settings = {
  bind_address: '127.0.0.1', port: 8080, allowed_networks: ['192.168.0.0/16'],
  download_directory: 'downloads', concurrent_downloads: 3,
  open_browser_on_start: false, start_with_system: false, ytdlp_auto_update: true,
  ytdlp_js_runtime: 'auto',
}
const completedFixture: CompletedFile[] = [{ id: 'af2bf705-8425-4178-9c5c-805e62db4f64', job_id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310', playlist: null, filename: 'media.mp4', thumbnail_available: true, size_bytes: 100, mime_type: 'video/mp4', title: 'Fixture media', browser_playable: true, playback: null, created_at: '2026-08-09T00:00:00Z' }]
const playlistCompletedFixture: CompletedFile[] = [
  { ...completedFixture[0], id: '10000000-0000-4000-8000-000000000002', job_id: '20000000-0000-4000-8000-000000000002', playlist: { id: 'fixture-list', title: 'Fixture playlist', index: 2 }, filename: '002-second.mp4', title: 'Second item', size_bytes: 60, created_at: '2026-08-10T00:00:00Z' },
  { ...completedFixture[0], id: '10000000-0000-4000-8000-000000000001', job_id: '20000000-0000-4000-8000-000000000001', playlist: { id: 'fixture-list', title: 'Fixture playlist', index: 1 }, filename: '001-first.mp4', title: 'First item', size_bytes: 40, created_at: '2026-08-09T00:00:00Z' },
]

async function mockApi(
  page: Page,
  runtime = readyRuntime,
  network = { urls: ['http://127.0.0.1:8080'], local_client: false },
) {
  let jobs: unknown[] = []
  let completed = [...completedFixture]
  let proxy: ProxySettings = { mode: 'system', url: null }
  let telegram: TelegramIntegration = {
    settings: { enabled: false, use_proxy: false, send_completed_media: false, upload_limit_mb: 50, allowed_user_ids: [], notify_queued: true, notify_completed: true, notify_failed: true, privacy_acknowledged: false },
    status: { state: 'disabled', token_configured: false, token_source: 'missing', bot_username: null, last_success_at: null, error: null },
  }
  await page.route('**/*', async (route) => {
    const request = route.request()
    const path = new URL(request.url()).pathname
    if (!path.startsWith('/api/')) return route.fallback()
    if (path === '/api/events') return route.fulfill({ status: 204 })
    if (path === '/api/status') return route.fulfill({ json: { version: '0.1.0', server: 'ready', runtime_ready: runtime.every((item) => item.status === 'ready'), storage_ready: true } })
    if (path === '/api/runtime/javascript') return route.fulfill({ json: [
      { name: 'deno', detected: false, version: null },
      { name: 'node', detected: true, version: 'v25.9.0' },
      { name: 'quickjs', detected: false, version: null },
    ] })
    if (path === '/api/runtime' && request.method() === 'GET') return route.fulfill({ json: runtime })
    if (/^\/api\/runtime\/.+\/(install|update|repair)$/.test(path)) return route.fulfill({ status: 202, json: { accepted: true } })
    if (path === '/api/downloads' && request.method() === 'GET') return route.fulfill({ json: jobs })
    if (path === '/api/downloads' && request.method() === 'POST') {
      const body = request.postDataJSON()
      const job = { ...body, id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310', status: 'downloading', progress_percent: 25, downloaded_bytes: 25, total_bytes: 100, speed_bytes_per_second: 10, eta_seconds: 5, error_code: null, error_message: null, created_at: '2026-08-09T00:00:00Z', updated_at: '2026-08-09T00:00:00Z' }
      jobs = [job]
      return route.fulfill({ status: 201, json: job })
    }
    if (path === '/api/media/analyze') return route.fulfill({ json: { kind: 'media', id: 'fixture', extractor: 'Generic', title: 'Fixture media', webpage_url: 'https://example.test/media', duration_seconds: 12, thumbnail_url: null, playlist_count: null, entries: [], formats: [{ id: '720', label: '720p', extension: 'mp4', video_codec: 'avc1', audio_codec: 'aac', width: 1280, height: 720, fps: 30, bitrate_kbps: 1000, filesize_bytes: 100, has_video: true, has_audio: true }] } })
    if (/^\/api\/files\/.+\/thumbnail$/.test(path)) return route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="9"><rect width="16" height="9" fill="#0f766e"/></svg>' })
    if (path === '/api/completed') return route.fulfill({ json: completed })
    if (/^\/api\/files\/.+\/progress$/.test(path) && request.method() === 'PUT') {
      const id = path.split('/').at(-2); const body = request.postDataJSON(); const progress = { file_id: id, position_seconds: body.position_seconds, duration_seconds: body.duration_seconds, completed: body.position_seconds >= body.duration_seconds * .95, updated_at: '2026-08-10T00:00:00Z' }
      completed = completed.map((file) => file.id === id ? { ...file, playback: progress } : file)
      return route.fulfill({ json: progress })
    }
    if (/^\/api\/files\/.+\/progress$/.test(path) && request.method() === 'DELETE') { const id = path.split('/').at(-2); completed = completed.map((file) => file.id === id ? { ...file, playback: null } : file); return route.fulfill({ status: 204 }) }
    if (/^\/api\/files\/.+$/.test(path) && request.method() === 'DELETE') { const id = path.split('/').at(-1); completed = completed.filter((file) => file.id !== id); return route.fulfill({ status: 204 }) }
    if (/^\/api\/files\/.+\/reveal$/.test(path)) return route.fulfill({ status: network.local_client ? 204 : 403, json: network.local_client ? undefined : { error: { code: 'LOCAL_CLIENT_REQUIRED', message: 'Host only' } } })
    if (path === '/api/history') return route.fulfill({ json: [] })
    if (path === '/api/logs') return route.fulfill({ json: [
      { id: 3, level: 'error', subsystem: 'runtime', message: 'runtime operation failed', details: 'component: yt-dlp\naction: automatic update\nkind: connection', created_at: '2026-08-12T19:18:08Z' },
      { id: 2, level: 'warn', subsystem: 'yt-dlp', message: 'extractor warning', details: 'job: fixture', created_at: '2026-08-12T19:17:08Z' },
      { id: 1, level: 'info', subsystem: 'runtime', message: 'runtime operation started', details: 'component: yt-dlp', created_at: '2026-08-12T19:16:08Z' },
    ] })
    if (path === '/api/settings') return route.fulfill({ json: request.method() === 'PUT' ? { ...request.postDataJSON(), listener_changed: false } : settings })
    if (path === '/api/network') return route.fulfill({ json: { bind_address: settings.bind_address, port: settings.port, urls: network.urls, authentication: false, restart_required_after_bind_change: false, local_client: network.local_client } })
    if (path === '/api/proxy') {
      if (!network.local_client) return route.fulfill({ status: 403, json: { error: { code: 'LOCAL_CLIENT_REQUIRED', message: 'Host only' } } })
      if (request.method() === 'PUT') {
        const next = request.postDataJSON() as ProxySettings
        if (next.url?.includes('@')) return route.fulfill({ status: 400, json: { error: { code: 'INVALID_SETTINGS', message: 'authenticated proxies are not supported in this version' } } })
        proxy = next
      }
      return route.fulfill({ json: proxy })
    }
    if (path.startsWith('/api/telegram')) {
      if (!network.local_client) return route.fulfill({ status: 403, json: { error: { code: 'LOCAL_CLIENT_REQUIRED', message: 'Host only' } } })
      if (path === '/api/telegram/token' && request.method() === 'PUT') {
        telegram = { ...telegram, status: { ...telegram.status, token_configured: true, token_source: 'native' } }
      } else if (path === '/api/telegram/token' && request.method() === 'DELETE') {
        telegram = { ...telegram, status: { ...telegram.status, token_configured: false, token_source: 'missing' } }
      } else if (path === '/api/telegram/settings' && request.method() === 'PUT') {
        telegram = { ...telegram, settings: request.postDataJSON(), status: { ...telegram.status, state: request.postDataJSON().enabled ? 'connecting' : 'disabled' } }
      } else if (path === '/api/telegram/test' && request.method() === 'POST') {
        telegram = { ...telegram, status: { ...telegram.status, bot_username: 'fetch_fixture_bot', last_success_at: '2026-08-20T00:00:00Z' } }
      }
      return route.fulfill({ json: telegram })
    }
    if (path === '/api/diagnostics') return route.fulfill({ json: { checks: [] } })
    return route.fulfill({ status: 404, json: { error: { code: 'NOT_FOUND', message: 'Not found' } } })
  })
  return { setCompleted: (files: CompletedFile[]) => { completed = files } }
}

test('first run offers real managed runtime installation', async ({ page }) => {
  await mockApi(page, [{ name: 'yt-dlp', version: null, status: 'missing', progress_percent: null, error: null }, ...readyRuntime.slice(1)])
  await page.goto('/')
  await expect(page.locator('.logo-mark img').first()).toHaveAttribute('src', '/logo.png')
  await expect.poll(() => page.locator('.logo-mark img').first().evaluate((image: HTMLImageElement) => image.naturalWidth)).toBeGreaterThan(0)
  await expect(page.getByRole('dialog', { name: 'Set up Fetch runtime' })).toBeVisible()
  const request = page.waitForRequest((request) => request.url().includes('/api/runtime/yt-dlp/install'))
  await page.getByRole('button', { name: 'Install yt-dlp' }).click()
  expect((await request).method()).toBe('POST')
})

test('analyze, configure, queue, and save settings', async ({ page }) => {
  await mockApi(page)
  await page.goto('/')
  await page.getByPlaceholder('https://example.com/video-or-playlist').fill('https://example.test/media')
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByRole('heading', { name: 'Fixture media' })).toBeVisible()
  await page.getByLabel('Maximum quality').selectOption('720p')
  await page.getByLabel('Video codec').selectOption('h264')
  await page.getByRole('button', { name: 'Add to downloads' }).click()
  await expect(page).toHaveURL(/\/downloads$/)
  await expect(page.getByText('Fixture media')).toBeVisible()
  await expect(page.getByText('25.0%')).toBeVisible()
  await expect(page.getByText('0:12')).toBeVisible()
  await expect(page.getByText('Receiving media with yt-dlp')).toBeVisible()

  await page.goto('/completed')
  await expect(page.locator('.media-card img')).toHaveAttribute('src', /\/api\/files\/.+\/thumbnail/)
  await page.getByRole('button', { name: 'Watch', exact: true }).click()
  await expect(page.getByRole('dialog', { name: /Fixture media/ })).toBeVisible()
  await expect(page.locator('.player-dialog').getByRole('link', { name: 'Open file', exact: true })).toHaveAttribute('href', /\/api\/files\/.+\/stream/)
  await expect(page.locator('.player-dialog').getByRole('link', { name: 'Download', exact: true })).toHaveAttribute('href', /\/api\/files\/.+\/download/)
  await page.getByRole('button', { name: 'Close player' }).click()

  await page.goto('/settings#network')
  await page.getByRole('button', { name: 'Network' }).click()
  await expect(page.getByText('Network access has no application login')).toBeVisible()
  await page.getByRole('button', { name: 'Save settings' }).click()
  await expect(page.getByText('Saved', { exact: true })).toBeVisible()
})

test('host settings can enable background system startup', async ({ page }) => {
  await mockApi(page, readyRuntime, { urls: ['http://127.0.0.1:8080'], local_client: true })
  await page.goto('/settings#general')
  await page.getByLabel('Start Fetch with system').check()
  const startupSave = page.waitForRequest((request) => new URL(request.url()).pathname === '/api/settings' && request.method() === 'PUT')
  await page.getByRole('button', { name: 'Save settings' }).click()
  expect((await startupSave).postDataJSON().start_with_system).toBe(true)
  await expect(page.getByText('Saved', { exact: true })).toBeVisible()
})

test('JavaScript runtime controls detect Node and hot-apply the selection', async ({ page }) => {
  await mockApi(page, readyRuntime, { urls: ['http://127.0.0.1:8080'], local_client: true })
  await page.goto('/settings#runtime')
  await expect(page.getByText('Node').last()).toBeVisible()
  await expect(page.getByText('v25.9.0')).toBeVisible()
  await page.getByLabel('JavaScript runtime', { exact: true }).selectOption('node')
  const save = page.waitForRequest((request) => new URL(request.url()).pathname === '/api/settings' && request.method() === 'PUT')
  await page.getByRole('button', { name: 'Save settings' }).click()
  expect((await save).postDataJSON().ytdlp_js_runtime).toBe('node')
  await expect(page.getByText('Saved', { exact: true })).toBeVisible()
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    await page.evaluate(() => document.documentElement.clientWidth),
  )
})

test('remote settings cannot change host system startup', async ({ page }) => {
  await mockApi(page)
  await page.goto('/settings#general')
  await expect(page.getByLabel('Start Fetch with system')).toBeDisabled()
  await expect(page.getByText('Change this setting on the device running Fetch.')).toBeVisible()
})

test('host can hot-apply the shared outbound proxy', async ({ page }) => {
  await mockApi(page, readyRuntime, { urls: ['http://127.0.0.1:8080'], local_client: true })
  await page.goto('/settings#network')
  await page.getByLabel('Outbound proxy mode').selectOption('custom')
  await page.getByLabel('Proxy URL').fill('socks5://127.0.0.1:1080')
  const save = page.waitForRequest((request) => new URL(request.url()).pathname === '/api/proxy' && request.method() === 'PUT')
  await page.getByRole('button', { name: 'Save proxy' }).click()
  expect((await save).postDataJSON()).toEqual({ mode: 'custom', url: 'socks5://127.0.0.1:1080' })
  await expect(page.getByText('Proxy saved')).toBeVisible()
  await expect(page.getByText('opted-in Telegram reconnect using the saved mode')).toBeVisible()
})

test('LAN clients cannot read or change the host proxy endpoint', async ({ page }) => {
  let proxyRequests = 0
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/proxy') proxyRequests += 1
  })
  await mockApi(page, readyRuntime, { urls: ['http://192.168.1.25:8080'], local_client: false })
  await page.goto('/settings#network')
  await expect(page.getByText('Proxy configuration is private to the host.')).toBeVisible()
  await expect(page.getByLabel('Outbound proxy mode')).toHaveCount(0)
  expect(proxyRequests).toBe(0)
})

test('host can configure Telegram without the token appearing in responses or the input', async ({ page }) => {
  const renderErrors: string[] = []
  page.on('console', (message) => {
    if (message.type() === 'error' && /message compilation error|syntaxerror/i.test(message.text())) renderErrors.push(message.text())
  })
  page.on('pageerror', (error) => renderErrors.push(error.message))
  await mockApi(page, readyRuntime, { urls: ['http://127.0.0.1:8080'], local_client: true })
  await page.goto('/settings#general')
  const integrationsTab = page.getByRole('button', { name: 'Integrations' })
  await expect(integrationsTab).toBeInViewport()
  await integrationsTab.click()
  await expect(page).toHaveURL(/#integrations$/)
  await expect(page.getByRole('heading', { name: 'Telegram bot' })).toBeVisible()
  const tokenInput = page.getByLabel('New bot token')
  await expect(tokenInput).toBeDisabled()
  await expect(page.getByRole('status').getByText('Disabled', { exact: true })).toBeVisible()
  await page.getByLabel('Enable Telegram bot').check()
  await expect(page.getByText('Action needed')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Save Telegram settings' })).toBeDisabled()
  await tokenInput.fill('123456:fixture-secret')
  await page.getByRole('button', { name: 'Show bot token' }).click()
  await expect(tokenInput).toHaveAttribute('type', 'text')
  await page.getByRole('button', { name: 'Hide bot token' }).click()
  await expect(tokenInput).toHaveAttribute('type', 'password')
  const tokenSave = page.waitForRequest((request) => new URL(request.url()).pathname === '/api/telegram/token' && request.method() === 'PUT')
  await page.getByRole('button', { name: 'Save token' }).click()
  expect((await tokenSave).postDataJSON()).toEqual({ token: '123456:fixture-secret' })
  await expect(tokenInput).toHaveValue('')
  await expect(page.getByText('Configured via native')).toBeVisible()

  await page.getByLabel('Allowed Telegram user IDs, one per line').fill('123456789')
  await page.getByLabel('Use Fetch proxy').check()
  await page.getByLabel('Send completed media').check()
  await expect(page.getByLabel('Maximum attachment size')).toHaveValue('50')
  await page.getByLabel('Maximum attachment size').fill('0')
  await expect(page.getByText('Set the media upload limit between 1 and 50 MB.')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Save Telegram settings' })).toBeDisabled()
  await page.getByLabel('Maximum attachment size').fill('25')
  await page.getByText('I understand that submitted URLs').click()
  await expect(page.getByText('Ready to save')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Save Telegram settings' })).toBeEnabled()
  const settingsSave = page.waitForRequest((request) => new URL(request.url()).pathname === '/api/telegram/settings' && request.method() === 'PUT')
  await page.getByRole('button', { name: 'Save Telegram settings' }).click()
  expect((await settingsSave).postDataJSON()).toMatchObject({ enabled: true, use_proxy: true, send_completed_media: true, upload_limit_mb: 25, allowed_user_ids: [123456789] })
  await expect(page.getByText('Telegram updated')).toBeVisible()
  expect(renderErrors).toEqual([])
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    await page.evaluate(() => document.documentElement.clientWidth),
  )
})

test('LAN clients cannot request Telegram configuration', async ({ page }) => {
  let telegramRequests = 0
  page.on('request', (request) => {
    if (new URL(request.url()).pathname.startsWith('/api/telegram')) telegramRequests += 1
  })
  await mockApi(page, readyRuntime, { urls: ['http://192.168.1.25:8080'], local_client: false })
  await page.goto('/settings#integrations')
  await expect(page.getByText('Telegram configuration is private to the host.')).toBeVisible()
  await expect(page.getByPlaceholder('Paste a BotFather token')).toHaveCount(0)
  expect(telegramRequests).toBe(0)
})

test('logs expose detailed severity filters on desktop and mobile', async ({ page }, testInfo) => {
  await mockApi(page)
  await page.goto('/logs')

  await expect(page.getByRole('button', { name: 'All 3' })).toHaveAttribute('aria-pressed', 'true')
  await expect(page.locator('[data-log-entry]')).toHaveCount(3)
  if (testInfo.project.name === 'desktop-chromium') {
    const alignment = await page.locator('[data-log-entry]').evaluateAll((entries) => entries.map((entry) => {
      const elements = ['time', '.log-level', '.log-subsystem', '.log-message', '.log-details-button']
        .map((selector) => entry.querySelector(selector)?.getBoundingClientRect())
        .filter((box): box is DOMRect => Boolean(box))
      const centers = elements.map((box) => box.top + box.height / 2)
      return {
        verticalSpread: Math.max(...centers) - Math.min(...centers),
        levelLeft: entry.querySelector('.log-level')?.getBoundingClientRect().left,
        subsystemLeft: entry.querySelector('.log-subsystem')?.getBoundingClientRect().left,
      }
    }))
    expect(alignment.every(({ verticalSpread }) => verticalSpread < 1)).toBe(true)
    expect(new Set(alignment.map(({ levelLeft }) => levelLeft)).size).toBe(1)
    expect(new Set(alignment.map(({ subsystemLeft }) => subsystemLeft)).size).toBe(1)
  }
  await page.getByRole('button', { name: 'Errors 1' }).click()
  await expect(page.locator('[data-log-entry]')).toHaveCount(1)
  await expect(page.getByText('runtime operation failed')).toBeVisible()
  await expect(page.getByText('kind: connection')).toHaveCount(0)
  await page.getByRole('button', { name: 'Details' }).click()
  await expect(page.getByText('kind: connection')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Details' })).toHaveAttribute('aria-expanded', 'true')
  await page.getByRole('button', { name: 'Details' }).click()
  await expect(page.getByText('kind: connection')).toHaveCount(0)
  await expect(page.getByText('extractor warning')).toHaveCount(0)
  await page.getByRole('button', { name: 'Warnings 1' }).click()
  await expect(page.locator('[data-log-entry]')).toHaveCount(1)
  await expect(page.getByText('extractor warning')).toBeVisible()
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    await page.evaluate(() => document.documentElement.clientWidth),
  )
})

test('playlist entries retain shared folder context and item order', async ({ page }) => {
  await mockApi(page)
  await page.route('**/api/media/analyze', (route) => route.fulfill({ json: {
    kind: 'playlist', id: 'fixture-list', extractor: 'Generic', title: 'Fixture playlist', webpage_url: 'https://example.test/playlist', duration_seconds: null, thumbnail_url: null, playlist_count: 2, formats: [],
    entries: [
      { id: 'one', title: 'First item', url: 'https://example.test/one', duration_seconds: 10, thumbnail_url: null },
      { id: 'two', title: 'Second item', url: 'https://example.test/two', duration_seconds: 20, thumbnail_url: null },
    ],
  } }))
  const requests: Record<string, unknown>[] = []
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/downloads' && request.method() === 'POST') requests.push(request.postDataJSON())
  })
  await page.goto('/')
  await page.getByPlaceholder('https://example.com/video-or-playlist').fill('https://example.test/playlist')
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByRole('heading', { name: 'Fixture playlist' })).toBeVisible()
  await expect(page.getByText('Saved in an ordered folder')).toBeVisible()
  await page.getByRole('button', { name: 'Add 2 items' }).click()
  await expect(page).toHaveURL(/\/downloads$/)
  expect(requests).toHaveLength(2)
  expect(requests.map((request) => request.playlist)).toEqual([
    { id: 'fixture-list', title: 'Fixture playlist', index: 1 },
    { id: 'fixture-list', title: 'Fixture playlist', index: 2 },
  ])
})

test('completed playlists open from stacked cards into ordered media galleries', async ({ page }) => {
  const api = await mockApi(page)
  api.setCompleted([...playlistCompletedFixture, ...completedFixture])
  await page.goto('/completed')

  const playlist = page.locator('[data-playlist-id="fixture-list"]')
  await expect(page.locator('.completed-grid .media-card')).toHaveCount(2)
  await expect(playlist).toContainText('2 items')
  await expect(playlist).toContainText('100 B')
  await expect(playlist).toHaveCSS('position', 'relative')
  expect(await playlist.evaluate((element) => [
    getComputedStyle(element, '::before').transform !== 'none',
    getComputedStyle(element, '::after').transform !== 'none',
  ])).toEqual([true, true])

  await playlist.getByRole('button', { name: 'Open playlist Fixture playlist' }).click()
  await expect(page).toHaveURL(/\/completed\?playlist=fixture-list$/)
  const gallery = page.locator('[data-playlist-gallery]')
  await expect(gallery.locator('.media-card')).toHaveCount(2)
  await expect(gallery.locator('.media-card').nth(0)).toContainText('First item')
  await expect(gallery.locator('.media-card').nth(1)).toContainText('Second item')

  await page.getByRole('button', { name: 'Back to completed' }).click()
  await expect(page).toHaveURL(/\/completed$/)
  await expect(page.getByText('Fixture media')).toBeVisible()
})

test('completed cards stay inside a narrow mobile viewport', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 720 })
  const api = await mockApi(page)
  api.setCompleted([...playlistCompletedFixture, ...completedFixture])
  await page.goto('/completed')

  const assertContained = async (selector: string) => {
    expect(await page.locator(selector).evaluateAll((elements) => elements.every((element) => {
      const card = element.getBoundingClientRect()
      const viewportWidth = document.documentElement.clientWidth
      return card.left >= 0 && card.right <= viewportWidth && element.scrollWidth <= element.clientWidth
    }))).toBe(true)
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320)
  }

  await assertContained('.completed-grid .media-card')
  await assertContained('[data-playlist-id="fixture-list"] .media-card')
  await page.getByRole('button', { name: 'Open playlist Fixture playlist' }).click()
  await assertContained('[data-playlist-gallery] .media-card')
})

test('completed cards show saved watch progress and allow starting over', async ({ page }) => {
  const api = await mockApi(page)
  api.setCompleted([{ ...completedFixture[0], playback: { file_id: completedFixture[0].id, position_seconds: 30, duration_seconds: 100, completed: false, updated_at: '2026-08-10T00:00:00Z' } }])
  await page.goto('/completed')

  const card = page.locator('.media-card')
  await expect(card.locator('.watch-progress > span')).toHaveCSS('width', /.+/)
  await expect(card.getByRole('button', { name: 'Resume' })).toBeVisible()
  await card.getByRole('button', { name: 'Resume' }).click()
  const reset = page.waitForRequest((request) => request.url().endsWith('/progress') && request.method() === 'DELETE')
  await page.getByRole('button', { name: 'Start over' }).click()
  await reset
  await page.getByRole('button', { name: 'Close player' }).click()
  await expect(card.locator('.watch-progress')).toHaveCount(0)
})

test('an open Completed page updates when a download finishes', async ({ page }) => {
  await page.addInitScript(() => {
    const sources: EventTarget[] = []
    class BrowserEventSource extends EventTarget {
      onopen: ((event: Event) => void) | null = null
      onerror: ((event: Event) => void) | null = null
      constructor(public readonly url: string) {
        super()
        sources.push(this)
        setTimeout(() => this.onopen?.(new Event('open')), 0)
      }
      close() {}
    }
    Object.defineProperty(window, 'EventSource', { configurable: true, value: BrowserEventSource })
    Object.defineProperty(window, '__emitFetchEvent', { configurable: true, value: (name: string, data: unknown) => {
      sources.at(-1)?.dispatchEvent(new MessageEvent(name, { data: JSON.stringify(data) }))
    } })
  })
  const api = await mockApi(page)
  api.setCompleted([])
  await page.goto('/completed')
  await expect(page.getByText('No completed files')).toBeVisible()

  api.setCompleted([...completedFixture])
  await page.evaluate((file) => {
    const emit = (window as typeof window & { __emitFetchEvent: (name: string, data: unknown) => void }).__emitFetchEvent
    emit('download.completed', { id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310', status: 'completed' })
    emit('library.completed', file)
  }, completedFixture[0])
  await expect(page.getByText('Fixture media')).toBeVisible()
  await expect(page.getByText('No completed files')).toHaveCount(0)
})

test('theme preference persists and mobile navigation exposes the primary flow', async ({ page }, testInfo) => {
  await mockApi(page)
  await page.goto('/settings#general')
  await page.getByLabel('Theme').selectOption('light')
  await expect(page.getByLabel('Theme')).toHaveCSS('appearance', 'none')
  await expect(page.getByLabel('Theme')).toHaveCSS('padding-right', '36px')
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await page.reload()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await page.keyboard.press('Tab')
  await expect(page.locator(':focus-visible')).toHaveCSS('outline-color', 'rgb(8, 127, 114)')
  await expect(page.locator(':focus-visible')).toHaveCSS('outline-style', 'solid')

  if (testInfo.project.name === 'mobile-chromium') {
    await expect(page.locator('.topbar h1')).toBeHidden()
    await expect(page.locator('.topbar .toolbar-btn')).toContainText(/Live|Shared live|Connecting|Reconnecting|Starting|Offline/)
    await expect(page.locator('.mobile-bar')).toBeVisible()
    await expect(page.locator('.mobile-bar .mobile-nav')).toHaveCount(4)
    await page.getByRole('button', { name: 'Advanced' }).click()
    await expect(page.getByRole('link', { name: /Logs and diagnostics/ })).toBeVisible()
    await page.locator('.mobile-bar').getByRole('link', { name: 'Files', exact: true }).click()
    await expect(page).toHaveURL(/\/completed$/)
  }
})

test('language switches immediately, persists, and remains responsive', async ({ page }) => {
  await mockApi(page)
  await page.goto('/settings#general')

  await page.getByLabel('Language').selectOption('ru')
  await expect(page.getByRole('heading', { level: 2, name: 'Настройки', exact: true })).toBeVisible()
  await expect(page.locator('html')).toHaveAttribute('lang', 'ru')
  await page.reload()
  await expect(page.getByLabel('Язык')).toHaveValue('ru')
  await expect(page.getByRole('button', { name: 'Загрузки' })).toBeVisible()

  await page.getByLabel('Язык').selectOption('tg')
  await expect(page.getByRole('heading', { level: 2, name: 'Танзимот', exact: true })).toBeVisible()
  await expect(page.locator('html')).toHaveAttribute('lang', 'tg')
  await expect(page.getByRole('button', { name: 'Боргириҳо' })).toBeVisible()
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    await page.evaluate(() => document.documentElement.clientWidth),
  )
})

test('one primary tab owns realtime and another tab can take over', async ({ page, context }) => {
  await context.addInitScript(() => {
    class BrowserEventSource extends EventTarget {
      onopen: ((event: Event) => void) | null = null
      onerror: ((event: Event) => void) | null = null
      private closed = false

      constructor(public readonly url: string) {
        super()
        const active = Number(localStorage.getItem('fetch.test.active-sse') ?? '0') + 1
        localStorage.setItem('fetch.test.active-sse', String(active))
        setTimeout(() => this.onopen?.(new Event('open')), 0)
      }

      close() {
        if (this.closed) return
        this.closed = true
        const active = Math.max(0, Number(localStorage.getItem('fetch.test.active-sse') ?? '1') - 1)
        localStorage.setItem('fetch.test.active-sse', String(active))
      }
    }
    Object.defineProperty(window, 'EventSource', { configurable: true, value: BrowserEventSource })
  })

  await mockApi(page)
  await page.goto('/')
  await expect(page.getByRole('button', { name: 'Live' })).toBeVisible()

  const second = await context.newPage()
  await mockApi(second)
  await second.goto('/')
  await expect(second.getByText('Another Fetch tab is primary.')).toBeVisible()
  await expect.poll(() => second.evaluate(() => Number(localStorage.getItem('fetch.test.active-sse')))).toBe(1)

  await second.getByRole('button', { name: 'Make primary' }).click()
  await expect(second.getByRole('button', { name: 'Live' })).toBeVisible()
  await expect(page.getByText('Another Fetch tab is primary.')).toBeVisible()
  await expect.poll(() => second.evaluate(() => Number(localStorage.getItem('fetch.test.active-sse')))).toBe(1)

  await second.close()
  await expect(page.getByRole('button', { name: 'Live' })).toBeVisible({ timeout: 8_000 })
  await expect.poll(() => page.evaluate(() => Number(localStorage.getItem('fetch.test.active-sse')))).toBe(1)
})

test('LAN QR access renders a scannable selected address', async ({ page }) => {
  await mockApi(page, readyRuntime, { urls: ['http://127.0.0.1:8080', 'http://192.168.1.25:8080'], local_client: true })
  await page.goto('/')
  await page.getByRole('button', { name: 'Open Fetch on mobile' }).click()
  const dialog = page.getByRole('dialog', { name: 'Open Fetch on mobile' })
  await expect(dialog).toBeVisible()
  await expect(dialog.getByText('http://192.168.1.25:8080')).toBeVisible()
  await expect(dialog.locator('svg[shape-rendering="crispEdges"]')).toBeVisible()
})

test('host completed actions reveal folders and permanently delete media', async ({ page }) => {
  await mockApi(page, readyRuntime, { urls: ['http://127.0.0.1:8080'], local_client: true })
  await page.goto('/completed')
  await expect(page.getByRole('button', { name: 'Open folder' })).toBeVisible()
  await expect(page.locator('.media-card').getByRole('link', { name: 'Download', exact: true })).toHaveCount(0)

  const reveal = page.waitForRequest((request) => request.url().endsWith('/reveal') && request.method() === 'POST')
  await page.getByRole('button', { name: 'Open folder' }).click()
  await reveal

  await page.getByRole('button', { name: 'Delete Fixture media' }).click()
  await expect(page.getByRole('dialog', { name: 'Delete downloaded file?' })).toBeVisible()
  const deletion = page.waitForRequest((request) => /\/api\/files\/[a-f0-9-]+$/.test(request.url()) && request.method() === 'DELETE')
  await page.getByRole('button', { name: 'Delete permanently' }).click()
  await deletion
  await expect(page.getByText('No completed files')).toBeVisible()
})

test('completed filters and sorting preserve playlist navigation', async ({ page }) => {
  await mockApi(page)
  const audio = { ...completedFixture[0], id: 'audio-item', title: 'A song', mime_type: 'audio/mpeg', created_at: '2026-08-11T00:00:00Z' }
  await page.route('**/api/completed', (route) => route.fulfill({ json: [...completedFixture, ...playlistCompletedFixture, audio] }))
  await page.goto('/completed')
  const titles = page.locator('.completed-grid h3')
  await expect(titles).toHaveText(['A song', 'Fixture playlist', 'Fixture media'])
  await page.getByRole('button', { name: 'Sort by: Date' }).click()
  await page.getByRole('radio', { name: 'Name', exact: true }).check()
  await expect(titles).toHaveText(['A song', 'Fixture media', 'Fixture playlist'])
  await page.getByLabel('Reverse', { exact: true }).check()
  await expect(titles).toHaveText(['Fixture playlist', 'Fixture media', 'A song'])
  await page.getByRole('radio', { name: 'Date', exact: true }).check()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('button', { name: 'Sort by: Date' })).toHaveAttribute('aria-expanded', 'false')
  await expect(titles).toHaveText(['Fixture media', 'Fixture playlist', 'A song'])
  const filters = page.getByRole('group', { name: 'Media type filter' })
  await filters.getByRole('button', { name: 'Audio', exact: true }).click()
  await expect(titles).toHaveText(['A song'])
  await filters.getByRole('button', { name: 'Video', exact: true }).click()
  await expect(titles).toHaveCount(3)
  await expect(titles).not.toContainText(['A song'])
  await filters.getByRole('button', { name: 'Playlist', exact: true }).click()
  await expect(titles).toHaveText(['Fixture playlist'])
  await page.getByRole('button', { name: 'Open playlist Fixture playlist' }).click()
  await expect(page.locator('[data-playlist-gallery] h3')).toHaveText(['First item', 'Second item'])
  await page.getByRole('button', { name: 'Back to completed' }).click()
  await expect(filters.getByRole('button', { name: 'Playlist', exact: true })).toHaveAttribute('aria-pressed', 'true')
  await filters.getByRole('button', { name: 'All', exact: true }).click()
  await expect(titles).toHaveCount(3)
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
})

test('downloads status filters include queued jobs and reset empty results', async ({ page }) => {
  await mockApi(page)
  const statuses = ['created', 'analyzing', 'ready', 'queued', 'downloading', 'postprocessing', 'completed', 'failed', 'stopped']
  await page.route('**/api/downloads', (route) => route.fulfill({ json: statuses.map((status, index) => ({
    id: String(index), status, title: status, url: 'https://example.com/media', mode: 'video',
    total_bytes: null, downloaded_bytes: null, progress_percent: null, duration_seconds: null,
  })) }))
  await page.goto('/downloads')
  const cards = page.locator('article')
  await expect(cards).toHaveCount(9)
  const filters = page.getByRole('group', { name: 'Download status filter' })
  await filters.getByRole('button', { name: 'In Progress', exact: true }).click()
  await expect(cards).toHaveCount(6)
  await filters.getByRole('button', { name: 'Completed', exact: true }).click()
  await expect(cards.locator('h3')).toHaveText(['completed'])
  await filters.getByRole('button', { name: 'Error', exact: true }).click()
  await expect(cards.locator('h3')).toHaveText(['failed'])
  await filters.getByRole('button', { name: 'All', exact: true }).click()
  await expect(cards).toHaveCount(9)
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  await page.goto('/completed')
  await page.getByRole('group', { name: 'Media type filter' }).getByRole('button', { name: 'Audio', exact: true }).click()
  await expect(page.getByText('No matching items', { exact: true })).toBeVisible()
  await page.getByRole('group', { name: 'Media type filter' }).getByRole('button', { name: 'All', exact: true }).click()
  await expect(page.locator('.completed-grid h3')).toHaveText(['Fixture media'])
})

async function mockMetadata(page: Page, audio = false, editable = true, fail = false) {
  const fields: Record<string, string> = audio
    ? { title: 'Fixture media', artist: 'Artist', album: 'Album', album_artist: '', track: '1', track_total: '8', date: '', genre: '', copyright: '', comment: '', description: '', disc: '', disc_total: '', composer: '' }
    : { title: 'Fixture media', artist: 'Creator', description: 'Description', date: '', genre: '', copyright: '', comment: '' }
  let saved = false
  await page.route('**/api/files/*/metadata**', async (route) => {
    const path = new URL(route.request().url()).pathname
    if (path.endsWith('/status')) return route.fulfill({ json: { file_id: completedFixture[0].id, operation_id: saved ? 'operation-1' : null, state: saved ? (fail ? 'failed' : 'completed') : 'idle', error: fail ? 'The original file was preserved.' : null } })
    if (path.endsWith('/artwork')) return route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="32" height="32" fill="teal"/></svg>' })
    if (route.request().method() === 'PUT') {
      saved = true
      return route.fulfill({ status: 202, json: { file_id: completedFixture[0].id, operation_id: 'operation-1', state: 'saving', error: null } })
    }
    return route.fulfill({ json: { revision: 'revision-1', editable, media_type: audio ? 'audio' : 'video', container: audio ? 'mp3' : 'mp4', fields, supported_fields: editable ? Object.keys(fields) : [], artwork_available: true, artwork_editable: editable, information: { filename: 'media.mp4', duration: '12', size: '100', bit_rate: '1000' } } })
  })
}

test('metadata pencil opens modal, advanced fields and dirty guard without playback', async ({ page }) => {
  await mockApi(page)
  await mockMetadata(page)
  await page.goto('/completed')
  const pencil = page.getByRole('button', { name: 'Edit metadata for Fixture media' })
  await pencil.click()
  const dialog = page.getByRole('dialog', { name: 'Edit metadata', exact: true })
  await expect(dialog.getByLabel('Title', { exact: true })).toHaveValue('Fixture media')
  await expect(dialog.getByLabel('Creator', { exact: true })).toBeVisible()
  await expect(dialog.getByLabel('Description', { exact: true })).toBeVisible()
  await expect(dialog.getByLabel('Genre', { exact: true })).toHaveCount(0)
  await dialog.getByRole('button', { name: 'Advanced', exact: true }).click()
  await expect(dialog.getByLabel('Genre', { exact: true })).toBeVisible()
  await dialog.getByLabel('Title', { exact: true }).fill('Edited title')
  await page.keyboard.press('Escape')
  await expect(dialog.getByText('Discard your unsaved changes?')).toBeVisible()
  await dialog.getByRole('button', { name: 'Keep editing' }).click()
  await expect(dialog.getByLabel('Title', { exact: true })).toHaveValue('Edited title')
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click()
  await dialog.getByRole('button', { name: 'Discard changes', exact: true }).click()
  await expect(dialog).toHaveCount(0)
  await expect(pencil).toBeFocused()
  await expect(page.locator('video')).toHaveCount(0)
})

test('audio metadata saves changed tags and uploaded artwork with responsive footer', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await mockApi(page)
  await mockMetadata(page, true)
  await page.goto('/completed')
  await page.getByRole('button', { name: 'Edit metadata for Fixture media' }).click()
  const dialog = page.getByRole('dialog', { name: 'Edit metadata', exact: true })
  await expect(dialog.getByLabel('Album artist', { exact: true })).toBeVisible()
  await dialog.getByLabel('Title', { exact: true }).fill('New audio title')
  await dialog.getByLabel('Upload / replace', { exact: true }).setInputFiles({ name: 'cover.png', mimeType: 'image/png', buffer: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=', 'base64') })
  await dialog.getByRole('button', { name: 'Advanced', exact: true }).click()
  await dialog.getByLabel('Total tracks', { exact: true }).fill('10')
  await expect(dialog.getByRole('button', { name: 'Save changes' })).toBeInViewport()
  await page.screenshot({ path: test.info().outputPath('metadata-editor.png') })
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true)
  const request = page.waitForRequest((request) => request.method() === 'PUT' && request.url().endsWith('/metadata'))
  await dialog.getByRole('button', { name: 'Save changes' }).click()
  const body = (await request).postDataJSON()
  expect(body.fields).toEqual({ title: 'New audio title', track_total: '10' })
  expect(body.artwork.action).toBe('replace')
  expect(body.revision).toBe('revision-1')
  await expect(dialog).toHaveCount(0)
})

test('metadata removal failures preserve form and unsupported files remain read-only', async ({ page }) => {
  await mockApi(page)
  await mockMetadata(page, false, true, true)
  await page.goto('/completed')
  await page.getByRole('button', { name: 'Edit metadata for Fixture media' }).click()
  const dialog = page.getByRole('dialog', { name: 'Edit metadata', exact: true })
  await dialog.getByRole('button', { name: 'Remove', exact: true }).click()
  const request = page.waitForRequest((request) => request.method() === 'PUT' && request.url().endsWith('/metadata'))
  await dialog.getByRole('button', { name: 'Save changes' }).click()
  expect((await request).postDataJSON().artwork).toEqual({ action: 'remove' })
  await expect(dialog.getByRole('alert')).toContainText('The original file was preserved.')
  await expect(dialog.getByRole('button', { name: 'Save changes' })).toBeEnabled()
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click()
  await dialog.getByRole('button', { name: 'Discard changes', exact: true }).click()
  await mockMetadata(page, false, false)
  await page.getByRole('button', { name: 'Edit metadata for Fixture media' }).click()
  await expect(dialog.getByText('Editing this file format is not supported.', { exact: false })).toBeVisible()
  await expect(dialog.getByRole('button', { name: 'Save changes' })).toBeDisabled()
})


for (const [locale, edit, title, advanced, save] of [
  ['ru', 'Изменить метаданные', 'Название', 'Дополнительно', 'Сохранить изменения'],
  ['tg', 'Таҳрири метамаълумот', 'Ном', 'Иловагӣ', 'Сабти тағйирот'],
]) {
  test(`metadata modal is localized and contained in ${locale}`, async ({ page }) => {
    await page.addInitScript((locale) => localStorage.setItem('fetch.locale', locale), locale)
    await mockApi(page)
    await mockMetadata(page, true)
    await page.goto('/completed')
    await page.locator('.metadata-pencil').click()
    const dialog = page.getByRole('dialog', { name: edit, exact: true })
    await expect(dialog.getByLabel(title, { exact: true })).toHaveValue('Fixture media')
    await dialog.getByRole('button', { name: advanced, exact: true }).click()
    await expect(dialog.getByRole('button', { name: save, exact: true })).toBeInViewport()
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true)
    await expect(dialog).not.toContainText('metadata.')
  })
}
