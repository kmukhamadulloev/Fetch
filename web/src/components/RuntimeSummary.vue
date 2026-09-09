<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bot, Cpu, Network, Server } from '@lucide/vue'
import { useStatusStore } from '@/stores/status'
import { useRealtimeStore } from '@/stores/realtime'
import { useRuntimeStore } from '@/stores/runtime'
import { useSettingsStore } from '@/stores/settings'
import { useTelegramStore } from '@/stores/telegram'
import { useProxyStore } from '@/stores/proxy'

const { t } = useI18n()
const status = useStatusStore()
const realtime = useRealtimeStore()
const runtime = useRuntimeStore()
const settings = useSettingsStore()
const telegram = useTelegramStore()
const proxy = useProxyStore()
const connected = computed(() => realtime.connection === 'connected')
const backend = computed(() => connected.value ? 'connected'
  : status.error || realtime.connection === 'offline' ? 'offline'
  : realtime.connection === 'reconnecting' ? 'reconnecting' : 'connecting')
const runtimeState = computed(() => !connected.value ? 'unknown'
  : runtime.error || runtime.components.some((item) => item.status === 'failed') ? 'error'
  : ['yt-dlp', 'ffmpeg', 'ffprobe'].every((name) => runtime.components.some((item) => item.name === name && item.status === 'ready')) ? 'ready' : 'pending')
const telegramState = computed(() => !connected.value ? 'unknown' : telegram.error ? 'error'
  : telegram.value?.status.state === 'backing_off' ? 'reconnecting'
  : telegram.value?.status.state ?? 'unknown')
const rows = computed(() => [
  { id: 'backend', icon: Server, label: t('summary.backend'), state: backend.value, help: '' },
  { id: 'runtime', icon: Cpu, label: t('shell.runtime'), state: runtimeState.value, help: t('shell.runtimeHelp') },
  ...(settings.network?.local_client && telegram.value?.settings.enabled
    ? [{ id: 'telegram', icon: Bot, label: t('summary.telegram'), state: telegramState.value, help: '' }] : []),
  ...(settings.network?.local_client && proxy.value?.mode === 'custom'
    ? [{ id: 'proxy', icon: Network, label: t('summary.proxy'), state: !connected.value ? 'unknown' : proxy.error ? 'error' : 'configured', help: t('summary.proxyHelp') }] : []),
])
function bulb(state: string) {
  if (['connected', 'ready'].includes(state)) return 'bg-emerald-400 shadow-[0_0_8px_#34d39955]'
  if (['offline', 'error'].includes(state)) return 'bg-rose-400'
  if (state === 'configured') return 'bg-sky-400'
  if (['connecting', 'reconnecting', 'pending'].includes(state)) return 'bg-amber-400'
  return 'bg-zinc-500'
}
onMounted(async () => {
  if (!settings.network) await settings.refresh()
  if (settings.network?.local_client) await Promise.all([telegram.refresh(), proxy.refresh()])
})
</script>

<template>
  <div class="runtime-summary space-y-3" role="status" aria-live="polite" :aria-label="t('summary.title')">
    <p class="text-[10px] font-semibold uppercase tracking-wider text-muted">{{ t('summary.title') }}</p>
    <div v-for="row in rows" :key="row.id" class="flex items-center gap-2 text-[11px]" :data-summary-row="row.id" :title="row.help || undefined">
      <component :is="row.icon" :size="14" class="shrink-0 text-muted" aria-hidden="true" />
      <span class="min-w-0 flex-1">{{ row.label }}</span>
      <span class="flex max-w-[55%] items-center gap-1.5 text-right text-muted">
        <span class="size-1.5 shrink-0 rounded-full" :class="bulb(row.state)" aria-hidden="true"></span>
        {{ t(`summary.${row.state}`) }}
      </span>
    </div>
  </div>
</template>
