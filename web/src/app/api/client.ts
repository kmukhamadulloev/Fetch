import { i18n } from '@/i18n'

export interface AppStatus {
  version: string
  server: 'ready'
  runtime_ready: boolean
  storage_ready: boolean
}

export type RuntimeStatus = 'missing' | 'installing' | 'ready' | 'updating' | 'failed'
export interface RuntimeComponent {
  name: 'yt-dlp' | 'ffmpeg' | 'ffprobe'
  version: string | null
  status: RuntimeStatus
  progress_percent: number | null
  error: string | null
}
export type YtDlpJsRuntime = 'auto' | 'deno' | 'node' | 'quickjs' | 'disabled'
export interface JavaScriptRuntime {
  name: 'deno' | 'node' | 'quickjs'
  detected: boolean
  version: string | null
}

export interface MediaFormat {
  id: string
  label: string
  extension: string | null
  video_codec: string | null
  audio_codec: string | null
  width: number | null
  height: number | null
  fps: number | null
  bitrate_kbps: number | null
  filesize_bytes: number | null
  has_video: boolean
  has_audio: boolean
}

export interface PlaylistEntry {
  id: string | null
  title: string
  url: string | null
  duration_seconds: number | null
  thumbnail_url: string | null
}

export interface MediaInfo {
  kind: 'media' | 'playlist'
  id: string | null
  extractor: string | null
  title: string
  webpage_url: string | null
  duration_seconds: number | null
  thumbnail_url: string | null
  playlist_count: number | null
  entries: PlaylistEntry[]
  formats: MediaFormat[]
}

export type DownloadStatus = 'created' | 'analyzing' | 'ready' | 'queued' | 'downloading' | 'postprocessing' | 'completed' | 'failed' | 'stopped'
export interface PlaylistContext {
  id: string
  title: string
  index: number
}
export interface DownloadRequest {
  url: string
  title?: string | null
  duration_seconds?: number | null
  mode: 'video' | 'audio'
  format_id?: string | null
  quality?: string | null
  container?: string | null
  video_codec?: string | null
  audio_codec?: string | null
  embed_metadata: boolean
  embed_thumbnail: boolean
  subtitles: boolean
  playlist?: PlaylistContext | null
  output_directory?: string | null
}
export interface DownloadJob extends DownloadRequest {
  id: string
  status: DownloadStatus
  progress_percent: number | null
  downloaded_bytes: number | null
  total_bytes: number | null
  speed_bytes_per_second: number | null
  eta_seconds: number | null
  error_code: string | null
  error_message: string | null
  created_at: string
  updated_at: string
}
export interface CompletedFile {
  id: string
  job_id: string
  playlist: PlaylistContext | null
  filename: string
  thumbnail_available: boolean
  size_bytes: number
  mime_type: string
  title: string | null
  browser_playable: boolean
  playback: PlaybackProgress | null
  created_at: string
}
export interface PlaybackProgress {
  file_id: string
  position_seconds: number
  duration_seconds: number
  completed: boolean
  updated_at: string
}
export interface ApplicationSettings {
  bind_address: string
  port: number
  allowed_networks: string[]
  download_directory: string
  concurrent_downloads: number
  open_browser_on_start: boolean
  start_with_system: boolean
  ytdlp_auto_update: boolean
  ytdlp_js_runtime: YtDlpJsRuntime
}
export interface NetworkInfo {
  bind_address: string
  port: number
  urls: string[]
  authentication: false
  restart_required_after_bind_change: boolean
  local_client: boolean
}
export type ProxyMode = 'system' | 'direct' | 'custom'
export interface ProxySettings {
  mode: ProxyMode
  url: string | null
}
export interface TelegramSettings {
  enabled: boolean
  use_proxy: boolean
  send_completed_media: boolean
  upload_limit_mb: number
  allowed_user_ids: number[]
  notify_queued: boolean
  notify_completed: boolean
  notify_failed: boolean
  privacy_acknowledged: boolean
}
export interface TelegramStatus {
  failed_attempts: number
  retry_at: string | null
  state: 'disabled' | 'connecting' | 'connected' | 'backing_off' | 'error'
  token_configured: boolean
  token_source: 'native' | 'environment' | 'missing'
  bot_username: string | null
  last_success_at: string | null
  error: string | null
}
export interface TelegramIntegration {
  settings: TelegramSettings
  status: TelegramStatus
}
export type SettingsSaveResponse = ApplicationSettings & { listener_changed: boolean }
export interface DiagnosticLogEntry {
  id: number
  level: string
  subsystem: string
  message: string
  details: string | null
  created_at: string
}
export interface DiagnosticsReport {
  checks: { name: string; healthy: boolean; message: string }[]
}

