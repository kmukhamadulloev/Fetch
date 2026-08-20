<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Bot, Check, CheckCircle2, Clipboard, Cpu, Download, ExternalLink, FileClock, Film,
  KeyRound, Network, RefreshCw, RotateCcw, SlidersHorizontal, SquareTerminal, Trash2, TriangleAlert, Waypoints, Wrench,
} from '@lucide/vue'
import { useRuntimeStore } from '@/stores/runtime'
import { useSettingsStore } from '@/stores/settings'
import { useProxyStore } from '@/stores/proxy'
import { useTelegramStore } from '@/stores/telegram'
import { useAppearance, type ThemePreference } from '@/stores/appearance'
import type { ApplicationSettings, ProxyMode, ProxySettings, RuntimeComponent, TelegramSettings } from '@/app/api/client'

const tabs = [
  { id: 'general', label: 'General', icon: SlidersHorizontal },
  { id: 'downloads', label: 'Downloads', icon: Download },
  { id: 'network', label: 'Network', icon: Network },
  { id: 'integrations', label: 'Integrations', icon: Bot },
  { id: 'runtime', label: 'Runtime', icon: Cpu },
  { id: 'advanced', label: 'Advanced', icon: Wrench },
] as const
type TabId = typeof tabs[number]['id']

const route = useRoute()
const router = useRouter()
const settings = useSettingsStore()
const runtime = useRuntimeStore()
const proxy = useProxyStore()
const telegram = useTelegramStore()
const appearance = useAppearance()
const active = ref<TabId>('general')
const form = ref<ApplicationSettings | null>(null)
const networks = ref('')
const copiedUrl = ref<string | null>(null)
const proxyForm = ref<ProxySettings | null>(null)
const telegramForm = ref<TelegramSettings | null>(null)
const telegramUsers = ref('')
const telegramToken = ref('')

function tabFromHash(hash: string): TabId {
  const candidate = hash.replace('#', '') as TabId
  return tabs.some((tab) => tab.id === candidate) ? candidate : 'general'
}

watch(() => route.hash, (hash) => { active.value = tabFromHash(hash) }, { immediate: true })
watch(active, (tab) => {
  if (route.hash !== `#${tab}`) void router.replace({ hash: `#${tab}` })
})
watch(() => settings.value, (value) => {
  if (!value) return
  form.value = { ...value, allowed_networks: [...value.allowed_networks] }
  networks.value = value.allowed_networks.join('\n')
}, { immediate: true })
watch(() => proxy.value, (value) => {
  proxyForm.value = value ? { ...value } : null
}, { immediate: true })
watch(() => telegram.value, (value) => {
  if (!value) return
  telegramForm.value = { ...value.settings, allowed_user_ids: [...value.settings.allowed_user_ids] }
  telegramUsers.value = value.settings.allowed_user_ids.join('\n')
}, { immediate: true })

const runtimeReady = computed(() => runtime.components.filter((item) => item.status === 'ready').length)
const ytdlp = computed(() => runtime.components.find((item) => item.name === 'yt-dlp'))
const ffmpeg = computed(() => runtime.components.find((item) => item.name === 'ffmpeg'))
const ffprobe = computed(() => runtime.components.find((item) => item.name === 'ffprobe'))
const runtimeBusy = (component?: RuntimeComponent) => component?.status === 'installing' || component?.status === 'updating'
const isHost = computed(() => settings.network?.local_client === true)
const selectedJsRuntimeMissing = computed(() => {
  const selected = form.value?.ytdlp_js_runtime
  if (!selected || selected === 'auto' || selected === 'disabled') return false
  return runtime.javascript.find((item) => item.name === selected)?.detected === false
})

function javascriptRuntimeLabel(name: 'deno' | 'node' | 'quickjs') {
  return name === 'quickjs' ? 'QuickJS' : `${name[0].toUpperCase()}${name.slice(1)}`
}

onMounted(async () => {
  await Promise.all([settings.refresh(), runtime.refresh()])
  if (isHost.value) await Promise.all([proxy.refresh(), telegram.refresh()])
})

function save() {
  if (!form.value) return
  void settings.save({ ...form.value, allowed_networks: networks.value.split(/\s+/).filter(Boolean) })
}

function changeTheme(event: Event) {
  appearance.setTheme((event.target as HTMLSelectElement).value as ThemePreference)
}

