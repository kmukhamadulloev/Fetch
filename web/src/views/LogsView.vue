<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { CheckCircle2, ChevronDown, RefreshCw, Trash2, TriangleAlert } from '@lucide/vue'
import { clearLogs, getDiagnostics, getLogs, type DiagnosticLogEntry, type DiagnosticsReport } from '@/app/api/client'
import { countDiagnosticLogs, filterDiagnosticLogs, logLevelFilters, normalizedLogLevel, type LogLevelFilter } from '@/app/logs'
import { formatDateTime } from '@/i18n/format'

const logs = ref<DiagnosticLogEntry[]>([])
const report = ref<DiagnosticsReport | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)
const levelFilter = ref<LogLevelFilter>('all')
const expandedLogs = ref(new Set<number>())
const { t, locale } = useI18n()
const visibleLogs = computed(() => filterDiagnosticLogs(logs.value, levelFilter.value))
const levelCount = (filter: LogLevelFilter) => countDiagnosticLogs(logs.value, filter)
const levelClass = (level: string) => ({
  error: 'text-rose-300',
  warn: 'text-amber-300',
  info: 'text-sky-300',
}[normalizedLogLevel(level)])
const isExpanded = (id: number) => expandedLogs.value.has(id)
const filterLabel = (filter: LogLevelFilter) => t(`logsView.${filter === 'warn' ? 'warnings' : filter === 'error' ? 'errors' : filter}`)
function toggleDetails(id: number) {
  const next = new Set(expandedLogs.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  expandedLogs.value = next
}
async function refresh() {
  loading.value = true
  try { [logs.value, report.value] = await Promise.all([getLogs(), getDiagnostics()]); error.value = null }
  catch (cause) { error.value = cause instanceof Error ? cause.message : t('logsView.unavailable') }
  finally { loading.value = false }
}
async function clear() { await clearLogs(); logs.value = [] }
onMounted(refresh)
</script>

<template>
  <section>
    <div class="flex items-end justify-between gap-4"><div><p class="eyebrow">{{ t('logsView.eyebrow') }}</p><h2 class="mt-2 text-2xl font-semibold">{{ t('logsView.title') }}</h2><p class="mt-1 text-sm text-muted">{{ t('logsView.description') }}</p></div><div class="flex gap-2"><button class="secondary-btn" type="button" :disabled="loading" @click="refresh"><RefreshCw :size="14" />{{ t('logsView.refresh') }}</button><button class="secondary-btn" type="button" @click="clear"><Trash2 :size="14" />{{ t('logsView.clear') }}</button></div></div>
    <p v-if="error" class="error-panel mt-5">{{ error }}</p>
    <div v-if="report" class="mt-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4"><div v-for="check in report.checks" :key="check.name" class="runtime-card"><div class="flex items-center gap-2"><CheckCircle2 v-if="check.healthy" class="text-emerald-300" :size="17" /><TriangleAlert v-else class="text-rose-300" :size="17" /><span class="text-xs font-semibold">{{ check.name }}</span></div><p class="mt-2 break-words text-[11px] leading-5 text-muted">{{ check.message }}</p></div></div>
    <div class="card mt-5 overflow-hidden">
      <div class="border-b border-border bg-black/15 px-4 py-3">
        <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div class="text-xs">{{ t('logsView.retained', { visible: visibleLogs.length, total: logs.length }) }}</div>
          <div class="flex max-w-full gap-1 overflow-x-auto pb-0.5" :aria-label="t('logsView.filters')">
            <button v-for="filter in logLevelFilters" :key="filter.value" class="log-filter" :class="{ active: levelFilter === filter.value }" type="button" :aria-pressed="levelFilter === filter.value" @click="levelFilter = filter.value">
              {{ filterLabel(filter.value) }} <span>{{ levelCount(filter.value) }}</span>
            </button>
          </div>
        </div>
      </div>
      <div class="min-h-96 max-h-[600px] overflow-auto bg-[#090b0e] p-3 font-mono text-[11px] leading-5 sm:p-4 sm:leading-6">
        <article v-for="entry in visibleLogs" :key="entry.id" class="log-entry" data-log-entry>
          <div class="log-entry-summary">
            <time class="text-zinc-500" :datetime="entry.created_at">{{ formatDateTime(entry.created_at, locale) }}</time>
            <span class="log-level" :class="levelClass(entry.level)">{{ normalizedLogLevel(entry.level) }}</span>
            <span class="log-subsystem">{{ entry.subsystem }}</span>
            <div class="log-message">{{ entry.message }}</div>
            <button v-if="entry.details" class="log-details-button" type="button" :aria-expanded="isExpanded(entry.id)" :aria-controls="`log-details-${entry.id}`" @click="toggleDetails(entry.id)">
              {{ t('common.details') }} <ChevronDown :class="{ 'rotate-180': isExpanded(entry.id) }" :size="14" />
            </button>
          </div>
          <pre v-if="entry.details && isExpanded(entry.id)" :id="`log-details-${entry.id}`" class="log-details">{{ entry.details }}</pre>
        </article>
        <p v-if="!logs.length" class="p-2 text-zinc-500">{{ t('logsView.noOutput') }}</p>
        <p v-else-if="!visibleLogs.length" class="p-2 text-zinc-500">{{ t('logsView.noFiltered', { level: filterLabel(levelFilter) }) }}</p>
      </div>
    </div>
  </section>
</template>
