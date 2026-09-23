import { ref } from 'vue'
import { defineStore } from 'pinia'
import { getProcesses, processAction, type ProcessingJob } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useProcessesStore = defineStore('processes', () => {
  const jobs = ref<ProcessingJob[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const acting = ref<string | null>(null)
  let inFlight: Promise<void> | null = null
  let revision = 0
  const versions = new Map<string, number>()
  function applyEvent(payload: unknown) {
    const job = payload as ProcessingJob
    const index = jobs.value.findIndex(item => item.id === job.id)
    const previous = jobs.value[index]
    const rank = (state: ProcessingJob['state']) => state === 'queued' ? 0 : state === 'running' ? 1 : 2
    if (previous && (Date.parse(previous.updated_at) > Date.parse(job.updated_at)
      || (Date.parse(previous.updated_at) === Date.parse(job.updated_at) && rank(previous.state) > rank(job.state)))) return
    versions.set(job.id, ++revision)
    if (index < 0) jobs.value.unshift(job)
    else jobs.value[index] = job
  }
  function refresh() {
    if (inFlight) return inFlight
    const started = revision
    loading.value = jobs.value.length === 0
    inFlight = (async () => {
      try {
        const snapshot = await getProcesses()
        for (const job of snapshot) if ((versions.get(job.id) ?? 0) <= started) applyEvent(job)
        error.value = null
      } catch (cause) { error.value = cause instanceof Error ? cause.message : i18n.global.t('processing.unavailable') }
      finally { loading.value = false; inFlight = null }
    })()
    return inFlight
  }
  async function act(job: ProcessingJob, action: 'cancel' | 'retry') {
    if (acting.value) return
    acting.value = job.id
    try { const result = await processAction(job.id, action); applyEvent(result); error.value = null; return result }
    catch (cause) { error.value = cause instanceof Error ? cause.message : i18n.global.t('processing.actionFailed') }
    finally { acting.value = null }
  }
  return { jobs, loading, error, acting, refresh, applyEvent, act }
})
