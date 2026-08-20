<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { History } from '@lucide/vue'
import { useLibraryStore } from '@/stores/library'
import { formatDateTime } from '@/i18n/format'
const library = useLibraryStore()
const { t, locale } = useI18n()
const statusKey = (status: string) => status === 'postprocessing' ? 'finalizing' : status
onMounted(() => library.refresh())
</script>

<template>
  <section>
    <p class="eyebrow">{{ t('historyView.eyebrow') }}</p><h2 class="mt-2 text-2xl font-semibold">{{ t('historyView.title') }}</h2><p class="mt-1 text-sm text-muted">{{ t('historyView.description') }}</p>
    <div v-if="library.history.length" class="card mt-6 overflow-x-auto"><table class="history-table"><thead><tr><th>{{ t('historyView.media') }}</th><th>{{ t('historyView.mode') }}</th><th>{{ t('historyView.status') }}</th><th>{{ t('historyView.updated') }}</th></tr></thead><tbody><tr v-for="job in library.history" :key="job.id"><td><div class="max-w-md truncate font-medium">{{ job.title ?? job.url }}</div><div class="max-w-md truncate font-mono text-[10px] text-muted">{{ job.url }}</div></td><td>{{ t(`common.${job.mode}`) }}</td><td><span class="badge" :class="{ muted: job.status !== 'completed' }">{{ t(`downloadsView.status.${statusKey(job.status)}`) }}</span></td><td class="text-muted">{{ formatDateTime(job.updated_at, locale) }}</td></tr></tbody></table></div>
    <div v-else class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><History :size="22" /></div><h3 class="mt-4 text-sm font-semibold">{{ t('historyView.noHistory') }}</h3></div>
  </section>
</template>
