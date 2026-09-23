<script setup lang="ts">
import { Activity, CircleCheck, LayoutGrid, TriangleAlert } from '@lucide/vue'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import DownloadProcessRow from '@/components/DownloadProcessRow.vue'
import { useDownloadsStore } from '@/stores/downloads'
import { useProcessesStore } from '@/stores/processes'
import type { ProcessingJob } from '@/app/api/client'
const { t } = useI18n()
const route = useRoute()
const downloads = useDownloadsStore()
const processes = useProcessesStore()
const statusFilter = ref('all')
const typeFilter = ref('all')
const statusFilters = ['all', 'completed', 'inProgress', 'error'] as const
const types = ['all', 'download', 'conversion', 'edit', 'metadata'] as const
const icons = { all: LayoutGrid, completed: CircleCheck, inProgress: Activity, error: TriangleAlert }
const now = ref(Date.now())
let ticker: ReturnType<typeof setInterval> | undefined
onMounted(() => { void processes.refresh(); ticker = setInterval(() => { now.value = Date.now() }, 1000) })
onBeforeUnmount(() => clearInterval(ticker))
const rows = computed(() => [
  ...downloads.jobs.map(download => ({ id: download.id, kind: 'download', state: download.status, created: download.created_at, download, process: null })),
  ...processes.jobs.map(process => ({ id: process.id, kind: process.kind, state: process.state, created: process.created_at, download: null, process })),
].sort((a, b) => b.created.localeCompare(a.created)))
const visible = computed(() => rows.value.filter(row => {
  if (typeFilter.value !== 'all' && row.kind !== typeFilter.value) return false
  if (statusFilter.value === 'completed') return row.state === 'completed'
  if (statusFilter.value === 'error') return ['failed', 'interrupted'].includes(row.state)
  if (statusFilter.value === 'inProgress') return ['created', 'analyzing', 'ready', 'queued', 'downloading', 'postprocessing', 'running'].includes(row.state)
  return true
}))
function elapsed(job: ProcessingJob) {
  if (!job.started_at) return null
  const seconds = Math.max(0, Math.floor(((job.finished_at ? Date.parse(job.finished_at) : now.value) - Date.parse(job.started_at)) / 1000))
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}
function action(job: ProcessingJob) {
  if (job.kind === 'metadata') return null
  if (['queued', 'running'].includes(job.state)) return 'cancel'
  if (['failed', 'cancelled', 'interrupted'].includes(job.state)) return 'retry'
  return null
}
async function act(job: ProcessingJob) { const name = action(job); if (name) await processes.act(job, name) }
watch([() => route.query.process, () => rows.value.length], async () => {
  if (typeof route.query.process !== 'string') return
  typeFilter.value = 'all'; statusFilter.value = 'all'
  await nextTick()
  const row = document.getElementById(`process-${route.query.process}`)
  row?.focus(); row?.scrollIntoView({ block: 'center' })
}, { immediate: true })
</script>

<template>
  <section>
    <div class="browse-header">
      <div class="browse-heading"><p class="eyebrow">{{ t('processing.title') }}</p><h2 class="mt-2 text-2xl font-semibold">{{ t('processing.title') }}</h2><p class="mt-1 text-sm text-muted">{{ t('processing.subtitle') }}</p></div>
      <div class="browse-filters" role="group" :aria-label="t('browse.statusFilter')">
        <button v-for="filter in statusFilters" :key="filter" class="browse-filter" :class="{ active: statusFilter === filter }" type="button" :aria-pressed="statusFilter === filter" @click="statusFilter = filter"><component :is="icons[filter]" :size="15" aria-hidden="true" />{{ t(`browse.${filter}`) }}</button>
      </div>
    </div>
    <div class="mt-4 flex flex-wrap gap-2" role="group" :aria-label="t('processing.typeFilter')"><button v-for="type in types" :key="type" type="button" class="browse-filter" :class="{ active: typeFilter === type }" :aria-pressed="typeFilter === type" @click="typeFilter = type">{{ t(`processing.types.${type}`) }}</button></div>
    <p v-if="downloads.error || processes.error" class="error-panel mt-5">{{ downloads.error || processes.error }}</p>
    <div v-if="visible.length" class="mt-6 space-y-3">
      <div v-for="row in visible" :id="`process-${row.id}`" :key="row.id" tabindex="-1" class="rounded-xl focus-visible:outline-2 focus-visible:outline-offset-4">
        <DownloadProcessRow v-if="row.download" :job="row.download" :now="now" />
        <article v-else-if="row.process" class="card p-4 sm:p-5">
          <div class="flex items-start gap-4"><div class="empty-icon"><Activity :size="19" /></div><div class="min-w-0 flex-1">
            <div class="flex items-start justify-between gap-3"><div class="min-w-0"><h3 class="truncate text-sm font-semibold">{{ row.process.title }}</h3><p class="mt-1 text-xs text-muted">{{ t(`processing.types.${row.kind}`) }}</p></div><span class="badge shrink-0" :class="{ danger: ['failed', 'interrupted'].includes(row.state), success: row.state === 'completed' }">{{ t(`processing.states.${row.state}`) }}</span></div>
            <div class="mt-4 rounded-lg border border-border bg-panel-2 p-3">
              <div class="flex flex-wrap items-center justify-between gap-2 text-[11px]"><span>{{ t(`processing.stages.${row.process.stage}`) }}</span><span v-if="row.process.progress_percent != null">{{ row.process.progress_percent.toFixed(1) }}%</span></div>
              <div v-if="row.state === 'running' || row.state === 'completed'" role="progressbar" :aria-label="row.process.title" :aria-valuenow="row.process.progress_percent ?? undefined" :aria-valuemin="0" :aria-valuemax="100" class="progress mt-2" :class="{ indeterminate: row.process.progress_percent == null }"><div :style="{ width: `${row.process.progress_percent ?? 0}%` }"></div></div>
              <p v-if="elapsed(row.process)" class="mt-2 text-[11px] text-muted">{{ t('processing.elapsed', { duration: elapsed(row.process) }) }}</p>
              <p v-if="row.process.eta_seconds != null" class="mt-1 text-[11px] text-muted">{{ t('common.remaining', { duration: `${Math.ceil(row.process.eta_seconds)}s` }) }}</p>
            </div>
            <p v-if="row.process.error" class="error-panel mt-3">{{ row.process.error }}</p>
          </div></div>
          <div class="mt-4 flex justify-end gap-2"><RouterLink v-if="row.process.output_file_id" class="secondary-btn" :to="{ path: '/completed', query: { file: row.process.output_file_id } }">{{ t('processing.viewFile') }}</RouterLink><button v-if="action(row.process)" class="secondary-btn" type="button" :disabled="!!processes.acting" @click="act(row.process)">{{ t(`processing.${action(row.process)}`) }}</button></div>
        </article>
      </div>
    </div>
    <div v-else-if="downloads.loading || processes.loading" class="card mt-6 animate-pulse p-8"><div class="h-24 rounded-xl bg-zinc-800/60"></div></div>
    <div v-else class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><Activity :size="22" /></div><h3 class="mt-4 text-sm font-semibold">{{ t(rows.length ? 'browse.noMatches' : 'processing.noJobs') }}</h3><p class="mt-2 text-xs text-muted">{{ t(rows.length ? 'browse.resetHelp' : 'processing.noJobsHelp') }}</p></div>
  </section>
</template>
