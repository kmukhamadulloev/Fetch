import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { clearPlaybackProgress, deleteCompleted, getCompleted, getHistory, revealCompleted, savePlaybackProgress, type CompletedFile, type DownloadJob, type PlaybackProgress } from '@/app/api/client'

export interface CompletedPlaylistGroup {
  id: string
  title: string
  files: CompletedFile[]
  size_bytes: number
  latest_created_at: string
  watched_files: number
  progress_percent: number
}

export const useLibraryStore = defineStore('library', () => {
  const completed = ref<CompletedFile[]>([])
  const history = ref<DownloadJob[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const actingId = ref<string | null>(null)
  const playbackError = ref<string | null>(null)
  const standalone = computed(() => completed.value.filter((file) => !file.playlist))
  const playlists = computed<CompletedPlaylistGroup[]>(() => {
    const groups = new Map<string, CompletedPlaylistGroup>()
    for (const file of completed.value) {
      if (!file.playlist) continue
      const group = groups.get(file.playlist.id) ?? {
        id: file.playlist.id,
        title: file.playlist.title,
        files: [],
        size_bytes: 0,
        latest_created_at: file.created_at,
        watched_files: 0,
        progress_percent: 0,
      }
      group.files.push(file)
      group.size_bytes += file.size_bytes
      if (file.playback?.completed) group.watched_files++
      if (file.playback) group.progress_percent += file.playback.completed ? 100 : Math.min(100, file.playback.position_seconds / file.playback.duration_seconds * 100)
      if (file.created_at > group.latest_created_at) group.latest_created_at = file.created_at
      groups.set(group.id, group)
    }
    return [...groups.values()]
      .map((group) => ({
        ...group,
        progress_percent: group.files.length ? group.progress_percent / group.files.length : 0,
        files: group.files.sort((left, right) =>
          (left.playlist?.index ?? 0) - (right.playlist?.index ?? 0)
          || left.created_at.localeCompare(right.created_at)),
      }))
      .sort((left, right) => right.latest_created_at.localeCompare(left.latest_created_at))
  })

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

  function applyPlayback(payload: unknown) {
    const progress = payload as PlaybackProgress
    const file = completed.value.find((item) => item.id === progress.file_id)
    if (file) file.playback = progress
  }

  function clearPlayback(payload: unknown) {
    const id = payload as string
    const file = completed.value.find((item) => item.id === id)
    if (file) file.playback = null
  }

  async function saveProgress(file: CompletedFile, positionSeconds: number, durationSeconds: number) {
    try {
      applyPlayback(await savePlaybackProgress(file.id, positionSeconds, durationSeconds))
      playbackError.value = null
    } catch (cause) {
      playbackError.value = cause instanceof Error ? cause.message : 'Watch progress could not be saved'
    }
  }

  async function resetProgress(file: CompletedFile) {
    try {
      await clearPlaybackProgress(file.id)
      clearPlayback(file.id)
      playbackError.value = null
      return true
    } catch (cause) {
      playbackError.value = cause instanceof Error ? cause.message : 'Watch progress could not be reset'
      return false
    }
  }

  return { completed, standalone, playlists, history, loading, error, actingId, playbackError, refresh, reveal, remove, applyCompleted, applyDownload, applyPlayback, clearPlayback, saveProgress, resetProgress }
})
