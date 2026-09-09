<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, LayoutGrid, CircleCheck, FileVideo, ListVideo, LoaderCircle, Music2, Trash2, TriangleAlert, X } from '@lucide/vue'
import SortDropdown from '@/components/SortDropdown.vue'
import CompletedMediaCard from '@/components/CompletedMediaCard.vue'
import MediaPlayerDialog from '@/components/MediaPlayerDialog.vue'
import { useLibraryStore, type CompletedPlaylistGroup } from '@/stores/library'
import { useSettingsStore } from '@/stores/settings'
import type { CompletedFile } from '@/app/api/client'
import { formatBytes } from '@/i18n/format'

type LibraryEntry =
  | { kind: 'file'; file: CompletedFile; updated_at: string }
  | { kind: 'playlist'; playlist: CompletedPlaylistGroup; updated_at: string }

const library = useLibraryStore()
const settings = useSettingsStore()
const route = useRoute()
const router = useRouter()
const { t, locale } = useI18n()
const mediaFilter = ref('all')
const sortBy = ref('date')
const reverse = ref(false)
const filters = ['all', 'audio', 'video', 'playlist'] as const
const filterIcons = { all: LayoutGrid, audio: Music2, video: FileVideo, playlist: ListVideo }
const selected = ref<CompletedFile | null>(null)
const pendingDelete = ref<CompletedFile | null>(null)
const failedPlaylistThumbnails = ref(new Set<string>())

const requestedPlaylistId = computed(() => typeof route.query.playlist === 'string' ? route.query.playlist : null)
const activePlaylist = computed(() => requestedPlaylistId.value ? library.playlists.find((playlist) => playlist.id === requestedPlaylistId.value) ?? null : null)
const libraryEntries = computed<LibraryEntry[]>(() => {
  const files = mediaFilter.value === 'audio' || mediaFilter.value === 'video'
    ? library.completed.filter((file) => file.mime_type.startsWith(`${mediaFilter.value}/`))
    : library.standalone
  const entries: LibraryEntry[] = [
    ...(mediaFilter.value === 'playlist' ? [] : files.map((file) => ({ kind: 'file' as const, file, updated_at: file.created_at }))),
    ...(['all', 'playlist'].includes(mediaFilter.value) ? library.playlists.map((playlist) => ({ kind: 'playlist' as const, playlist, updated_at: playlist.latest_created_at })) : []),
  ]
  const name = (entry: LibraryEntry) => entry.kind === 'file' ? entry.file.title ?? entry.file.filename : entry.playlist.title
  const id = (entry: LibraryEntry) => entry.kind === 'file' ? entry.file.id : entry.playlist.id
  const collator = new Intl.Collator(locale.value, { numeric: true, sensitivity: 'base' })
  return entries.sort((left, right) => {
    const order = sortBy.value === 'name'
      ? collator.compare(name(left), name(right))
      : Date.parse(right.updated_at) - Date.parse(left.updated_at)
    return (order || id(left).localeCompare(id(right))) * (reverse.value ? -1 : 1)
  })
})

onMounted(() => Promise.all([library.refresh(), settings.refresh()]))

function bytes(value: number) { return formatBytes(value, locale.value) }
function playlistThumbnailFailed(id: string) {
  failedPlaylistThumbnails.value = new Set([...failedPlaylistThumbnails.value, id])
}
function openPlaylist(playlist: CompletedPlaylistGroup) {
  void router.push({ name: 'completed', query: { playlist: playlist.id } })
}
function closePlaylist() {
  void router.push({ name: 'completed' })
}
async function confirmDelete() {
  if (!pendingDelete.value) return
  const file = pendingDelete.value
  if (await library.remove(file)) {
    if (selected.value?.id === file.id) selected.value = null
    pendingDelete.value = null
    if (requestedPlaylistId.value && !activePlaylist.value) closePlaylist()
  }
}
</script>

