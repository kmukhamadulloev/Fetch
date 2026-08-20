import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { createDownload, downloadAction, getDownloads, type DownloadJob, type DownloadRequest } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useDownloadsStore = defineStore('downloads', () => {
  const jobs = ref<DownloadJob[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const active = computed(() => jobs.value.filter((job) => ['queued', 'downloading', 'postprocessing'].includes(job.status)))

  function merge(job: DownloadJob) {
    const index = jobs.value.findIndex((item) => item.id === job.id)
    if (index === -1) jobs.value.unshift(job)
    else jobs.value[index] = job
  }

  async function refresh() {
    loading.value = true
    try { jobs.value = await getDownloads(); error.value = null }
    catch (cause) { error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.downloadsUnavailable') }
    finally { loading.value = false }
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
