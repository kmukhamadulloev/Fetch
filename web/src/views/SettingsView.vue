<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Check, CheckCircle2, Clipboard, Cpu, Download, ExternalLink, FileClock, Film,
  Network, RefreshCw, RotateCcw, SlidersHorizontal, SquareTerminal, TriangleAlert, Wrench,
} from '@lucide/vue'
import { useRuntimeStore } from '@/stores/runtime'
import { useSettingsStore } from '@/stores/settings'
import { useAppearance, type ThemePreference } from '@/stores/appearance'
import type { ApplicationSettings, RuntimeComponent } from '@/app/api/client'

const tabs = [
  { id: 'general', label: 'General', icon: SlidersHorizontal },
  { id: 'downloads', label: 'Downloads', icon: Download },
  { id: 'network', label: 'Network', icon: Network },
  { id: 'runtime', label: 'Runtime', icon: Cpu },
  { id: 'advanced', label: 'Advanced', icon: Wrench },
] as const
type TabId = typeof tabs[number]['id']

const route = useRoute()
const router = useRouter()
const settings = useSettingsStore()
const runtime = useRuntimeStore()
const appearance = useAppearance()
const active = ref<TabId>('general')
const form = ref<ApplicationSettings | null>(null)
const networks = ref('')
const copiedUrl = ref<string | null>(null)

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

const runtimeReady = computed(() => runtime.components.filter((item) => item.status === 'ready').length)
const ytdlp = computed(() => runtime.components.find((item) => item.name === 'yt-dlp'))
const ffmpeg = computed(() => runtime.components.find((item) => item.name === 'ffmpeg'))
const ffprobe = computed(() => runtime.components.find((item) => item.name === 'ffprobe'))
const runtimeBusy = (component?: RuntimeComponent) => component?.status === 'installing' || component?.status === 'updating'

onMounted(async () => {
  await Promise.all([settings.refresh(), runtime.refresh()])
})

function save() {
  if (!form.value) return
  void settings.save({ ...form.value, allowed_networks: networks.value.split(/\s+/).filter(Boolean) })
}

function changeTheme(event: Event) {
  appearance.setTheme((event.target as HTMLSelectElement).value as ThemePreference)
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
</script>

<template>
  <section>
    <p class="eyebrow">Configuration</p>
    <h2 class="mt-2 text-2xl font-semibold">Settings</h2>
    <p class="mt-1 text-sm text-muted">Application, network, download, and runtime preferences.</p>
    <p v-if="settings.error || runtime.error" class="error-panel mt-5" role="alert">{{ settings.error ?? runtime.error }}</p>

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
          <div class="warning-panel"><TriangleAlert class="shrink-0 text-amber-500" :size="20" /><div><div class="text-xs font-medium">Network access has no application login</div><p class="mt-1 text-xs leading-5 opacity-70">Every allowed client can control downloads and access completed files. Fetch never opens router ports or public tunnels.</p></div></div>
        </section>

        <section v-else-if="active === 'runtime'" class="space-y-4" aria-labelledby="settings-runtime">
          <div class="flex items-end justify-between gap-4"><div><h3 id="settings-runtime" class="text-sm font-semibold">Managed runtime</h3><p class="mt-1 text-xs text-muted">{{ runtimeReady }} of {{ runtime.components.length }} components healthy.</p></div><button class="secondary-btn" type="button" :disabled="runtime.loading" @click="runtime.refresh"><RefreshCw :class="{ 'animate-spin': runtime.loading }" :size="14" />Refresh</button></div>
          <div class="runtime-card">
            <div class="flex items-start gap-3 sm:gap-4"><div class="empty-icon shrink-0"><SquareTerminal :size="18" /></div><div class="min-w-0 flex-1"><div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">yt-dlp</span><span class="badge" :class="{ muted: ytdlp?.status !== 'ready' }">{{ ytdlp?.status ?? 'checking' }}</span></div><p class="mt-1 break-all text-xs text-muted">{{ ytdlp?.version ?? ytdlp?.error ?? 'Version unavailable' }}</p><div class="mt-4 flex flex-wrap gap-2"><button v-if="ytdlp?.status === 'missing'" class="secondary-btn" type="button" @click="runtime.act('yt-dlp', 'install')">Install</button><template v-else><button class="secondary-btn" type="button" :disabled="runtimeBusy(ytdlp)" @click="runtime.act('yt-dlp', 'update')"><RefreshCw :class="{ 'animate-spin': runtimeBusy(ytdlp) }" :size="14" />Update</button><button class="secondary-btn" type="button" :disabled="runtimeBusy(ytdlp)" @click="runtime.act('yt-dlp', 'repair')"><RotateCcw :size="14" />Repair</button></template></div></div></div>
          </div>
          <div class="runtime-card">
            <div class="flex items-start gap-3 sm:gap-4"><div class="empty-icon shrink-0"><Film :size="18" /></div><div class="min-w-0 flex-1"><div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">FFmpeg + FFprobe</span><span class="badge" :class="{ muted: ffmpeg?.status !== 'ready' || ffprobe?.status !== 'ready' }">{{ ffmpeg?.status === 'ready' && ffprobe?.status === 'ready' ? 'ready' : ffmpeg?.status ?? 'checking' }}</span></div><p class="mt-1 break-all text-xs text-muted">FFmpeg {{ ffmpeg?.version ?? '—' }} · FFprobe {{ ffprobe?.version ?? '—' }}</p><div class="mt-4 flex flex-wrap gap-2"><button v-if="ffmpeg?.status === 'missing'" class="secondary-btn" type="button" @click="runtime.act('ffmpeg', 'install')">Install pair</button><template v-else><button class="secondary-btn" type="button" :disabled="runtimeBusy(ffmpeg)" @click="runtime.act('ffmpeg', 'update')"><RefreshCw :class="{ 'animate-spin': runtimeBusy(ffmpeg) }" :size="14" />Update</button><button class="secondary-btn" type="button" :disabled="runtimeBusy(ffmpeg)" @click="runtime.act('ffmpeg', 'repair')"><RotateCcw :size="14" />Repair pair</button></template></div></div></div>
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

        <div v-if="active !== 'advanced'" class="settings-savebar">
          <span class="min-h-5 text-xs" aria-live="polite"><span v-if="settings.saved" class="helper text-emerald-500"><CheckCircle2 :size="15" />Saved</span></span>
          <button class="primary-btn" type="button" :disabled="settings.saving" @click="save">{{ settings.saving ? 'Saving…' : 'Save settings' }}</button>
        </div>
      </div>
      <div v-else class="card min-h-80 animate-pulse" aria-label="Loading settings"></div>
    </div>
  </section>
</template>
