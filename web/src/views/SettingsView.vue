<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  Bot, Check, CheckCircle2, Clipboard, Cpu, Download, Eye, EyeOff, ExternalLink, FileClock, Film,
  KeyRound, Network, RefreshCw, RotateCcw, SlidersHorizontal, SquareTerminal, Trash2, TriangleAlert, Waypoints, Wrench,
} from '@lucide/vue'
import { useRuntimeStore } from '@/stores/runtime'
import { useSettingsStore } from '@/stores/settings'
import { useProxyStore } from '@/stores/proxy'
import { useTelegramStore } from '@/stores/telegram'
import { useAppearance, type ThemePreference } from '@/stores/appearance'
import { setLocale, supportedLocales, type AppLocale } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import type { ApplicationSettings, ProxyMode, ProxySettings, RuntimeComponent, TelegramSettings } from '@/app/api/client'

const tabs = [
  { id: 'general', label: 'settings.general', icon: SlidersHorizontal },
  { id: 'downloads', label: 'settings.downloads', icon: Download },
  { id: 'network', label: 'settings.network', icon: Network },
  { id: 'integrations', label: 'settings.integrations', icon: Bot },
  { id: 'runtime', label: 'settings.runtime', icon: Cpu },
  { id: 'advanced', label: 'settings.advanced', icon: Wrench },
] as const
type TabId = typeof tabs[number]['id']

const route = useRoute()
const router = useRouter()
const { t, locale } = useI18n()
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
const telegramTokenVisible = ref(false)

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
  if (!value || (telegramForm.value && active.value === 'integrations')) return
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
const telegramUserValues = computed(() => telegramUsers.value.split(/\s+/).filter(Boolean))
const telegramParsedUserIds = computed(() => telegramUserValues.value.map((value) => Number(value)))
const telegramUserIdsValid = computed(() => telegramUserValues.value.length > 0
  && telegramUserValues.value.length <= 64
  && new Set(telegramParsedUserIds.value).size === telegramParsedUserIds.value.length
  && telegramParsedUserIds.value.every((value) => Number.isSafeInteger(value) && value > 0))
const telegramUploadLimitValid = computed(() => !telegramForm.value?.send_completed_media
  || (Number.isInteger(telegramForm.value.upload_limit_mb)
    && telegramForm.value.upload_limit_mb >= 1
    && telegramForm.value.upload_limit_mb <= 50))
const telegramSetupIssues = computed(() => {
  if (!telegramForm.value?.enabled || !telegram.value) return []
  const issues: string[] = []
  if (!telegram.value.status.token_configured) issues.push(t('settings.tokenRequired'))
  if (!telegramUserIdsValid.value) issues.push(t('settings.usersRequired'))
  if (!telegramUploadLimitValid.value) issues.push(t('settings.uploadInvalid'))
  if (telegramForm.value.send_completed_media && !telegramForm.value.notify_completed) issues.push(t('settings.completedRequired'))
  if (!telegramForm.value.privacy_acknowledged) issues.push(t('settings.privacyRequired'))
  return issues
})

function javascriptRuntimeLabel(name: 'deno' | 'node' | 'quickjs') {
  return name === 'quickjs' ? 'QuickJS' : `${name[0].toUpperCase()}${name.slice(1)}`
}
function statusLabel(status: string | undefined) {
  return t(`statuses.${status ?? 'checking'}`)
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

function changeLocale(event: Event) {
  setLocale((event.target as HTMLSelectElement).value as AppLocale)
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
    settings.error = t('settings.clipboardError')
  }
}

function parsedTelegramUsers() {
  return telegramParsedUserIds.value
}

async function saveTelegramSettings() {
  if (!telegramForm.value || !isHost.value) return
  if (telegramSetupIssues.value.length) {
    telegram.error = telegramSetupIssues.value[0]
    return
  }
  const users = parsedTelegramUsers()
  if (users.some((value) => !Number.isSafeInteger(value) || value <= 0)) {
    telegram.error = t('settings.usersInvalid')
    return
  }
  await telegram.saveSettings({ ...telegramForm.value, allowed_user_ids: users })
}

async function saveTelegramToken() {
  if (!telegramToken.value.trim() || !isHost.value) return
  if (await telegram.saveToken(telegramToken.value)) {
    telegramToken.value = ''
    telegramTokenVisible.value = false
  }
}
</script>

