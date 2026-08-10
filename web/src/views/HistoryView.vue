<script setup lang="ts">
import { onMounted } from 'vue'
import { History } from '@lucide/vue'
import { useLibraryStore } from '@/stores/library'
const library = useLibraryStore()
onMounted(() => library.refresh())
</script>

<template>
  <section>
    <p class="eyebrow">Activity</p><h2 class="mt-2 text-2xl font-semibold">History</h2><p class="mt-1 text-sm text-muted">Completed, failed, and stopped jobs persisted across restarts.</p>
    <div v-if="library.history.length" class="card mt-6 overflow-x-auto"><table class="history-table"><thead><tr><th>Media</th><th>Mode</th><th>Status</th><th>Updated</th></tr></thead><tbody><tr v-for="job in library.history" :key="job.id"><td><div class="max-w-md truncate font-medium">{{ job.title ?? job.url }}</div><div class="max-w-md truncate font-mono text-[10px] text-muted">{{ job.url }}</div></td><td>{{ job.mode }}</td><td><span class="badge" :class="{ muted: job.status !== 'completed' }">{{ job.status }}</span></td><td class="text-muted">{{ new Date(job.updated_at).toLocaleString() }}</td></tr></tbody></table></div>
    <div v-else class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><History :size="22" /></div><h3 class="mt-4 text-sm font-semibold">No history yet</h3></div>
  </section>
</template>
