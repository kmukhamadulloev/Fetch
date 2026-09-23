<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { QrCode, Radio, RefreshCw, Settings, X } from '@lucide/vue'
import RuntimeSummary from '@/components/RuntimeSummary.vue'
import AppLogo from '@/components/AppLogo.vue'
import AppNavigation from '@/components/AppNavigation.vue'
import NetworkQrDialog from '@/components/NetworkQrDialog.vue'
import { useStatusStore } from '@/stores/status'
import { useRuntimeStore } from '@/stores/runtime'
import { useDownloadsStore } from '@/stores/downloads'
import { useLibraryStore } from '@/stores/library'
import { useRealtimeStore } from '@/stores/realtime'

const route = useRoute()
const { t } = useI18n()
const status = useStatusStore()
const runtime = useRuntimeStore()
const downloads = useDownloadsStore()
const library = useLibraryStore()
const realtime = useRealtimeStore()
const statusPanelOpen = ref(false)
const secondaryNoticeDismissed = ref(false)
const retrying = ref(false)
const qrOpen = ref(false)
const title = computed(() => route.meta.titleKey ? t(String(route.meta.titleKey)) : 'Fetch')
const subtitle = computed(() => route.meta.subtitleKey ? t(String(route.meta.subtitleKey)) : '')
const serverLabel = computed(() => {
  if (status.error) return t('shell.offline')
  if (realtime.connection === 'connected') return t(realtime.role === 'secondary' ? 'shell.sharedLive' : 'shell.live')
  if (realtime.connection === 'reconnecting') return t('shell.reconnecting')
  return status.value ? t('shell.connecting') : t('shell.starting')
})
const statusDot = computed(() => status.error
  ? 'bg-rose-400'
  : realtime.connection === 'reconnecting' ? 'bg-amber-400' : 'bg-emerald-400')
const showSecondaryNotice = computed(() => realtime.role === 'secondary' && !secondaryNoticeDismissed.value)

watch(() => realtime.role, (next, previous) => {
  if (next === 'secondary' && previous !== 'secondary') secondaryNoticeDismissed.value = false
})

async function retryAll() {
  retrying.value = true
  realtime.retry()
  await Promise.all([status.refresh(), runtime.refresh(), downloads.refresh(), library.refresh()])
  retrying.value = false
}
</script>

<template>
  <div class="flex h-full bg-app text-app-text">
    <a class="skip-link" href="#main-content">{{ t('shell.skip') }}</a>
    <aside class="sidebar">
      <div class="flex h-16 items-center gap-3 px-5"><AppLogo /></div>
      <AppNavigation variant="desktop" />
      <div class="border-t border-border p-3">
        <RuntimeSummary />
      </div>
    </aside>

    <main id="main-content" class="min-w-0 flex-1 overflow-y-auto pb-20 lg:pb-0" tabindex="-1">
      <header class="topbar relative">
        <div class="flex items-center gap-3"><div class="flex items-center gap-3 lg:hidden"><AppLogo /></div><div class="hidden lg:block"><h1 class="text-sm font-semibold sm:text-base">{{ title }}</h1><p class="text-xs text-muted">{{ subtitle }}</p></div></div>
        <div class="flex items-center gap-2">
          <button class="toolbar-btn" type="button" :aria-label="serverLabel" :aria-expanded="statusPanelOpen" aria-controls="connection-status-panel" @click="statusPanelOpen = !statusPanelOpen"><span class="size-2 rounded-full" :class="statusDot"></span><span>{{ serverLabel }}</span></button>
          <button class="icon-btn" type="button" :aria-label="t('shell.openMobile')" @click="qrOpen = true"><QrCode :size="17" /></button>
          <RouterLink class="icon-btn" to="/settings" :aria-label="t('shell.settings')"><Settings :size="16" /></RouterLink>
        </div>
        <div v-if="statusPanelOpen" id="connection-status-panel" class="connection-popover" role="status">
          <div class="flex items-start gap-3">
            <Radio class="mt-0.5 shrink-0 text-accent" :size="17" />
            <div class="min-w-0 flex-1">
              <div class="text-xs font-semibold">{{ realtime.role === 'primary' ? t('shell.primaryTab') : realtime.role === 'secondary' ? t('shell.secondaryTab') : t('shell.choosingTab') }}</div>
              <p class="mt-1 text-[11px] leading-5 text-muted">
                {{ realtime.role === 'primary' ? t('shell.primaryHelp') : realtime.role === 'secondary' ? t('shell.secondaryHelp') : t('shell.choosingHelp') }}
              </p>
              <p v-if="status.error || realtime.error" class="mt-2 text-[11px] leading-5 text-rose-300">{{ status.error ?? realtime.error }}</p>
              <div class="mt-3 flex flex-wrap gap-2">
                <button v-if="realtime.role === 'secondary'" class="secondary-btn" type="button" @click="realtime.takeOver(); statusPanelOpen = false">{{ t('shell.makePrimary') }}</button>
                <button v-if="status.error || realtime.error" class="secondary-btn" type="button" :disabled="retrying" @click="retryAll"><RefreshCw :class="{ 'animate-spin': retrying }" :size="14" />{{ retrying ? t('shell.retrying') : t('shell.retryConnection') }}</button>
              </div>
            </div>
          </div>
        </div>
      </header>
      <div v-if="showSecondaryNotice" class="connection-notice" role="status">
        <Radio class="shrink-0 text-accent" :size="18" />
        <p class="min-w-0 flex-1"><strong>{{ t('shell.anotherPrimary') }}</strong> {{ t('shell.sharedHelp') }}</p>
        <button class="secondary-btn shrink-0" type="button" @click="realtime.takeOver()">{{ t('shell.makePrimaryShort') }}</button>
        <button class="icon-btn shrink-0" type="button" :aria-label="t('shell.dismissNotice')" @click="secondaryNoticeDismissed = true"><X :size="15" /></button>
      </div>
      <div v-if="status.error" class="connection-notice connection-error" role="alert">
        <p class="min-w-0 flex-1"><strong>{{ t('shell.unavailable') }}</strong> {{ t('shell.unavailableHelp') }} {{ status.error }}</p>
        <button class="secondary-btn shrink-0" type="button" :disabled="retrying" @click="retryAll"><RefreshCw :class="{ 'animate-spin': retrying }" :size="14" />{{ retrying ? t('shell.retrying') : t('shell.retry') }}</button>
      </div>
      <div class="mx-auto w-full max-w-[1360px] p-4 sm:p-6 lg:p-8"><RouterView /></div>
    </main>
    <AppNavigation variant="mobile" />
    <NetworkQrDialog v-if="qrOpen" @close="qrOpen = false" />
  </div>
</template>
