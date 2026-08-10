<script setup lang="ts">
import { Activity, Clock3, Gauge, HardDriveDownload, RotateCcw, Square, TimerReset } from '@lucide/vue'
import { useDownloadsStore } from '@/stores/downloads'
import type { DownloadJob, DownloadStatus } from '@/app/api/client'

const downloads = useDownloadsStore()

const statusCopy: Record<DownloadStatus, { label: string; detail: string }> = {
  created: { label: 'Created', detail: 'Preparing the job' },
  analyzing: { label: 'Analyzing', detail: 'Reading media information' },
  ready: { label: 'Ready', detail: 'Ready to enter the queue' },
  queued: { label: 'Queued', detail: 'Waiting for an available download slot' },
  downloading: { label: 'Downloading', detail: 'Receiving media with yt-dlp' },
  postprocessing: { label: 'Finalizing', detail: 'Merging and processing with FFmpeg' },
  completed: { label: 'Completed', detail: 'Saved successfully to disk' },
  failed: { label: 'Failed', detail: 'The job needs attention before retrying' },
  stopped: { label: 'Stopped', detail: 'Paused by the user and ready to resume' },
}

function bytes(value: number | null) {
  if (value == null) return 'Unknown size'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = value, unit = 0
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++ }
  return `${size.toFixed(unit ? 1 : 0)} ${units[unit]}`
}
function duration(value: number | null | undefined) {
  if (value == null) return 'Unknown duration'
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
    <p class="eyebrow">Queue</p><h2 class="mt-2 text-2xl font-semibold">Downloads</h2><p class="mt-1 text-sm text-muted">Live transfer details, media length, output size, and every processing stage.</p>
    <p v-if="downloads.error" class="error-panel mt-5">{{ downloads.error }}</p>
    <div v-if="downloads.loading" class="card mt-6 animate-pulse p-8"><div class="h-24 rounded-xl bg-zinc-800/60"></div></div>
    <div v-else-if="downloads.jobs.length" class="mt-6 space-y-3">
      <article v-for="job in downloads.jobs" :key="job.id" class="card p-4 sm:p-5">
        <div class="flex items-start gap-4">
          <div class="empty-icon"><Activity :size="19" /></div>
          <div class="min-w-0 flex-1">
            <div class="flex items-start justify-between gap-3"><div class="min-w-0"><h3 class="truncate text-sm font-semibold">{{ job.title ?? job.url }}</h3><p class="mt-1 truncate font-mono text-[10px] text-muted">{{ job.url }}</p></div><span class="badge shrink-0" :class="statusClass(job.status)">{{ statusCopy[job.status].label }}</span></div>

            <div class="mt-3 flex flex-wrap gap-x-4 gap-y-2 text-[11px] text-muted">
              <span class="helper"><Clock3 :size="13" />{{ duration(job.duration_seconds) }}</span>
              <span class="helper"><HardDriveDownload :size="13" />{{ bytes(job.total_bytes) }}</span>
              <span class="helper capitalize"><Gauge :size="13" />{{ job.mode }}</span>
            </div>

            <div class="mt-4 rounded-lg border border-border bg-panel-2 p-3">
              <div class="flex flex-wrap items-center justify-between gap-2 text-[11px]"><span class="font-medium">{{ statusCopy[job.status].detail }}</span><span v-if="showProgress(job)" class="font-semibold">{{ progress(job).toFixed(1) }}%</span></div>
              <div v-if="showProgress(job)" class="progress mt-2" :class="{ indeterminate: job.status === 'postprocessing' && job.progress_percent == null }"><div :style="{ width: `${progress(job)}%` }"></div></div>
              <div v-if="job.status === 'downloading' || job.status === 'postprocessing'" class="mt-2 flex flex-wrap justify-between gap-2 text-[11px] text-muted">
                <span>{{ bytes(job.downloaded_bytes) }}<template v-if="job.total_bytes != null"> / {{ bytes(job.total_bytes) }}</template></span>
                <span><template v-if="job.speed_bytes_per_second != null">{{ bytes(job.speed_bytes_per_second) }}/s</template><template v-if="job.speed_bytes_per_second != null && job.eta_seconds != null"> · </template><template v-if="job.eta_seconds != null">{{ duration(job.eta_seconds) }} remaining</template></span>
              </div>
            </div>

            <p v-if="job.error_message" class="error-panel mt-3">{{ job.error_message }}</p>
          </div>
        </div>
        <div v-if="['queued','downloading','postprocessing','failed','stopped'].includes(job.status)" class="mt-4 flex justify-end"><button class="secondary-btn" type="button" @click="downloads.act(job, action(job) as 'stop' | 'resume' | 'retry')"><Square v-if="action(job) === 'stop'" :size="14" /><RotateCcw v-else-if="action(job) === 'retry'" :size="14" /><TimerReset v-else :size="14" />{{ action(job) }}</button></div>
      </article>
    </div>
    <div v-else class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><Activity :size="22" /></div><h3 class="mt-4 text-sm font-semibold">No downloads yet</h3><p class="mt-2 text-xs text-muted">Analyze a URL and add a real yt-dlp job.</p></div>
  </section>
</template>