function changeProxyMode(event: Event) {
  if (!proxyForm.value) return
  const mode = (event.target as HTMLSelectElement).value as ProxyMode
  proxyForm.value = { mode, url: mode === 'custom' ? (proxyForm.value.url ?? '') : null }
}

function saveProxy() {
  if (!proxyForm.value || !isHost.value) return
  void proxy.save({
    mode: proxyForm.value.mode,
    url: proxyForm.value.mode === 'custom' ? proxyForm.value.url : null,
  })
}

async function copyUrl(url: string) {
  try {
    await navigator.clipboard.writeText(url)
    copiedUrl.value = url
    window.setTimeout(() => { if (copiedUrl.value === url) copiedUrl.value = null }, 1800)
  } catch {
    settings.error = 'The browser did not allow clipboard access. Select and copy the URL manually.'
  }
}

function parsedTelegramUsers() {
  return telegramUsers.value.split(/\s+/).filter(Boolean).map((value) => Number(value))
}

async function saveTelegramSettings() {
  if (!telegramForm.value || !isHost.value) return
  const users = parsedTelegramUsers()
  if (users.some((value) => !Number.isSafeInteger(value) || value <= 0)) {
    telegram.error = 'Telegram user IDs must be positive whole numbers.'
    return
  }
  await telegram.saveSettings({ ...telegramForm.value, allowed_user_ids: users })
}

async function saveTelegramToken() {
  if (!telegramToken.value.trim() || !isHost.value) return
  if (await telegram.saveToken(telegramToken.value)) telegramToken.value = ''
}
</script>