export class ApiError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message)
  }
}

const requestTimeoutMs = 10_000

async function fetchApi(path: string, init?: RequestInit): Promise<Response> {
  const controller = new AbortController()
  const timeout = window.setTimeout(() => controller.abort(), requestTimeoutMs)
  const abortFromCaller = () => controller.abort()
  init?.signal?.addEventListener('abort', abortFromCaller, { once: true })
  try {
    return await fetch(path, { ...init, signal: controller.signal })
  } catch (cause) {
    if (controller.signal.aborted && !init?.signal?.aborted) {
      throw new ApiError(0, i18n.global.t('errors.serverTimeout'))
    }
    if (cause instanceof ApiError) throw cause
    throw new ApiError(0, i18n.global.t('errors.serverUnavailable'))
  } finally {
    window.clearTimeout(timeout)
    init?.signal?.removeEventListener('abort', abortFromCaller)
  }
}

export async function getStatus(signal?: AbortSignal): Promise<AppStatus> {
  const response = await fetchApi('/api/status', { headers: { Accept: 'application/json' }, signal })
  if (!response.ok) throw new ApiError(response.status, i18n.global.t('errors.statusRequest', { status: response.status }))
  return response.json() as Promise<AppStatus>
}

async function jsonRequest<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetchApi(path, {
    ...init,
    headers: { Accept: 'application/json', ...(init?.body ? { 'Content-Type': 'application/json' } : {}), ...init?.headers },
  })
  if (!response.ok) {
    const payload = await response.json().catch(() => null) as { error?: { message?: string } } | null
    throw new ApiError(response.status, payload?.error?.message ?? i18n.global.t('errors.requestFailed', { status: response.status }))
  }
  return response.json() as Promise<T>
}

export function getRuntime(): Promise<RuntimeComponent[]> {
  return jsonRequest('/api/runtime')
}

export function getJavaScriptRuntimes(): Promise<JavaScriptRuntime[]> {
  return jsonRequest('/api/runtime/javascript')
}

export function startRuntimeAction(component: RuntimeComponent['name'], action: 'install' | 'update' | 'repair'): Promise<{ accepted: boolean }> {
  return jsonRequest(`/api/runtime/${component}/${action}`, { method: 'POST' })
}

export function analyzeMedia(url: string): Promise<MediaInfo> {
  return jsonRequest('/api/media/analyze', { method: 'POST', body: JSON.stringify({ url }) })
}

export function getDownloads(): Promise<DownloadJob[]> {
  return jsonRequest('/api/downloads')
}

export function createDownload(request: DownloadRequest): Promise<DownloadJob> {
  return jsonRequest('/api/downloads', { method: 'POST', body: JSON.stringify(request) })
}

export function downloadAction(id: string, action: 'stop' | 'resume' | 'retry'): Promise<DownloadJob> {
  return jsonRequest(`/api/downloads/${id}/${action}`, { method: 'POST' })
}

export function getCompleted(): Promise<CompletedFile[]> {
  return jsonRequest('/api/completed')
}

export async function revealCompleted(id: string): Promise<void> {
  const response = await fetchApi(`/api/files/${id}/reveal`, { method: 'POST' })
  if (!response.ok) {
    const payload = await response.json().catch(() => null) as { error?: { message?: string } } | null
    throw new ApiError(response.status, payload?.error?.message ?? i18n.global.t('errors.openFolderStatus', { status: response.status }))
  }
}

export async function deleteCompleted(id: string): Promise<void> {
  const response = await fetchApi(`/api/files/${id}`, { method: 'DELETE' })
  if (!response.ok) {
    const payload = await response.json().catch(() => null) as { error?: { message?: string } } | null
    throw new ApiError(response.status, payload?.error?.message ?? i18n.global.t('errors.deleteFileStatus', { status: response.status }))
  }
}

