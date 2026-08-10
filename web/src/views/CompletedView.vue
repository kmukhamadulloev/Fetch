<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { CircleCheck, Download, ExternalLink, FileVideo, FolderOpen, LoaderCircle, Music2, Play, Trash2, TriangleAlert, X } from '@lucide/vue'
import MediaPlayerDialog from '@/components/MediaPlayerDialog.vue'
import { useLibraryStore } from '@/stores/library'
import { useSettingsStore } from '@/stores/settings'
import type { CompletedFile } from '@/app/api/client'

const library = useLibraryStore()
const settings = useSettingsStore()
const selected = ref<CompletedFile | null>(null)
const pendingDelete = ref<CompletedFile | null>(null)
const failedThumbnails = ref(new Set<string>())
onMounted(() => Promise.all([library.refresh(), settings.refresh()]))

function bytes(value: number) {
  const units = ['B', 'KB', 'MB', 'GB']
  let size = value, unit = 0
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++ }
  return `${size.toFixed(unit ? 1 : 0)} ${units[unit]}`
}
function thumbnailFailed(id: string) {
  failedThumbnails.value = new Set([...failedThumbnails.value, id])
}
async function confirmDelete() {
  if (!pendingDelete.value) return
  const file = pendingDelete.value
  if (await library.remove(file)) {
    if (selected.value?.id === file.id) selected.value = null
    pendingDelete.value = null
  }
}
</script>

<template>
  <section>
    <div class="flex items-end justify-between gap-4"><div><p class="eyebrow">Files</p><h2 class="mt-2 text-2xl font-semibold">Completed</h2><p class="mt-1 text-sm text-muted">Play compatible media, open it on the host, or download it remotely.</p></div><span v-if="library.completed.length" class="badge muted">{{ library.completed.length }} {{ library.completed.length === 1 ? 'file' : 'files' }}</span></div>
    <p v-if="library.error" class="error-panel mt-5" role="alert">{{ library.error }}</p>
    <div v-if="library.completed.length" class="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
      <article v-for="file in library.completed" :key="file.id" class="media-card card overflow-hidden">
        <button v-if="file.browser_playable" class="media-cover group w-full" type="button" :aria-label="`Play ${file.title ?? file.filename}`" @click="selected = file">
          <img v-if="file.thumbnail_available && !failedThumbnails.has(file.id)" class="size-full object-cover" :src="`/api/files/${file.id}/thumbnail`" alt="" loading="lazy" @error="thumbnailFailed(file.id)" />
          <Music2 v-else-if="file.mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" />
          <span class="play-button"><Play class="ml-0.5" fill="currentColor" :size="20" /></span>
        </button>
        <div v-else class="media-cover"><img v-if="file.thumbnail_available && !failedThumbnails.has(file.id)" class="size-full object-cover" :src="`/api/files/${file.id}/thumbnail`" alt="" loading="lazy" @error="thumbnailFailed(file.id)" /><Music2 v-else-if="file.mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" /></div>
        <div class="p-4">
          <div class="flex items-start gap-3"><div class="min-w-0 flex-1"><h3 class="truncate text-sm font-semibold" :title="file.title ?? file.filename">{{ file.title ?? file.filename }}</h3><p class="mt-1 truncate text-[11px] text-muted" :title="file.filename">{{ file.filename }} · {{ bytes(file.size_bytes) }}</p></div><button class="icon-btn size-8 shrink-0 text-rose-300" type="button" :aria-label="`Delete ${file.title ?? file.filename}`" @click="pendingDelete = file"><Trash2 :size="14" /></button></div>
          <div class="mt-4 grid grid-cols-2 gap-2">
            <button v-if="file.browser_playable" class="secondary-btn" type="button" @click="selected = file"><Play :size="15" />{{ file.mime_type.startsWith('audio/') ? 'Play' : 'Watch' }}</button>
            <a v-else class="secondary-btn" :href="`/api/files/${file.id}/stream`" target="_blank" rel="noopener"><ExternalLink :size="15" />Open file</a>
            <button v-if="settings.loading || !settings.network" class="secondary-btn" type="button" disabled><LoaderCircle class="animate-spin" :size="15" />Checking host…</button>
            <button v-else-if="settings.network.local_client" class="primary-btn" type="button" :disabled="library.actingId === file.id" @click="library.reveal(file)"><LoaderCircle v-if="library.actingId === file.id" class="animate-spin" :size="15" /><FolderOpen v-else :size="15" />Open folder</button>
            <a v-else class="primary-btn" :href="`/api/files/${file.id}/download`"><Download :size="15" />Download</a>
          </div>
          <p v-if="!file.browser_playable" class="mt-3 text-[11px] leading-5 text-muted">Preview is unavailable in this browser. Fetch serves the original file without transcoding.</p>
        </div>
      </article>
    </div>
    <div v-else class="card mt-6 flex min-h-80 flex-col items-center justify-center p-8 text-center"><div class="empty-icon"><CircleCheck :size="22" /></div><h3 class="mt-4 text-sm font-semibold">No completed files</h3><p class="mt-2 text-xs text-muted">Completed downloads will appear here.</p></div>
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
