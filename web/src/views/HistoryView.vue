<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { History } from '@lucide/vue'
import { useLibraryStore } from '@/stores/library'
import { useProcessesStore } from '@/stores/processes'
import { formatDateTime } from '@/i18n/format'
const library = useLibraryStore()
const processes = useProcessesStore()
const { t, locale } = useI18n()
const typeFilter = ref('all')
const types = ['all', 'download', 'conversion', 'edit', 'metadata'] as const
const statusKey = (status: string) => status === 'postprocessing' ? 'finalizing' : status
const rows = computed(() => [
  ...library.history.filter(job => ['completed', 'failed', 'stopped'].includes(job.status)).map(job => ({
    id: `download-${job.id}`, title: job.title ?? job.url, detail: job.url,
    kind: 'download', mode: job.mode, state: job.status,
    status: `downloadsView.status.${statusKey(job.status)}`, updated: job.updated_at,
  })),
  ...processes.jobs.filter(job => ['completed', 'failed', 'cancelled', 'interrupted'].includes(job.state)).map(job => ({
    id: `process-${job.id}`, title: job.title, detail: null,
    kind: job.kind, mode: null, state: job.state,
    status: `processing.states.${job.state}`, updated: job.finished_at ?? job.updated_at,
  })),
].sort((a, b) => Date.parse(b.updated) - Date.parse(a.updated) || a.id.localeCompare(b.id)))
const visible = computed(() => rows.value.filter(row => typeFilter.value === 'all' || row.kind === typeFilter.value))
onMounted(() => { void library.refresh(); void processes.refresh() })
</script>

<template>
  <section>
    <div class="browse-header">
      <div class="browse-heading"><p class="eyebrow">{{ t('historyView.eyebrow') }}</p><h2 class="mt-2 text-2xl font-semibold">{{ t('historyView.title') }}</h2><p class="mt-1 text-sm text-muted">{{ t('historyView.description') }}</p></div>
      <div class="browse-controls">
        <select v-model="typeFilter" class="select process-type-select" :aria-label="t('processing.typeFilter')">
          <option v-for="type in types" :key="type" :value="type">{{ t(`processing.types.${type}`) }}</option>
        </select>
      </div>
    </div>
    <p v-if="library.error || processes.error" class="error-panel mt-5" role="alert">{{ library.error || processes.error }}</p>
    <div v-if="visible.length" class="card mt-6 overflow-x-auto"><table class="history-table"><thead><tr><th>{{ t('historyView.media') }}</th><th>{{ t('historyView.operation') }}</th><th>{{ t('historyView.status') }}</th><th>{{ t('historyView.updated') }}</th></tr></thead><tbody><tr v-for="job in visible" :key="job.id"><td><div class="max-w-md truncate font-medium">{{ job.title }}</div><div v-if="job.detail" class="max-w-md truncate font-mono text-[10px] text-muted">{{ job.detail }}</div></td><td>{{ t(`processing.types.${job.kind}`) }}<span v-if="job.mode" class="block text-xs text-muted">{{ t(`common.${job.mode}`) }}</span></td><td><span class="badge" :class="{ muted: job.state !== 'completed' }">{{ t(job.status) }}</span></td><td class="text-muted">{{ formatDateTime(job.updated, locale) }}</td></tr></tbody></table></div>
    <div v-else-if="library.loading || processes.loading" class="card mt-6 animate-pulse p-8" aria-busy="true"><div class="h-24 rounded-xl bg-zinc-800/60"></div></div>
    <div v-else-if="!library.error && !processes.error" class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><History :size="22" /></div><h3 class="mt-4 text-sm font-semibold">{{ t(rows.length ? 'browse.noMatches' : 'historyView.noHistory') }}</h3><button v-if="rows.length" type="button" class="secondary-btn mt-4" @click="typeFilter = 'all'">{{ t('processing.types.all') }}</button></div>
  </section>
</template>
