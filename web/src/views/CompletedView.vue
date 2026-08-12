<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, CircleCheck, FileVideo, ListVideo, LoaderCircle, Music2, Trash2, TriangleAlert, X } from '@lucide/vue'
import CompletedMediaCard from '@/components/CompletedMediaCard.vue'
import MediaPlayerDialog from '@/components/MediaPlayerDialog.vue'
import { useLibraryStore, type CompletedPlaylistGroup } from '@/stores/library'
import { useSettingsStore } from '@/stores/settings'
import type { CompletedFile } from '@/app/api/client'

type LibraryEntry =
  | { kind: 'file'; file: CompletedFile; updated_at: string }
  | { kind: 'playlist'; playlist: CompletedPlaylistGroup; updated_at: string }

const library = useLibraryStore()
const settings = useSettingsStore()
const route = useRoute()
const router = useRouter()
const selected = ref<CompletedFile | null>(null)
const pendingDelete = ref<CompletedFile | null>(null)
const failedPlaylistThumbnails = ref(new Set<string>())

const requestedPlaylistId = computed(() => typeof route.query.playlist === 'string' ? route.query.playlist : null)
const activePlaylist = computed(() => requestedPlaylistId.value ? library.playlists.find((playlist) => playlist.id === requestedPlaylistId.value) ?? null : null)
const libraryEntries = computed<LibraryEntry[]>(() => [
  ...library.standalone.map((file) => ({ kind: 'file' as const, file, updated_at: file.created_at })),
  ...library.playlists.map((playlist) => ({ kind: 'playlist' as const, playlist, updated_at: playlist.latest_created_at })),
].sort((left, right) => right.updated_at.localeCompare(left.updated_at)))

onMounted(() => Promise.all([library.refresh(), settings.refresh()]))

