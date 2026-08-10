<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { CheckCircle2, RefreshCw, Trash2, TriangleAlert } from '@lucide/vue'
import { clearLogs, getDiagnostics, getLogs, type DiagnosticLogEntry, type DiagnosticsReport } from '@/app/api/client'

const logs = ref<DiagnosticLogEntry[]>([])
const report = ref<DiagnosticsReport | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)
async function refresh() {
  loading.value = true
  try { [logs.value, report.value] = await Promise.all([getLogs(), getDiagnostics()]); error.value = null }
  catch (cause) { error.value = cause instanceof Error ? cause.message : 'Diagnostics are unavailable' }
  finally { loading.value = false }
}
async function clear() { await clearLogs(); logs.value = [] }
onMounted(refresh)
</script>

<template>
  <section>
    <div class="flex items-end justify-between gap-4"><div><p class="eyebrow">Diagnostics</p><h2 class="mt-2 text-2xl font-semibold">Logs</h2><p class="mt-1 text-sm text-muted">Retained yt-dlp, FFmpeg, and application diagnostics.</p></div><div class="flex gap-2"><button class="secondary-btn" type="button" :disabled="loading" @click="refresh"><RefreshCw :size="14" />Refresh</button><button class="secondary-btn" type="button" @click="clear"><Trash2 :size="14" />Clear</button></div></div>
    <p v-if="error" class="error-panel mt-5">{{ error }}</p>
    <div v-if="report" class="mt-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4"><div v-for="check in report.checks" :key="check.name" class="runtime-card"><div class="flex items-center gap-2"><CheckCircle2 v-if="check.healthy" class="text-emerald-300" :size="17" /><TriangleAlert v-else class="text-rose-300" :size="17" /><span class="text-xs font-semibold">{{ check.name }}</span></div><p class="mt-2 break-words text-[11px] leading-5 text-muted">{{ check.message }}</p></div></div>
    <div class="card mt-5 overflow-hidden"><div class="border-b border-border bg-black/15 px-4 py-3 text-xs">Retained process output · {{ logs.length }} entries</div><div class="min-h-96 max-h-[600px] overflow-auto bg-[#090b0e] p-4 font-mono text-[11px] leading-6"><div v-for="entry in logs" :key="entry.id" class="grid grid-cols-[150px_70px_90px_1fr] gap-3 border-b border-white/[.025] py-1"><span class="text-zinc-600">{{ new Date(entry.created_at).toLocaleString() }}</span><span :class="entry.level === 'error' ? 'text-rose-300' : entry.level === 'warn' ? 'text-amber-300' : 'text-zinc-500'">{{ entry.level }}</span><span class="text-accent">{{ entry.subsystem }}</span><span class="whitespace-pre-wrap text-zinc-300">{{ entry.message }}<span v-if="entry.details" class="block text-zinc-600">{{ entry.details }}</span></span></div><p v-if="!logs.length" class="text-zinc-600">No retained diagnostic output.</p></div></div>
  </section>
</template>