export function savePlaybackProgress(id: string, positionSeconds: number, durationSeconds: number): Promise<PlaybackProgress> {
  return jsonRequest(`/api/files/${id}/progress`, {
    method: 'PUT',
    body: JSON.stringify({ position_seconds: positionSeconds, duration_seconds: durationSeconds }),
  })
}

export async function clearPlaybackProgress(id: string): Promise<void> {
  const response = await fetchApi(`/api/files/${id}/progress`, { method: 'DELETE' })
  if (!response.ok) throw new ApiError(response.status, i18n.global.t('errors.resetProgress', { status: response.status }))
}

export function getHistory(): Promise<DownloadJob[]> {
  return jsonRequest('/api/history')
}

export function getSettings(): Promise<ApplicationSettings> { return jsonRequest('/api/settings') }
export function putSettings(settings: ApplicationSettings): Promise<SettingsSaveResponse> {
  return jsonRequest('/api/settings', { method: 'PUT', body: JSON.stringify(settings) })
}
export function getNetworkInfo(): Promise<NetworkInfo> { return jsonRequest('/api/network') }
export function getProxySettings(): Promise<ProxySettings> { return jsonRequest('/api/proxy') }
export function putProxySettings(settings: ProxySettings): Promise<ProxySettings> {
  return jsonRequest('/api/proxy', { method: 'PUT', body: JSON.stringify(settings) })
}
export function getTelegramIntegration(): Promise<TelegramIntegration> { return jsonRequest('/api/telegram') }
export function putTelegramSettings(settings: TelegramSettings): Promise<TelegramIntegration> {
  return jsonRequest('/api/telegram/settings', { method: 'PUT', body: JSON.stringify(settings) })
}
export function putTelegramToken(token: string): Promise<TelegramIntegration> {
  return jsonRequest('/api/telegram/token', { method: 'PUT', body: JSON.stringify({ token }) })
}
export function deleteTelegramToken(): Promise<TelegramIntegration> {
  return jsonRequest('/api/telegram/token', { method: 'DELETE' })
}
export async function restartTelegram(): Promise<TelegramIntegration> {
  return jsonRequest('/api/telegram/restart', { method: 'POST' })
}

export function testTelegramConnection(): Promise<TelegramIntegration> {
  return jsonRequest('/api/telegram/test', { method: 'POST' })
}
export function getLogs(): Promise<DiagnosticLogEntry[]> { return jsonRequest('/api/logs') }
export function getDiagnostics(): Promise<DiagnosticsReport> { return jsonRequest('/api/diagnostics') }
export async function clearLogs(): Promise<void> {
  const response = await fetchApi('/api/logs', { method: 'DELETE' })
  if (!response.ok) throw new ApiError(response.status, i18n.global.t('errors.clearLogs', { status: response.status }))
}

export interface MediaMetadata {
  revision: string
  editable: boolean
  media_type: 'audio' | 'video'
  container: string
  fields: Record<string, string>
  supported_fields: string[]
  artwork_available: boolean
  artwork_editable: boolean
  information: Record<string, string>
}
export type ArtworkUpdate = { action: 'keep' } | { action: 'remove' } | { action: 'replace'; data: string }
export interface MetadataUpdate {
  revision: string
  fields: Record<string, string>
  artwork: ArtworkUpdate
}
export interface MetadataSaveStatus {
  file_id: string
  operation_id: string | null
  state: 'idle' | 'saving' | 'completed' | 'failed'
  error: string | null
}
export function getMetadata(id: string): Promise<MediaMetadata> { return jsonRequest(`/api/files/${id}/metadata`) }
export function saveMetadata(id: string, update: MetadataUpdate): Promise<MetadataSaveStatus> {
  return jsonRequest(`/api/files/${id}/metadata`, { method: 'PUT', body: JSON.stringify(update) })
}
export function getMetadataStatus(id: string): Promise<MetadataSaveStatus> { return jsonRequest(`/api/files/${id}/metadata/status`) }