<template>
  <section>
    <p class="eyebrow">Configuration</p>
    <h2 class="mt-2 text-2xl font-semibold">Settings</h2>
    <p class="mt-1 text-sm text-muted">Application, network, download, and runtime preferences.</p>
    <p v-if="settings.error || runtime.error || proxy.error || telegram.error" class="error-panel mt-5" role="alert">{{ settings.error ?? runtime.error ?? proxy.error ?? telegram.error }}</p>

    <div class="settings-layout mt-6">
      <nav class="settings-nav card" aria-label="Settings sections">
        <button v-for="tab in tabs" :key="tab.id" class="settings-tab" :class="{ active: active === tab.id }" type="button" @click="active = tab.id">
          <component :is="tab.icon" :size="16" />{{ tab.label }}
        </button>
      </nav>

      <div v-if="form" class="min-w-0">
        <section v-if="active === 'general'" class="card p-5 sm:p-6" aria-labelledby="settings-general">
          <h3 id="settings-general" class="text-sm font-semibold">General</h3>
          <p class="mt-1 text-xs text-muted">Appearance and startup behavior for this Fetch interface.</p>
          <div class="mt-6 space-y-5">
            <label class="setting-row">
              <span><span class="setting-title">Open browser on startup</span><span class="setting-help">Open the local web UI when Fetch starts.</span></span>
              <input v-model="form.open_browser_on_start" type="checkbox" />
            </label>
            <label class="setting-row">
              <span><span class="setting-title">Start Fetch with system</span><span class="setting-help">{{ settings.network && !settings.network.local_client ? 'Change this setting on the device running Fetch.' : 'Launch Fetch in the tray when you sign in, without opening a browser.' }}</span></span>
              <input v-model="form.start_with_system" type="checkbox" :disabled="Boolean(settings.network && !settings.network.local_client)" />
            </label>
            <label class="setting-row">
              <span><span class="setting-title">Theme</span><span class="setting-help">System follows the appearance preference of this device.</span></span>
              <select class="select setting-control" :value="appearance.preference.value" aria-label="Theme" @change="changeTheme">
                <option value="system">Use system</option><option value="dark">Dark</option><option value="light">Light</option>
              </select>
            </label>
          </div>
          <p class="mt-5 text-[11px] leading-5 text-muted">Theme is saved on this device. Server behavior settings are saved for every client.</p>
        </section>

        <section v-else-if="active === 'downloads'" class="card p-5 sm:p-6" aria-labelledby="settings-downloads">
          <h3 id="settings-downloads" class="text-sm font-semibold">Downloads</h3>
          <p class="mt-1 text-xs text-muted">Default output location and queue capacity.</p>
          <div class="mt-6 grid gap-5">
            <label class="field"><span>Download directory</span><input v-model="form.download_directory" class="input font-mono text-xs" autocomplete="off" /></label>
            <label class="field max-w-xs"><span>Concurrent downloads</span><select v-model.number="form.concurrent_downloads" class="select"><option v-for="count in 16" :key="count" :value="count">{{ count }}</option></select></label>
          </div>
          <div class="info-panel mt-5"><Download :size="18" class="shrink-0" /><span>New jobs use the directory immediately, and queued jobs react to concurrency changes. Active downloads continue without interruption.</span></div>
        </section>

        <section v-else-if="active === 'network'" class="space-y-5" aria-labelledby="settings-network">
          <div class="card p-5 sm:p-6">
            <h3 id="settings-network" class="text-sm font-semibold">Web interface</h3>
            <p class="mt-1 text-xs leading-5 text-muted">Control which local and LAN clients can reach Fetch.</p>
            <div class="mt-6 grid gap-4 sm:grid-cols-2">
              <label class="field"><span>Bind address</span><input v-model="form.bind_address" class="input font-mono" autocomplete="off" /></label>
              <label class="field"><span>Port</span><input v-model.number="form.port" class="input font-mono" type="number" min="1" max="65535" /></label>
              <label class="field sm:col-span-2"><span>Allowed CIDR networks, one per line</span><textarea v-model="networks" class="input min-h-28 resize-y font-mono text-xs" spellcheck="false"></textarea></label>
            </div>
            <div v-if="settings.network" class="network-urls mt-5">
              <div class="text-[10px] font-semibold uppercase tracking-wider text-muted">Available URLs</div>
              <div v-for="url in settings.network.urls" :key="url" class="network-url-row">
                <code>{{ url }}</code>
                <button class="icon-btn shrink-0" type="button" :aria-label="`Copy ${url}`" @click="copyUrl(url)"><Check v-if="copiedUrl === url" :size="15" /><Clipboard v-else :size="15" /></button>
              </div>
            </div>
            <p class="mt-4 text-[11px] leading-5 text-muted">Network changes apply immediately. If the address changes, Fetch moves this tab to the new URL automatically.</p>
          </div>
          <div class="card p-5 sm:p-6" aria-labelledby="settings-proxy">
            <div class="flex items-start gap-3 sm:gap-4">
              <div class="empty-icon shrink-0"><Waypoints :size="18" /></div>
              <div class="min-w-0 flex-1">
                <h3 id="settings-proxy" class="text-sm font-semibold">Outbound downloads</h3>
                <p class="mt-1 text-xs leading-5 text-muted">Choose how yt-dlp analyzes media and starts new downloads.</p>
              </div>
            </div>
            <div v-if="!isHost" class="info-panel mt-5">
              <Network :size="18" class="shrink-0" />
              <span>Proxy configuration is private to the host. Open Settings on the device running Fetch to view or change it.</span>
            </div>
            <div v-else-if="proxy.loading" class="mt-5 min-h-24 animate-pulse rounded-xl bg-[var(--app-surface-2)]" aria-label="Loading proxy settings"></div>
            <div v-else-if="!proxyForm" class="error-panel mt-5" role="alert">
              <span>Proxy settings could not be loaded.</span>
              <button class="secondary-btn ml-auto shrink-0" type="button" @click="proxy.refresh">Retry</button>
            </div>
            <div v-else class="mt-5 space-y-4">
              <label class="field max-w-md">
                <span>Connection mode</span>
                <select class="select" :value="proxyForm.mode" aria-label="Download proxy mode" @change="changeProxyMode">
                  <option value="system">System default</option>
                  <option value="direct">Direct connection</option>
                  <option value="custom">Custom proxy</option>
                </select>
              </label>
              <label v-if="proxyForm.mode === 'custom'" class="field">
                <span>Proxy URL</span>
                <input v-model="proxyForm.url" class="input font-mono text-xs" type="url" inputmode="url" autocomplete="off" placeholder="socks5://127.0.0.1:1080" aria-describedby="proxy-help" />
              </label>
              <p id="proxy-help" class="text-[11px] leading-5 text-muted">Supports unauthenticated HTTP, HTTPS, SOCKS4, and SOCKS5 proxies. Active downloads keep their current route; queued and new work use the saved mode.</p>
              <div class="flex min-h-10 flex-wrap items-center justify-between gap-3">
                <span class="text-xs" aria-live="polite"><span v-if="proxy.saved" class="helper text-emerald-500"><CheckCircle2 :size="15" />Proxy saved</span></span>
                <button class="secondary-btn" type="button" :disabled="proxy.saving" @click="saveProxy">{{ proxy.saving ? 'Saving…' : 'Save proxy' }}</button>
              </div>
            </div>
          </div>
          <div class="warning-panel"><TriangleAlert class="shrink-0 text-amber-500" :size="20" /><div><div class="text-xs font-medium">Network access has no application login</div><p class="mt-1 text-xs leading-5 opacity-70">Every allowed client can control downloads and access completed files. Fetch never opens router ports or public tunnels.</p></div></div>
        </section>

        <section v-else-if="active === 'integrations'" class="space-y-5" aria-labelledby="settings-integrations">
          <div class="card p-5 sm:p-6">
            <div class="flex items-start gap-3 sm:gap-4">
              <div class="empty-icon shrink-0"><Bot :size="18" /></div>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 id="settings-integrations" class="text-sm font-semibold">Telegram bot</h3>
                  <span v-if="telegram.value" class="badge" :class="{ muted: telegram.value.status.state !== 'connected' }">{{ telegram.value.status.state.replace('_', ' ') }}</span>
                </div>
                <p class="mt-1 text-xs leading-5 text-muted">Submit and control downloads from allowlisted private Telegram chats. Fetch uses outbound long polling and never opens a public port.</p>
              </div>
            </div>
            <div v-if="!isHost" class="info-panel mt-5"><Network :size="18" class="shrink-0" /><span>Telegram configuration is private to the host. Open this page on the device running Fetch.</span></div>
            <div v-else-if="telegram.loading" class="mt-5 min-h-36 animate-pulse rounded-xl bg-[var(--app-surface-2)]" aria-label="Loading Telegram settings"></div>
            <div v-else-if="telegramForm && telegram.value" class="mt-6 space-y-5">
              <div class="rounded-xl bg-[var(--app-surface-2)] p-4">
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <div><div class="text-xs font-medium">Bot token</div><p class="mt-1 text-[11px] text-muted">{{ telegram.value.status.token_configured ? `Configured via ${telegram.value.status.token_source}` : 'Not configured' }}<template v-if="telegram.value.status.bot_username"> · @{{ telegram.value.status.bot_username }}</template><template v-if="telegram.value.status.last_success_at"> · Last contact {{ new Date(telegram.value.status.last_success_at).toLocaleString() }}</template></p></div>
                  <button v-if="telegram.value.status.token_configured && telegram.value.status.token_source === 'native'" class="icon-btn" type="button" aria-label="Remove Telegram token" :disabled="telegram.saving" @click="telegram.removeToken"><Trash2 :size="15" /></button>
                </div>
                <div class="mt-4 flex flex-col gap-2 sm:flex-row">
                  <label class="field min-w-0 flex-1"><span class="sr-only">New bot token</span><input v-model="telegramToken" class="input font-mono text-xs" type="password" autocomplete="new-password" placeholder="Paste a BotFather token" :disabled="telegram.value.status.token_source === 'environment'" /></label>
                  <button class="secondary-btn justify-center" type="button" :disabled="telegram.saving || !telegramToken.trim() || telegram.value.status.token_source === 'environment'" @click="saveTelegramToken"><KeyRound :size="14" />Save token</button>
                  <button class="secondary-btn justify-center" type="button" :disabled="telegram.saving || !telegram.value.status.token_configured" @click="telegram.test">Test connection</button>
                </div>
                <p v-if="telegram.value.status.token_source === 'environment'" class="mt-2 text-[11px] leading-5 text-muted">The environment token overrides the native credential store and must be changed outside this page.</p>
              </div>
              <div class="info-panel"><Bot :size="18" class="shrink-0" /><span>Create a bot with Telegram's <a class="underline" href="https://t.me/BotFather" target="_blank" rel="noreferrer">@BotFather</a>, save its token here, and enter the numeric IDs of the private accounts you trust. Telegram usernames are not accepted because they can change.</span></div>
              <label class="setting-row">
                <span><span class="setting-title">Enable Telegram bot</span><span class="setting-help">Starts one outbound poller immediately after settings are saved.</span></span>
                <input v-model="telegramForm.enabled" type="checkbox" />
              </label>
              <label class="field"><span>Allowed Telegram user IDs, one per line</span><textarea v-model="telegramUsers" class="input min-h-28 resize-y font-mono text-xs" inputmode="numeric" spellcheck="false" placeholder="123456789"></textarea><small>Only matching private chats are accepted. Group chats and every other user are silently ignored.</small></label>
              <div class="grid gap-3 sm:grid-cols-3">
                <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3"><span class="setting-title">Queued</span><input v-model="telegramForm.notify_queued" type="checkbox" /></label>
                <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3"><span class="setting-title">Completed</span><input v-model="telegramForm.notify_completed" type="checkbox" /></label>
                <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3"><span class="setting-title">Failed / stopped</span><input v-model="telegramForm.notify_failed" type="checkbox" /></label>
              </div>
              <label class="flex items-start gap-3 text-xs leading-5"><input v-model="telegramForm.privacy_acknowledged" class="mt-1" type="checkbox" /><span>I understand that submitted URLs, titles, commands, Telegram identifiers, and status messages pass through Telegram's service. Fetch does not upload downloaded media.</span></label>
              <div v-if="telegram.value.status.error" class="warning-panel"><TriangleAlert :size="18" class="shrink-0" /><span>{{ telegram.value.status.error }}</span></div>
              <div class="flex min-h-10 flex-wrap items-center justify-between gap-3">
                <span class="text-xs" aria-live="polite"><span v-if="telegram.saved" class="helper text-emerald-500"><CheckCircle2 :size="15" />Telegram updated</span></span>
                <button class="primary-btn" type="button" :disabled="telegram.saving" @click="saveTelegramSettings">{{ telegram.saving ? 'Saving…' : 'Save Telegram settings' }}</button>
              </div>
            </div>
          </div>
          <div class="info-panel"><Bot :size="18" class="shrink-0" /><span>Commands: /start, /help, /status, /downloads. Send a URL to choose Video or Audio; playlists require a second confirmation.</span></div>
        </section>

        <section v-else-if="active === 'runtime'" class="space-y-4" aria-labelledby="settings-runtime">
          <div class="flex items-end justify-between gap-4"><div><h3 id="settings-runtime" class="text-sm font-semibold">Managed runtime</h3><p class="mt-1 text-xs text-muted">{{ runtimeReady }} of {{ runtime.components.length }} components healthy.</p></div><button class="secondary-btn" type="button" :disabled="runtime.loading" @click="runtime.refresh"><RefreshCw :class="{ 'animate-spin': runtime.loading }" :size="14" />Refresh</button></div>
          <div class="runtime-card">
            <div class="flex items-start gap-3 sm:gap-4"><div class="empty-icon shrink-0"><SquareTerminal :size="18" /></div><div class="min-w-0 flex-1"><div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">yt-dlp</span><span class="badge" :class="{ muted: ytdlp?.status !== 'ready' }">{{ ytdlp?.status ?? 'checking' }}</span></div><p class="mt-1 break-all text-xs text-muted">{{ ytdlp?.version ?? ytdlp?.error ?? 'Version unavailable' }}</p><div class="mt-4 flex flex-wrap gap-2"><button v-if="ytdlp?.status === 'missing'" class="secondary-btn" type="button" @click="runtime.act('yt-dlp', 'install')">Install</button><template v-else><button class="secondary-btn" type="button" :disabled="runtimeBusy(ytdlp)" @click="runtime.act('yt-dlp', 'update')"><RefreshCw :class="{ 'animate-spin': runtimeBusy(ytdlp) }" :size="14" />Update</button><button class="secondary-btn" type="button" :disabled="runtimeBusy(ytdlp)" @click="runtime.act('yt-dlp', 'repair')"><RotateCcw :size="14" />Repair</button></template></div></div></div>
          </div>
          <div class="runtime-card">
            <div class="flex items-start gap-3 sm:gap-4"><div class="empty-icon shrink-0"><Film :size="18" /></div><div class="min-w-0 flex-1"><div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">FFmpeg + FFprobe</span><span class="badge" :class="{ muted: ffmpeg?.status !== 'ready' || ffprobe?.status !== 'ready' }">{{ ffmpeg?.status === 'ready' && ffprobe?.status === 'ready' ? 'ready' : ffmpeg?.status ?? 'checking' }}</span></div><p class="mt-1 break-all text-xs text-muted">FFmpeg {{ ffmpeg?.version ?? '—' }} · FFprobe {{ ffprobe?.version ?? '—' }}</p><div class="mt-4 flex flex-wrap gap-2"><button v-if="ffmpeg?.status === 'missing'" class="secondary-btn" type="button" @click="runtime.act('ffmpeg', 'install')">Install pair</button><template v-else><button class="secondary-btn" type="button" :disabled="runtimeBusy(ffmpeg)" @click="runtime.act('ffmpeg', 'update')"><RefreshCw :class="{ 'animate-spin': runtimeBusy(ffmpeg) }" :size="14" />Update</button><button class="secondary-btn" type="button" :disabled="runtimeBusy(ffmpeg)" @click="runtime.act('ffmpeg', 'repair')"><RotateCcw :size="14" />Repair pair</button></template></div></div></div>
          </div>
          <div class="runtime-card">
            <div class="flex min-w-0 items-start gap-3 sm:gap-4">
              <div class="empty-icon shrink-0"><Cpu :size="18" /></div>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">JavaScript challenges</span><span class="badge">{{ form.ytdlp_js_runtime }}</span></div>
                <p class="mt-1 text-xs leading-5 text-muted">Let yt-dlp use a supported host runtime when an extractor needs JavaScript.</p>
                <label class="field mt-4 max-w-md">
                  <span>JavaScript runtime</span>
                  <select v-model="form.ytdlp_js_runtime" class="select" aria-label="JavaScript runtime">
                    <option value="auto">Automatic (recommended)</option>
                    <option value="deno">Deno</option>
                    <option value="node">Node</option>
                    <option value="quickjs">QuickJS</option>
                    <option value="disabled">Disabled</option>
                  </select>
                </label>
                <div class="mt-4 grid min-w-0 gap-2 sm:grid-cols-3" aria-label="Detected JavaScript runtimes">
                  <div v-for="item in runtime.javascript" :key="item.name" class="min-w-0 rounded-lg bg-[var(--app-surface-2)] px-3 py-2">
                    <div class="flex items-center justify-between gap-2 text-xs font-medium"><span>{{ javascriptRuntimeLabel(item.name) }}</span><span :class="item.detected ? 'text-emerald-500' : 'text-muted'">{{ item.detected ? 'Detected' : 'Not found' }}</span></div>
                    <p class="mt-1 truncate font-mono text-[10px] text-muted" :title="item.version ?? undefined">{{ item.version ?? '—' }}</p>
                  </div>
                </div>
                <p v-if="selectedJsRuntimeMissing" class="mt-3 text-xs leading-5 text-amber-500" role="alert">The selected runtime was not found in Fetch's process environment. Choose Automatic or install it on the host PATH.</p>
                <p class="mt-3 text-[11px] leading-5 text-muted">Automatic tries Deno, Node, then QuickJS. Saving applies immediately to analysis and queued or new downloads; an active download keeps its current runtime.</p>
              </div>
            </div>
          </div>
          <label class="setting-row card px-5 py-4"><span><span class="setting-title">Automatically update yt-dlp</span><span class="setting-help">Keep extractor support current independently of Fetch releases.</span></span><input v-model="form.ytdlp_auto_update" type="checkbox" /></label>
        </section>

        <section v-else class="card p-5 sm:p-6" aria-labelledby="settings-advanced">
          <h3 id="settings-advanced" class="text-sm font-semibold">Advanced</h3>
          <p class="mt-1 text-xs text-muted">Inspect retained activity and diagnose the local installation.</p>
          <div class="mt-6 grid gap-3 sm:grid-cols-2">
            <RouterLink class="action-card" to="/history"><FileClock :size="19" /><span><strong>Download history</strong><small>Completed, failed, and stopped jobs</small></span><ExternalLink :size="15" /></RouterLink>
            <RouterLink class="action-card" to="/logs"><SquareTerminal :size="19" /><span><strong>Logs and diagnostics</strong><small>Runtime and filesystem health details</small></span><ExternalLink :size="15" /></RouterLink>
          </div>
          <div class="info-panel mt-5"><Wrench :size="18" class="shrink-0" /><span>Fetch intentionally does not accept arbitrary yt-dlp arguments here. Validated download options keep jobs reproducible and safe for every LAN client.</span></div>
        </section>

        <div v-if="active !== 'advanced' && active !== 'integrations'" class="settings-savebar">
          <span class="min-h-5 text-xs" aria-live="polite"><span v-if="settings.saved" class="helper text-emerald-500"><CheckCircle2 :size="15" />Saved</span></span>
          <button class="primary-btn" type="button" :disabled="settings.saving" @click="save">{{ settings.saving ? 'Saving…' : 'Save settings' }}</button>
        </div>
      </div>
      <div v-else class="card min-h-80 animate-pulse" aria-label="Loading settings"></div>
    </div>
  </section>
</template>