<template>
  <section class="min-w-0">
    <template v-if="requestedPlaylistId && activePlaylist">
      <button class="secondary-btn mb-5" type="button" @click="closePlaylist"><ArrowLeft :size="15" />{{ t('completedView.back') }}</button>
      <div class="flex min-w-0 flex-col items-start gap-3 min-[360px]:flex-row min-[360px]:items-end min-[360px]:justify-between">
        <div class="min-w-0"><p class="eyebrow">{{ t('completedView.playlist') }}</p><h2 class="mt-2 truncate text-2xl font-semibold" :title="activePlaylist.title">{{ activePlaylist.title }}</h2><p class="mt-1 text-sm text-muted">{{ t('completedView.downloadedVideos', { count: activePlaylist.files.length }) }} · {{ bytes(activePlaylist.size_bytes) }}</p></div>
        <span class="badge muted shrink-0"><ListVideo :size="12" />{{ activePlaylist.files.length }}</span>
      </div>
      <div class="mt-6 grid min-w-0 grid-cols-[minmax(0,1fr)] gap-4 sm:grid-cols-2 xl:grid-cols-3" data-playlist-gallery>
        <CompletedMediaCard v-for="file in activePlaylist.files" :key="file.id" :file="file" @play="selected = $event" @delete="pendingDelete = $event" />
      </div>
    </template>

    <template v-else-if="!requestedPlaylistId">
      <div class="browse-header">
        <div class="browse-heading">
          <p class="eyebrow">{{ t('completedView.eyebrow') }}</p>
          <div class="mt-2 flex flex-wrap items-center gap-2"><h2 class="text-2xl font-semibold">{{ t('completedView.title') }}</h2><span v-if="library.completed.length" class="badge muted whitespace-nowrap">{{ t('completedView.fileCount', { count: library.completed.length }) }}</span></div>
          <p class="mt-1 text-sm text-muted">{{ t('completedView.description') }}</p>
        </div>
        <div class="browse-controls">
          <div class="browse-filters" role="group" :aria-label="t('browse.mediaFilter')">
            <button v-for="filter in filters" :key="filter" class="browse-filter" :class="{ active: mediaFilter === filter }" type="button" :aria-pressed="mediaFilter === filter" @click="mediaFilter = filter"><component :is="filterIcons[filter]" :size="15" aria-hidden="true" /><span>{{ t(`browse.${filter}`) }}</span></button>
          </div>
          <SortDropdown v-model:sort-by="sortBy" v-model:reverse="reverse" />
        </div>
      </div>
      <p v-if="library.error" class="error-panel mt-5" role="alert">{{ library.error }}</p>

      <div v-if="library.loading && !library.completed.length" class="card mt-6 flex min-h-48 items-center justify-center gap-2 p-8 text-sm text-muted" role="status"><LoaderCircle class="animate-spin" :size="18" />{{ t('completedView.loading') }}</div>
      <div v-else-if="libraryEntries.length" class="completed-grid mt-6 grid min-w-0 grid-cols-[minmax(0,1fr)] gap-4 sm:grid-cols-2 xl:grid-cols-3">
        <template v-for="entry in libraryEntries" :key="entry.kind === 'file' ? entry.file.id : `playlist-${entry.playlist.id}`">
          <CompletedMediaCard v-if="entry.kind === 'file'" :file="entry.file" @play="selected = $event" @delete="pendingDelete = $event" />
          <article v-else class="playlist-card-shell min-w-0 w-full max-w-full" :data-playlist-id="entry.playlist.id">
            <div class="playlist-media-card media-card card min-w-0 w-full max-w-full overflow-hidden">
              <div class="media-cover">
                <img v-if="entry.playlist.files[0].thumbnail_available && !failedPlaylistThumbnails.has(entry.playlist.files[0].id)" class="size-full object-cover" :src="`/api/files/${entry.playlist.files[0].id}/thumbnail`" alt="" loading="lazy" @error="playlistThumbnailFailed(entry.playlist.files[0].id)" />
                <Music2 v-else-if="entry.playlist.files[0].mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" />
                <span class="playlist-card-icon"><ListVideo :size="19" /></span>
                <span class="playlist-card-count">{{ t('completedView.itemCount', { count: entry.playlist.files.length }) }}</span>
                <span v-if="entry.playlist.progress_percent > 0" class="watch-progress"><span :style="{ width: `${entry.playlist.progress_percent}%` }"></span></span>
              </div>
              <div class="p-4">
                <div class="min-w-0"><h3 class="truncate text-sm font-semibold" :title="entry.playlist.title">{{ entry.playlist.title }}</h3><p class="mt-1 truncate text-[11px] text-muted">{{ t('completedView.playlist') }} · {{ bytes(entry.playlist.size_bytes) }}<template v-if="entry.playlist.watched_files"> · {{ t('completedView.watched', { watched: entry.playlist.watched_files, total: entry.playlist.files.length }) }}</template></p></div>
                <button class="primary-btn mt-4 w-full" type="button" :aria-label="t('completedView.openPlaylistNamed', { name: entry.playlist.title })" @click="openPlaylist(entry.playlist)"><ListVideo :size="15" />{{ t('completedView.openPlaylist') }}</button>
              </div>
            </div>
          </article>
        </template>
      </div>
      <div v-else-if="!library.loading" class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><CircleCheck :size="22" /></div><h3 class="mt-4 text-sm font-semibold">{{ t(library.completed.length ? 'browse.noMatches' : 'completedView.noFiles') }}</h3><p class="mt-2 text-xs text-muted">{{ t(library.completed.length ? 'browse.resetHelp' : 'completedView.noFilesHelp') }}</p></div>
    </template>

    <div v-if="requestedPlaylistId && !activePlaylist && !library.loading" class="card mt-6 flex min-h-64 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><ListVideo :size="22" /></div><h3 class="mt-4 text-sm font-semibold">{{ t('completedView.unavailable') }}</h3><p class="mt-2 text-xs text-muted">{{ t('completedView.unavailableHelp') }}</p><button class="secondary-btn mt-5" type="button" @click="closePlaylist"><ArrowLeft :size="15" />{{ t('completedView.back') }}</button></div>

    <MediaPlayerDialog v-if="selected" :file="selected" :local-client="settings.network?.local_client ?? false" @close="selected = null" />
    <div v-if="pendingDelete" class="modal" role="dialog" aria-modal="true" aria-labelledby="delete-file-title">
      <button class="modal-backdrop" type="button" :aria-label="t('completedView.cancelDeletion')" @click="pendingDelete = null"></button>
      <div class="modal-panel max-w-md">
        <div class="flex items-start justify-between gap-4"><div class="empty-icon text-rose-300"><TriangleAlert :size="20" /></div><button class="icon-btn" type="button" :aria-label="t('completedView.cancelDeletion')" @click="pendingDelete = null"><X :size="16" /></button></div>
        <h2 id="delete-file-title" class="mt-5 text-lg font-semibold">{{ t('completedView.deleteTitle') }}</h2>
        <p class="mt-2 text-sm leading-6 text-muted">{{ t('completedView.deleteDescription', { name: pendingDelete.title ?? pendingDelete.filename }) }}</p>
        <div class="mt-6 flex justify-end gap-2"><button class="secondary-btn" type="button" :disabled="library.actingId === pendingDelete.id" @click="pendingDelete = null">{{ t('common.cancel') }}</button><button class="danger-btn" type="button" :disabled="library.actingId === pendingDelete.id" @click="confirmDelete"><LoaderCircle v-if="library.actingId === pendingDelete.id" class="animate-spin" :size="15" /><Trash2 v-else :size="15" />{{ library.actingId === pendingDelete.id ? t('common.deleting') : t('common.deletePermanently') }}</button></div>
      </div>
    </div>
  </section>
</template>
