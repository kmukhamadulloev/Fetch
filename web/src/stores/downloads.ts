import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { createDownload, downloadAction, getDownloads, type DownloadJob, type DownloadRequest } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useDownloadsStore = defineStore('downloads', () => {
  const jobs = ref<DownloadJob[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const active = computed(() => jobs.value.filter((job) => ['queued', 'downloading', 'postprocessing'].includes(job.status)))

  let inFlight: Promise<void> | null = null
  let revision = 0
  const versions = new Map<string, number>()
  function merge(job: DownloadJob) {
    versions.set(job.id, ++revision)
    const index = jobs.value.findIndex((item) => item.id === job.id)
    if (index === -1) jobs.value.unshift(job)
    else jobs.value[index] = job
  }

  function refresh() {
    if (inFlight) return inFlight
    const started = revision
    loading.value = jobs.value.length === 0
    inFlight = (async () => {
      try {
        const snapshot = await getDownloads()
        const current = new Map(jobs.value.map(job => [job.id, job]))
        jobs.value = snapshot.map(job => (versions.get(job.id) ?? 0) > started ? current.get(job.id)! : job)
        for (const [id, job] of current) if ((versions.get(id) ?? 0) > started && !jobs.value.some(item => item.id === id)) jobs.value.unshift(job)
        error.value = null
      } catch (cause) { error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.downloadsUnavailable') }
      finally { loading.value = false; inFlight = null }
    })()
    return inFlight
  }

  async function add(request: DownloadRequest) {
    try {
      const job = await createDownload(request)
      merge(job)
      error.value = null
      return job
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.downloadAdd')
      throw cause
    }
  }

  async function act(job: DownloadJob, action: 'stop' | 'resume' | 'retry') {
    try { merge(await downloadAction(job.id, action)); error.value = null }
    catch (cause) { error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.downloadAction') }
  }

  function applyEvent(payload: unknown) { merge(payload as DownloadJob) }

  return { jobs, active, loading, error, refresh, add, act, applyEvent }
})
