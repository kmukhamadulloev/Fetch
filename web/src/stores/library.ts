import { ref } from 'vue'
import { defineStore } from 'pinia'
import { deleteCompleted, getCompleted, getHistory, revealCompleted, type CompletedFile, type DownloadJob } from '@/app/api/client'

export const useLibraryStore = defineStore('library', () => {
  const completed = ref<CompletedFile[]>([])
  const history = ref<DownloadJob[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const actingId = ref<string | null>(null)

  async function refresh() {
    loading.value = true
    try {
      const [files, jobs] = await Promise.all([getCompleted(), getHistory()])
      completed.value = files
      history.value = jobs
      error.value = null
    } catch (cause) { error.value = cause instanceof Error ? cause.message : 'Library is unavailable' }
    finally { loading.value = false }
  }

  async function reveal(file: CompletedFile) {
    actingId.value = file.id
    try { await revealCompleted(file.id); error.value = null }
    catch (cause) { error.value = cause instanceof Error ? cause.message : 'Could not open the containing folder' }
    finally { actingId.value = null }
  }

  async function remove(file: CompletedFile) {
    actingId.value = file.id
    try {
      await deleteCompleted(file.id)
      completed.value = completed.value.filter((item) => item.id !== file.id)
      error.value = null
      return true
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Could not delete the completed file'
      return false
    } finally { actingId.value = null }
  }

  function applyCompleted(payload: unknown) {
    const file = payload as CompletedFile
    const index = completed.value.findIndex((item) => item.id === file.id)
    if (index === -1) completed.value.unshift(file)
    else completed.value[index] = file
  }

  function applyDownload(payload: unknown) {
    const job = payload as DownloadJob
    const index = history.value.findIndex((item) => item.id === job.id)
    if (index === -1) history.value.unshift(job)
    else history.value[index] = job
  }

  return { completed, history, loading, error, actingId, refresh, reveal, remove, applyCompleted, applyDownload }
})
