import { expect, test, type Page } from '@playwright/test'

const readyRuntime = [
  { name: 'yt-dlp', version: '2026.08.09', status: 'ready', progress_percent: null, error: null },
  { name: 'ffmpeg', version: '6.1', status: 'ready', progress_percent: null, error: null },
  { name: 'ffprobe', version: '6.1', status: 'ready', progress_percent: null, error: null },
]
const settings = {
  bind_address: '127.0.0.1', port: 8080, allowed_networks: ['192.168.0.0/16'],
  download_directory: 'downloads', concurrent_downloads: 3,
  open_browser_on_start: false, ytdlp_auto_update: true,
}
const completedFixture = [{ id: 'af2bf705-8425-4178-9c5c-805e62db4f64', job_id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310', filename: 'media.mp4', thumbnail_available: true, size_bytes: 100, mime_type: 'video/mp4', title: 'Fixture media', browser_playable: true, created_at: '2026-08-09T00:00:00Z' }]

async function mockApi(
  page: Page,
  runtime = readyRuntime,
  network = { urls: ['http://127.0.0.1:8080'], local_client: false },
) {
  let jobs: unknown[] = []
  let completed = [...completedFixture]
  await page.route('**/*', async (route) => {
    const request = route.request()
    const path = new URL(request.url()).pathname
    if (!path.startsWith('/api/')) return route.fallback()
    if (path === '/api/events') return route.fulfill({ status: 204 })
    if (path === '/api/status') return route.fulfill({ json: { version: '0.1.0', server: 'ready', runtime_ready: runtime.every((item) => item.status === 'ready'), storage_ready: true } })
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
    if (/^\/api\/files\/.+$/.test(path) && request.method() === 'DELETE') { completed = []; return route.fulfill({ status: 204 }) }
    if (/^\/api\/files\/.+\/reveal$/.test(path)) return route.fulfill({ status: network.local_client ? 204 : 403, json: network.local_client ? undefined : { error: { code: 'LOCAL_CLIENT_REQUIRED', message: 'Host only' } } })
    if (path === '/api/history' || path === '/api/logs') return route.fulfill({ json: [] })
    if (path === '/api/settings') return route.fulfill({ json: request.method() === 'PUT' ? { ...request.postDataJSON(), listener_changed: false } : settings })
    if (path === '/api/network') return route.fulfill({ json: { bind_address: settings.bind_address, port: settings.port, urls: network.urls, authentication: false, restart_required_after_bind_change: false, local_client: network.local_client } })
    if (path === '/api/diagnostics') return route.fulfill({ json: { checks: [] } })
    return route.fulfill({ status: 404, json: { error: { code: 'NOT_FOUND', message: 'Not found' } } })
  })
  return { setCompleted: (files: typeof completedFixture) => { completed = files } }
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
  await expect(page.getByText('Saved')).toBeVisible()
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