<template>
  <section>
    <p class="eyebrow">{{ t('settings.eyebrow') }}</p>
    <h2 class="mt-2 text-2xl font-semibold">{{ t('settings.title') }}</h2>
    <p class="mt-1 text-sm text-muted">{{ t('settings.description') }}</p>
    <p v-if="settings.error || runtime.error || proxy.error || telegram.error" class="error-panel mt-5" role="alert">{{ settings.error ?? runtime.error ?? proxy.error ?? telegram.error }}</p>

    <div class="settings-layout mt-6">
      <nav class="settings-nav card" :aria-label="t('settings.sections')">
        <button v-for="tab in tabs" :key="tab.id" class="settings-tab" :class="{ active: active === tab.id }" type="button" @click="active = tab.id">
          <component :is="tab.icon" :size="16" />{{ t(tab.label) }}
        </button>
      </nav>

      <div v-if="form" class="min-w-0">
        <section v-if="active === 'general'" class="card p-5 sm:p-6" aria-labelledby="settings-general">
          <h3 id="settings-general" class="text-sm font-semibold">{{ t('settings.general') }}</h3>
          <p class="mt-1 text-xs text-muted">{{ t('settings.generalDescription') }}</p>
          <div class="mt-6 space-y-5">
            <label class="setting-row">
              <span><span class="setting-title">{{ t('settings.openBrowser') }}</span><span class="setting-help">{{ t('settings.openBrowserHelp') }}</span></span>
              <input v-model="form.open_browser_on_start" type="checkbox" />
            </label>
            <label class="setting-row">
              <span><span class="setting-title">{{ t('settings.startSystem') }}</span><span class="setting-help">{{ settings.network && !settings.network.local_client ? t('settings.startSystemRemote') : t('settings.startSystemHelp') }}</span></span>
              <input v-model="form.start_with_system" type="checkbox" :disabled="Boolean(settings.network && !settings.network.local_client)" />
            </label>
            <label class="setting-row">
              <span><span class="setting-title">{{ t('settings.theme') }}</span><span class="setting-help">{{ t('settings.themeHelp') }}</span></span>
              <select class="select setting-control" :value="appearance.preference.value" :aria-label="t('settings.theme')" @change="changeTheme">
                <option value="system">{{ t('settings.useSystem') }}</option><option value="dark">{{ t('settings.dark') }}</option><option value="light">{{ t('settings.light') }}</option>
              </select>
            </label>
            <label class="setting-row">
              <span><span class="setting-title">{{ t('language.label') }}</span><span class="setting-help">{{ t('language.help') }}</span></span>
              <select class="select setting-control" :value="locale" :aria-label="t('language.label')" @change="changeLocale">
                <option v-for="value in supportedLocales" :key="value" :value="value">{{ t(value === 'en' ? 'language.english' : value === 'ru' ? 'language.russian' : 'language.tajik') }}</option>
              </select>
            </label>
          </div>
          <p class="mt-5 text-[11px] leading-5 text-muted">{{ t('settings.localPreferenceHelp') }}</p>
        </section>

        <section v-else-if="active === 'downloads'" class="card p-5 sm:p-6" aria-labelledby="settings-downloads">
          <h3 id="settings-downloads" class="text-sm font-semibold">{{ t('settings.downloads') }}</h3>
          <p class="mt-1 text-xs text-muted">{{ t('settings.downloadsDescription') }}</p>
          <div class="mt-6 grid gap-5">
            <label class="field"><span>{{ t('settings.downloadDirectory') }}</span><input v-model="form.download_directory" class="input font-mono text-xs" autocomplete="off" /></label>
            <label class="field max-w-xs"><span>{{ t('settings.concurrentDownloads') }}</span><select v-model.number="form.concurrent_downloads" class="select"><option v-for="count in 16" :key="count" :value="count">{{ count }}</option></select></label>
          </div>
          <div class="info-panel mt-5"><Download :size="18" class="shrink-0" /><span>{{ t('settings.downloadsApply') }}</span></div>
        </section>

        <section v-else-if="active === 'network'" class="space-y-5" aria-labelledby="settings-network">
          <div class="card p-5 sm:p-6">
            <h3 id="settings-network" class="text-sm font-semibold">{{ t('settings.webInterface') }}</h3>
            <p class="mt-1 text-xs leading-5 text-muted">{{ t('settings.webHelp') }}</p>
            <div class="mt-6 grid gap-4 sm:grid-cols-2">
              <label class="field"><span>{{ t('settings.bindAddress') }}</span><input v-model="form.bind_address" class="input font-mono" autocomplete="off" /></label>
              <label class="field"><span>{{ t('settings.port') }}</span><input v-model.number="form.port" class="input font-mono" type="number" min="1" max="65535" /></label>
              <label class="field sm:col-span-2"><span>{{ t('settings.allowedNetworks') }}</span><textarea v-model="networks" class="input min-h-28 resize-y font-mono text-xs" spellcheck="false"></textarea></label>
            </div>
            <div v-if="settings.network" class="network-urls mt-5">
              <div class="text-[10px] font-semibold uppercase tracking-wider text-muted">{{ t('settings.availableUrls') }}</div>
              <div v-for="url in settings.network.urls" :key="url" class="network-url-row">
                <code>{{ url }}</code>
                <button class="icon-btn shrink-0" type="button" :aria-label="t('settings.copyUrl', { url })" @click="copyUrl(url)"><Check v-if="copiedUrl === url" :size="15" /><Clipboard v-else :size="15" /></button>
              </div>
            </div>
            <p class="mt-4 text-[11px] leading-5 text-muted">{{ t('settings.networkApply') }}</p>
          </div>
          <div class="card p-5 sm:p-6" aria-labelledby="settings-proxy">
            <div class="flex items-start gap-3 sm:gap-4">
              <div class="empty-icon shrink-0"><Waypoints :size="18" /></div>
              <div class="min-w-0 flex-1">
                <h3 id="settings-proxy" class="text-sm font-semibold">{{ t('settings.outbound') }}</h3>
                <p class="mt-1 text-xs leading-5 text-muted">{{ t('settings.outboundHelp') }}</p>
              </div>
            </div>
            <div v-if="!isHost" class="info-panel mt-5">
              <Network :size="18" class="shrink-0" />
              <span>{{ t('settings.proxyRemote') }}</span>
            </div>
            <div v-else-if="proxy.loading" class="mt-5 min-h-24 animate-pulse rounded-xl bg-[var(--app-surface-2)]" :aria-label="t('settings.proxyLoading')"></div>
            <div v-else-if="!proxyForm" class="error-panel mt-5" role="alert">
              <span>{{ t('settings.proxyLoadError') }}</span>
              <button class="secondary-btn ml-auto shrink-0" type="button" @click="proxy.refresh">{{ t('common.retry') }}</button>
            </div>
            <div v-else class="mt-5 space-y-4">
              <label class="field max-w-md">
                <span>{{ t('settings.connectionMode') }}</span>
                <select class="select" :value="proxyForm.mode" :aria-label="t('settings.proxyMode')" @change="changeProxyMode">
                  <option value="system">{{ t('settings.proxySystem') }}</option>
                  <option value="direct">{{ t('settings.proxyDirect') }}</option>
                  <option value="custom">{{ t('settings.proxyCustom') }}</option>
                </select>
              </label>
              <label v-if="proxyForm.mode === 'custom'" class="field">
                <span>{{ t('settings.proxyUrl') }}</span>
                <input v-model="proxyForm.url" class="input font-mono text-xs" type="url" inputmode="url" autocomplete="off" placeholder="socks5://127.0.0.1:1080" aria-describedby="proxy-help" />
              </label>
              <p id="proxy-help" class="text-[11px] leading-5 text-muted">{{ t('settings.proxyHelp') }}</p>
              <div class="flex min-h-10 flex-wrap items-center justify-between gap-3">
                <span class="text-xs" aria-live="polite"><span v-if="proxy.saved" class="helper text-emerald-500"><CheckCircle2 :size="15" />{{ t('settings.proxySaved') }}</span></span>
                <button class="secondary-btn" type="button" :disabled="proxy.saving" @click="saveProxy">{{ proxy.saving ? t('settings.saving') : t('settings.saveProxy') }}</button>
              </div>
            </div>
          </div>
          <div class="warning-panel"><TriangleAlert class="shrink-0 text-amber-500" :size="20" /><div><div class="text-xs font-medium">{{ t('settings.noLogin') }}</div><p class="mt-1 text-xs leading-5 opacity-70">{{ t('settings.noLoginHelp') }}</p></div></div>
        </section>

        <section v-else-if="active === 'integrations'" class="space-y-5" aria-labelledby="settings-integrations">
          <div class="card p-5 sm:p-6">
            <div class="flex items-start gap-3 sm:gap-4">
              <div class="empty-icon shrink-0"><Bot :size="18" /></div>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 id="settings-integrations" class="text-sm font-semibold">{{ t('settings.telegramBot') }}</h3>
                  <span v-if="telegram.value" class="badge" :class="{ muted: telegram.value.status.state !== 'connected' }">{{ statusLabel(telegram.value.status.state) }}</span>
                </div>
                <p class="mt-1 text-xs leading-5 text-muted">{{ t('settings.telegramDescription') }}</p>
              </div>
            </div>
            <div v-if="!isHost" class="info-panel mt-5"><Network :size="18" class="shrink-0" /><span>{{ t('settings.telegramRemote') }}</span></div>
            <div v-else-if="telegram.loading" class="mt-5 min-h-36 animate-pulse rounded-xl bg-[var(--app-surface-2)]" :aria-label="t('settings.telegramLoading')"></div>
            <div v-else-if="telegramForm && telegram.value" class="mt-6 space-y-5">
              <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3">
                <span><span class="setting-title">{{ t('settings.enableTelegram') }}</span><span class="setting-help">{{ t('settings.enableTelegramHelp') }}</span></span>
                <input v-model="telegramForm.enabled" type="checkbox" />
              </label>

              <div class="runtime-summary" role="status" aria-live="polite">
                <div class="flex items-center justify-between gap-3 text-xs font-medium">
                  <span>{{ t('settings.telegramSetup') }}</span>
                  <span class="badge" :class="{ muted: !telegramForm.enabled || telegramSetupIssues.length }">{{ !telegramForm.enabled ? t('settings.disabled') : telegramSetupIssues.length ? t('settings.actionNeeded') : t('settings.readyToSave') }}</span>
                </div>
                <p v-if="!telegramForm.enabled" class="mt-2 text-[11px] leading-5 text-muted">{{ t('settings.telegramDisabledHelp') }}</p>
                <ul v-else-if="telegramSetupIssues.length" class="mt-2 space-y-1 text-[11px] leading-5 text-amber-500">
                  <li v-for="issue in telegramSetupIssues" :key="issue" class="flex items-start gap-2"><TriangleAlert class="mt-0.5 shrink-0" :size="13" />{{ issue }}</li>
                </ul>
                <p v-else class="mt-2 text-[11px] leading-5 text-emerald-500">{{ t('settings.telegramReadyHelp') }}</p>
              </div>

              <fieldset class="space-y-5 transition-opacity" :class="{ 'opacity-50': !telegramForm.enabled }" :disabled="!telegramForm.enabled">
                <div class="rounded-xl bg-[var(--app-surface-2)] p-4">
                  <div class="flex flex-wrap items-center justify-between gap-3">
                    <div><div class="text-xs font-medium">{{ t('settings.botToken') }}</div><p class="mt-1 text-[11px] text-muted">{{ telegram.value.status.token_configured ? t('settings.configuredVia', { source: telegram.value.status.token_source }) : t('settings.notConfigured') }}<template v-if="telegram.value.status.bot_username"> · @{{ telegram.value.status.bot_username }}</template><template v-if="telegram.value.status.last_success_at"> · {{ t('settings.lastContact', { date: formatDateTime(telegram.value.status.last_success_at, locale) }) }}</template></p></div>
                    <button v-if="telegram.value.status.token_configured && telegram.value.status.token_source === 'native'" class="icon-btn" type="button" :aria-label="t('settings.removeToken')" :disabled="telegram.saving" @click="telegram.removeToken"><Trash2 :size="15" /></button>
                  </div>
                  <div class="mt-4 flex flex-col gap-2 sm:flex-row">
                    <label class="field min-w-0 flex-1">
                      <span class="sr-only">{{ t('settings.newToken') }}</span>
                      <span class="relative block">
                        <input v-model="telegramToken" class="input pr-12 font-mono text-xs" :type="telegramTokenVisible ? 'text' : 'password'" autocomplete="new-password" :placeholder="telegram.value.status.token_configured ? t('settings.replaceToken') : t('settings.pasteToken')" :disabled="telegram.value.status.token_source === 'environment'" />
                        <button class="absolute right-1 top-1/2 flex size-8 -translate-y-1/2 items-center justify-center rounded-lg text-muted hover:text-app-text disabled:opacity-40" type="button" :aria-label="telegramTokenVisible ? t('settings.hideToken') : t('settings.showToken')" :disabled="!telegramToken || telegram.value.status.token_source === 'environment'" @click="telegramTokenVisible = !telegramTokenVisible">
                          <EyeOff v-if="telegramTokenVisible" :size="16" /><Eye v-else :size="16" />
                        </button>
                      </span>
                    </label>
                    <button class="secondary-btn justify-center" type="button" :disabled="telegram.saving || !telegramToken.trim() || telegram.value.status.token_source === 'environment'" @click="saveTelegramToken"><KeyRound :size="14" />{{ t('settings.saveToken') }}</button>
                    <button class="secondary-btn justify-center" type="button" :disabled="telegram.saving || !telegram.value.status.token_configured" @click="telegram.test">{{ t('settings.testConnection') }}</button>
                  </div>
                  <p class="mt-2 text-[11px] leading-5 text-muted">{{ t('settings.tokenEyeHelp') }}</p>
                  <p v-if="telegram.value.status.token_source === 'environment'" class="mt-2 text-[11px] leading-5 text-muted">{{ t('settings.environmentTokenHelp') }}</p>
                </div>

                <div class="info-panel"><Bot :size="18" class="shrink-0" /><i18n-t keypath="settings.botFatherHelp" tag="span"><template #botFather><a class="underline" href="https://t.me/BotFather" target="_blank" rel="noreferrer">@BotFather</a></template></i18n-t></div>
                <label class="field"><span>{{ t('settings.allowedUsers') }}</span><textarea v-model="telegramUsers" class="input min-h-28 resize-y font-mono text-xs" inputmode="numeric" spellcheck="false" placeholder="123456789"></textarea><small>{{ t('settings.allowedUsersHelp') }}</small></label>
                <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3">
                  <span><span class="setting-title">{{ t('settings.useProxy') }}</span><span class="setting-help">{{ t('settings.useProxyHelp') }}</span></span>
                  <input v-model="telegramForm.use_proxy" type="checkbox" />
                </label>
                <div>
                  <div class="mb-2 text-xs font-medium">{{ t('settings.notifications') }}</div>
                  <div class="grid gap-3 sm:grid-cols-3">
                    <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3"><span class="setting-title">{{ t('settings.queued') }}</span><input v-model="telegramForm.notify_queued" type="checkbox" /></label>
                    <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3"><span class="setting-title">{{ t('settings.completed') }}</span><input v-model="telegramForm.notify_completed" type="checkbox" /></label>
                    <label class="setting-row rounded-xl bg-[var(--app-surface-2)] px-4 py-3"><span class="setting-title">{{ t('settings.failedStopped') }}</span><input v-model="telegramForm.notify_failed" type="checkbox" /></label>
                  </div>
                </div>
                <div class="rounded-xl bg-[var(--app-surface-2)] p-4">
                  <label class="setting-row">
                    <span><span class="setting-title">{{ t('settings.sendMedia') }}</span><span class="setting-help">{{ t('settings.sendMediaHelp') }}</span></span>
                    <input v-model="telegramForm.send_completed_media" type="checkbox" :aria-label="t('settings.sendMedia')" />
                  </label>
                  <label class="field mt-4 max-w-64" :class="{ 'opacity-50': !telegramForm.send_completed_media }">
                    <span>{{ t('settings.maxAttachment') }}</span>
                    <span class="relative block">
                      <input v-model.number="telegramForm.upload_limit_mb" class="input pr-12" type="number" inputmode="numeric" min="1" max="50" step="1" :disabled="!telegramForm.send_completed_media" :aria-label="t('settings.maxAttachment')" />
                      <span class="pointer-events-none absolute right-4 top-1/2 -translate-y-1/2 text-xs text-muted">MB</span>
                    </span>
                    <small>{{ t('settings.attachmentHelp') }}</small>
                  </label>
                </div>
                <label class="flex items-start gap-3 text-xs leading-5"><input v-model="telegramForm.privacy_acknowledged" class="mt-1" type="checkbox" /><span>{{ t('settings.privacy') }}</span></label>
              </fieldset>
              <div v-if="telegram.value.status.error" class="warning-panel"><TriangleAlert :size="18" class="shrink-0" /><span>{{ telegram.value.status.error }}</span></div>
              <div class="flex min-h-10 flex-wrap items-center justify-between gap-3">
                <span class="text-xs" aria-live="polite"><span v-if="telegram.saved" class="helper text-emerald-500"><CheckCircle2 :size="15" />{{ t('settings.telegramUpdated') }}</span></span>
                <button class="primary-btn" type="button" :disabled="telegram.saving || (telegramForm.enabled && telegramSetupIssues.length > 0)" @click="saveTelegramSettings">{{ telegram.saving ? t('settings.saving') : t('settings.saveTelegram') }}</button>
              </div>
            </div>
          </div>
          <div class="info-panel"><Bot :size="18" class="shrink-0" /><span>{{ t('settings.commands') }}</span></div>
        </section>

        <section v-else-if="active === 'runtime'" class="space-y-4" aria-labelledby="settings-runtime">
          <div class="flex items-end justify-between gap-4"><div><h3 id="settings-runtime" class="text-sm font-semibold">{{ t('settings.managedRuntime') }}</h3><p class="mt-1 text-xs text-muted">{{ t('settings.componentsHealthy', { ready: runtimeReady, total: runtime.components.length }) }}</p></div><button class="secondary-btn" type="button" :disabled="runtime.loading" @click="runtime.refresh"><RefreshCw :class="{ 'animate-spin': runtime.loading }" :size="14" />{{ t('settings.refresh') }}</button></div>
          <div class="runtime-card">
            <div class="flex items-start gap-3 sm:gap-4"><div class="empty-icon shrink-0"><SquareTerminal :size="18" /></div><div class="min-w-0 flex-1"><div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">yt-dlp</span><span class="badge" :class="{ muted: ytdlp?.status !== 'ready' }">{{ statusLabel(ytdlp?.status) }}</span></div><p class="mt-1 break-all text-xs text-muted">{{ ytdlp?.version ?? ytdlp?.error ?? t('settings.versionUnavailable') }}</p><div class="mt-4 flex flex-wrap gap-2"><button v-if="ytdlp?.status === 'missing'" class="secondary-btn" type="button" @click="runtime.act('yt-dlp', 'install')">{{ t('settings.install') }}</button><template v-else><button class="secondary-btn" type="button" :disabled="runtimeBusy(ytdlp)" @click="runtime.act('yt-dlp', 'update')"><RefreshCw :class="{ 'animate-spin': runtimeBusy(ytdlp) }" :size="14" />{{ t('settings.update') }}</button><button class="secondary-btn" type="button" :disabled="runtimeBusy(ytdlp)" @click="runtime.act('yt-dlp', 'repair')"><RotateCcw :size="14" />{{ t('settings.repair') }}</button></template></div></div></div>
          </div>
          <div class="runtime-card">
            <div class="flex items-start gap-3 sm:gap-4"><div class="empty-icon shrink-0"><Film :size="18" /></div><div class="min-w-0 flex-1"><div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">FFmpeg + FFprobe</span><span class="badge" :class="{ muted: ffmpeg?.status !== 'ready' || ffprobe?.status !== 'ready' }">{{ ffmpeg?.status === 'ready' && ffprobe?.status === 'ready' ? statusLabel('ready') : statusLabel(ffmpeg?.status) }}</span></div><p class="mt-1 break-all text-xs text-muted">FFmpeg {{ ffmpeg?.version ?? '—' }} · FFprobe {{ ffprobe?.version ?? '—' }}</p><div class="mt-4 flex flex-wrap gap-2"><button v-if="ffmpeg?.status === 'missing'" class="secondary-btn" type="button" @click="runtime.act('ffmpeg', 'install')">{{ t('settings.installPair') }}</button><template v-else><button class="secondary-btn" type="button" :disabled="runtimeBusy(ffmpeg)" @click="runtime.act('ffmpeg', 'update')"><RefreshCw :class="{ 'animate-spin': runtimeBusy(ffmpeg) }" :size="14" />{{ t('settings.update') }}</button><button class="secondary-btn" type="button" :disabled="runtimeBusy(ffmpeg)" @click="runtime.act('ffmpeg', 'repair')"><RotateCcw :size="14" />{{ t('settings.repairPair') }}</button></template></div></div></div>
          </div>
          <div class="runtime-card">
            <div class="flex min-w-0 items-start gap-3 sm:gap-4">
              <div class="empty-icon shrink-0"><Cpu :size="18" /></div>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2"><span class="text-sm font-semibold">{{ t('settings.jsChallenges') }}</span><span class="badge">{{ form.ytdlp_js_runtime }}</span></div>
                <p class="mt-1 text-xs leading-5 text-muted">{{ t('settings.jsHelp') }}</p>
                <label class="field mt-4 max-w-md">
                  <span>{{ t('settings.jsRuntime') }}</span>
                  <select v-model="form.ytdlp_js_runtime" class="select" :aria-label="t('settings.jsRuntime')">
                    <option value="auto">{{ t('settings.automatic') }}</option>
                    <option value="deno">Deno</option>
                    <option value="node">Node</option>
                    <option value="quickjs">QuickJS</option>
                    <option value="disabled">{{ t('settings.disabledOption') }}</option>
                  </select>
                </label>
                <div class="mt-4 grid min-w-0 gap-2 sm:grid-cols-3" :aria-label="t('settings.detectedRuntimes')">
                  <div v-for="item in runtime.javascript" :key="item.name" class="min-w-0 rounded-lg bg-[var(--app-surface-2)] px-3 py-2">
                    <div class="flex items-center justify-between gap-2 text-xs font-medium"><span>{{ javascriptRuntimeLabel(item.name) }}</span><span :class="item.detected ? 'text-emerald-500' : 'text-muted'">{{ item.detected ? t('settings.detected') : t('settings.notFound') }}</span></div>
                    <p class="mt-1 truncate font-mono text-[10px] text-muted" :title="item.version ?? undefined">{{ item.version ?? '—' }}</p>
                  </div>
                </div>
                <p v-if="selectedJsRuntimeMissing" class="mt-3 text-xs leading-5 text-amber-500" role="alert">{{ t('settings.selectedMissing') }}</p>
                <p class="mt-3 text-[11px] leading-5 text-muted">{{ t('settings.automaticHelp') }}</p>
              </div>
            </div>
          </div>
          <label class="setting-row card px-5 py-4"><span><span class="setting-title">{{ t('settings.autoUpdate') }}</span><span class="setting-help">{{ t('settings.autoUpdateHelp') }}</span></span><input v-model="form.ytdlp_auto_update" type="checkbox" /></label>
        </section>

        <section v-else class="card p-5 sm:p-6" aria-labelledby="settings-advanced">
          <h3 id="settings-advanced" class="text-sm font-semibold">{{ t('settings.advanced') }}</h3>
          <p class="mt-1 text-xs text-muted">{{ t('settings.advancedDescription') }}</p>
          <div class="mt-6 grid gap-3 sm:grid-cols-2">
            <RouterLink class="action-card" to="/history"><FileClock :size="19" /><span><strong>{{ t('settings.downloadHistory') }}</strong><small>{{ t('settings.downloadHistoryHelp') }}</small></span><ExternalLink :size="15" /></RouterLink>
            <RouterLink class="action-card" to="/logs"><SquareTerminal :size="19" /><span><strong>{{ t('settings.logsDiagnostics') }}</strong><small>{{ t('settings.logsDiagnosticsHelp') }}</small></span><ExternalLink :size="15" /></RouterLink>
          </div>
          <div class="info-panel mt-5"><Wrench :size="18" class="shrink-0" /><span>{{ t('settings.advancedHelp') }}</span></div>
        </section>

        <div v-if="active !== 'advanced' && active !== 'integrations'" class="settings-savebar">
          <span class="min-h-5 text-xs" aria-live="polite"><span v-if="settings.saved" class="helper text-emerald-500"><CheckCircle2 :size="15" />{{ t('settings.saved') }}</span></span>
          <button class="primary-btn" type="button" :disabled="settings.saving" @click="save">{{ settings.saving ? t('settings.saving') : t('settings.saveSettings') }}</button>
        </div>
      </div>
      <div v-else class="card min-h-80 animate-pulse" :aria-label="t('settings.loadingSettings')"></div>
    </div>
  </section>
</template>