function bytes(value: number) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = value, unit = 0
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++ }
  return `${size.toFixed(unit ? 1 : 0)} ${units[unit]}`
}
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
      <button class="secondary-btn mb-5" type="button" @click="closePlaylist"><ArrowLeft :size="15" />Back to completed</button>
      <div class="flex min-w-0 flex-col items-start gap-3 min-[360px]:flex-row min-[360px]:items-end min-[360px]:justify-between">
        <div class="min-w-0"><p class="eyebrow">Playlist</p><h2 class="mt-2 truncate text-2xl font-semibold" :title="activePlaylist.title">{{ activePlaylist.title }}</h2><p class="mt-1 text-sm text-muted">{{ activePlaylist.files.length }} downloaded {{ activePlaylist.files.length === 1 ? 'video' : 'videos' }} · {{ bytes(activePlaylist.size_bytes) }}</p></div>
        <span class="badge muted shrink-0"><ListVideo :size="12" />{{ activePlaylist.files.length }}</span>
      </div>
      <div class="mt-6 grid min-w-0 grid-cols-[minmax(0,1fr)] gap-4 sm:grid-cols-2 xl:grid-cols-3" data-playlist-gallery>
        <CompletedMediaCard v-for="file in activePlaylist.files" :key="file.id" :file="file" @play="selected = $event" @delete="pendingDelete = $event" />
      </div>
    </template>

    <template v-else-if="!requestedPlaylistId">
      <div class="flex min-w-0 flex-col items-start gap-3 min-[360px]:flex-row min-[360px]:items-end min-[360px]:justify-between">
        <div class="min-w-0"><p class="eyebrow">Files</p><h2 class="mt-2 text-2xl font-semibold">Completed</h2><p class="mt-1 text-sm text-muted">Browse playlists and individual media downloaded by Fetch.</p></div>
        <span v-if="library.completed.length" class="badge muted shrink-0 whitespace-nowrap">{{ library.completed.length }} {{ library.completed.length === 1 ? 'file' : 'files' }}</span>
      </div>
      <p v-if="library.error" class="error-panel mt-5" role="alert">{{ library.error }}</p>

      <div v-if="library.loading && !library.completed.length" class="card mt-6 flex min-h-48 items-center justify-center gap-2 p-8 text-sm text-muted" role="status"><LoaderCircle class="animate-spin" :size="18" />Loading completed files…</div>
      <div v-else-if="libraryEntries.length" class="completed-grid mt-6 grid min-w-0 grid-cols-[minmax(0,1fr)] gap-4 sm:grid-cols-2 xl:grid-cols-3">
        <template v-for="entry in libraryEntries" :key="entry.kind === 'file' ? entry.file.id : `playlist-${entry.playlist.id}`">
          <CompletedMediaCard v-if="entry.kind === 'file'" :file="entry.file" @play="selected = $event" @delete="pendingDelete = $event" />
          <article v-else class="playlist-card-shell min-w-0 w-full max-w-full" :data-playlist-id="entry.playlist.id">
            <div class="playlist-media-card media-card card min-w-0 w-full max-w-full overflow-hidden">
              <div class="media-cover">
                <img v-if="entry.playlist.files[0].thumbnail_available && !failedPlaylistThumbnails.has(entry.playlist.files[0].id)" class="size-full object-cover" :src="`/api/files/${entry.playlist.files[0].id}/thumbnail`" alt="" loading="lazy" @error="playlistThumbnailFailed(entry.playlist.files[0].id)" />
                <Music2 v-else-if="entry.playlist.files[0].mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" />
                <span class="playlist-card-icon"><ListVideo :size="19" /></span>
                <span class="playlist-card-count">{{ entry.playlist.files.length }} {{ entry.playlist.files.length === 1 ? 'item' : 'items' }}</span>
                <span v-if="entry.playlist.progress_percent > 0" class="watch-progress"><span :style="{ width: `${entry.playlist.progress_percent}%` }"></span></span>
              </div>
              <div class="p-4">
                <div class="min-w-0"><h3 class="truncate text-sm font-semibold" :title="entry.playlist.title">{{ entry.playlist.title }}</h3><p class="mt-1 truncate text-[11px] text-muted">Playlist · {{ bytes(entry.playlist.size_bytes) }}<template v-if="entry.playlist.watched_files"> · {{ entry.playlist.watched_files }}/{{ entry.playlist.files.length }} watched</template></p></div>
                <button class="primary-btn mt-4 w-full" type="button" :aria-label="`Open playlist ${entry.playlist.title}`" @click="openPlaylist(entry.playlist)"><ListVideo :size="15" />Open playlist</button>
              </div>
            </div>
          </article>
        </template>
      </div>
      <div v-else-if="!library.loading" class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><CircleCheck :size="22" /></div><h3 class="mt-4 text-sm font-semibold">No completed files</h3><p class="mt-2 text-xs text-muted">Completed downloads will appear here.</p></div>
    </template>

    <div v-if="requestedPlaylistId && !activePlaylist && !library.loading" class="card mt-6 flex min-h-64 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><ListVideo :size="22" /></div><h3 class="mt-4 text-sm font-semibold">Playlist is unavailable</h3><p class="mt-2 text-xs text-muted">It may have no downloaded files remaining.</p><button class="secondary-btn mt-5" type="button" @click="closePlaylist"><ArrowLeft :size="15" />Back to completed</button></div>

    <MediaPlayerDialog v-if="selected" :file="selected" :local-client="settings.network?.local_client ?? false" @close="selected = null" />
    <div v-if="pendingDelete" class="modal" role="dialog" aria-modal="true" aria-labelledby="delete-file-title">
      <button class="modal-backdrop" type="button" aria-label="Cancel deletion" @click="pendingDelete = null"></button>
      <div class="modal-panel max-w-md">
        <div class="flex items-start justify-between gap-4"><div class="empty-icon text-rose-300"><TriangleAlert :size="20" /></div><button class="icon-btn" type="button" aria-label="Cancel deletion" @click="pendingDelete = null"><X :size="16" /></button></div>
        <h2 id="delete-file-title" class="mt-5 text-lg font-semibold">Delete downloaded file?</h2>
        <p class="mt-2 text-sm leading-6 text-muted"><strong class="text-app-text">{{ pendingDelete.title ?? pendingDelete.filename }}</strong> and its cached artwork will be permanently removed from disk. Download history will remain.</p>
        <div class="mt-6 flex justify-end gap-2"><button class="secondary-btn" type="button" :disabled="library.actingId === pendingDelete.id" @click="pendingDelete = null">Cancel</button><button class="danger-btn" type="button" :disabled="library.actingId === pendingDelete.id" @click="confirmDelete"><LoaderCircle v-if="library.actingId === pendingDelete.id" class="animate-spin" :size="15" /><Trash2 v-else :size="15" />{{ library.actingId === pendingDelete.id ? 'Deleting…' : 'Delete permanently' }}</button></div>
      </div>
    </div>
  </section>
</template>
