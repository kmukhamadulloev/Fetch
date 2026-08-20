<script setup lang="ts">
import { Activity, Clock3, Gauge, HardDriveDownload, RotateCcw, Square, TimerReset } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import { useDownloadsStore } from '@/stores/downloads'
import type { DownloadJob, DownloadStatus } from '@/app/api/client'
import { formatBytes } from '@/i18n/format'

const downloads = useDownloadsStore()
const { t, locale } = useI18n()

const statusCopy: Record<DownloadStatus, { label: string; detail: string }> = {
  created: { label: 'downloadsView.status.created', detail: 'downloadsView.status.createdDetail' },
  analyzing: { label: 'downloadsView.status.analyzing', detail: 'downloadsView.status.analyzingDetail' },
  ready: { label: 'downloadsView.status.ready', detail: 'downloadsView.status.readyDetail' },
  queued: { label: 'downloadsView.status.queued', detail: 'downloadsView.status.queuedDetail' },
  downloading: { label: 'downloadsView.status.downloading', detail: 'downloadsView.status.downloadingDetail' },
  postprocessing: { label: 'downloadsView.status.finalizing', detail: 'downloadsView.status.finalizingDetail' },
  completed: { label: 'downloadsView.status.completed', detail: 'downloadsView.status.completedDetail' },
  failed: { label: 'downloadsView.status.failed', detail: 'downloadsView.status.failedDetail' },
  stopped: { label: 'downloadsView.status.stopped', detail: 'downloadsView.status.stoppedDetail' },
}

function bytes(value: number | null) {
  return value == null ? t('common.unknownSize') : formatBytes(value, locale.value)
}
function duration(value: number | null | undefined) {
  if (value == null) return t('common.unknownDuration')
  const total = Math.max(0, Math.round(value))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = total % 60
  return hours ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}` : `${minutes}:${String(seconds).padStart(2, '0')}`
}
function action(job: DownloadJob) {
  if (job.status === 'failed') return 'retry'
  if (job.status === 'stopped') return 'resume'
  return 'stop'
}
function actionLabel(job: DownloadJob) { return t(`downloadsView.actions.${action(job)}`) }
function statusClass(status: DownloadStatus) {
  if (status === 'failed') return 'danger'
  if (status === 'completed') return 'success'
  if (status === 'downloading' || status === 'postprocessing') return ''
  return 'muted'
}
function progress(job: DownloadJob) {
  if (job.status === 'completed') return 100
  return Math.max(0, Math.min(100, job.progress_percent ?? 0))
}
function showProgress(job: DownloadJob) {
  return ['downloading', 'postprocessing', 'completed', 'stopped'].includes(job.status)
}
</script>

<template>
  <section>
    <p class="eyebrow">{{ t('downloadsView.eyebrow') }}</p><h2 class="mt-2 text-2xl font-semibold">{{ t('downloadsView.title') }}</h2><p class="mt-1 text-sm text-muted">{{ t('downloadsView.description') }}</p>
    <p v-if="downloads.error" class="error-panel mt-5">{{ downloads.error }}</p>
    <div v-if="downloads.loading" class="card mt-6 animate-pulse p-8"><div class="h-24 rounded-xl bg-zinc-800/60"></div></div>
    <div v-else-if="downloads.jobs.length" class="mt-6 space-y-3">
      <article v-for="job in downloads.jobs" :key="job.id" class="card p-4 sm:p-5">
        <div class="flex items-start gap-4">
          <div class="empty-icon"><Activity :size="19" /></div>
          <div class="min-w-0 flex-1">
            <div class="flex items-start justify-between gap-3"><div class="min-w-0"><h3 class="truncate text-sm font-semibold">{{ job.title ?? job.url }}</h3><p class="mt-1 truncate font-mono text-[10px] text-muted">{{ job.url }}</p></div><span class="badge shrink-0" :class="statusClass(job.status)">{{ t(statusCopy[job.status].label) }}</span></div>

            <div class="mt-3 flex flex-wrap gap-x-4 gap-y-2 text-[11px] text-muted">
              <span class="helper"><Clock3 :size="13" />{{ duration(job.duration_seconds) }}</span>
              <span class="helper"><HardDriveDownload :size="13" />{{ bytes(job.total_bytes) }}</span>
              <span class="helper"><Gauge :size="13" />{{ t(`common.${job.mode}`) }}</span>
            </div>

            <div class="mt-4 rounded-lg border border-border bg-panel-2 p-3">
              <div class="flex flex-wrap items-center justify-between gap-2 text-[11px]"><span class="font-medium">{{ t(statusCopy[job.status].detail) }}</span><span v-if="showProgress(job)" class="font-semibold">{{ progress(job).toFixed(1) }}%</span></div>
              <div v-if="showProgress(job)" class="progress mt-2" :class="{ indeterminate: job.status === 'postprocessing' && job.progress_percent == null }"><div :style="{ width: `${progress(job)}%` }"></div></div>
              <div v-if="job.status === 'downloading' || job.status === 'postprocessing'" class="mt-2 flex flex-wrap justify-between gap-2 text-[11px] text-muted">
                <span>{{ bytes(job.downloaded_bytes) }}<template v-if="job.total_bytes != null"> / {{ bytes(job.total_bytes) }}</template></span>
                <span><template v-if="job.speed_bytes_per_second != null">{{ bytes(job.speed_bytes_per_second) }}/s</template><template v-if="job.speed_bytes_per_second != null && job.eta_seconds != null"> · </template><template v-if="job.eta_seconds != null">{{ t('common.remaining', { duration: duration(job.eta_seconds) }) }}</template></span>
              </div>
            </div>

            <p v-if="job.error_message" class="error-panel mt-3">{{ job.error_message }}</p>
          </div>
        </div>
        <div v-if="['queued','downloading','postprocessing','failed','stopped'].includes(job.status)" class="mt-4 flex justify-end"><button class="secondary-btn" type="button" @click="downloads.act(job, action(job) as 'stop' | 'resume' | 'retry')"><Square v-if="action(job) === 'stop'" :size="14" /><RotateCcw v-else-if="action(job) === 'retry'" :size="14" /><TimerReset v-else :size="14" />{{ actionLabel(job) }}</button></div>
      </article>
    </div>
    <div v-else class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><Activity :size="22" /></div><h3 class="mt-4 text-sm font-semibold">{{ t('downloadsView.noTitle') }}</h3><p class="mt-2 text-xs text-muted">{{ t('downloadsView.noDescription') }}</p></div>
  </section>
</template>
