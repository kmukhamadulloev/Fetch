<script setup lang="ts">
import { ref } from 'vue'
import { Download, ExternalLink, FileVideo, FolderOpen, LoaderCircle, Music2, Play, Trash2 } from '@lucide/vue'
import { useLibraryStore } from '@/stores/library'
import { useSettingsStore } from '@/stores/settings'
import type { CompletedFile } from '@/app/api/client'

const props = defineProps<{ file: CompletedFile }>()
const emit = defineEmits<{ play: [file: CompletedFile]; delete: [file: CompletedFile] }>()
const library = useLibraryStore()
const settings = useSettingsStore()
const thumbnailFailed = ref(false)

function bytes(value: number) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = value, unit = 0
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++ }
  return `${size.toFixed(unit ? 1 : 0)} ${units[unit]}`
}
function progressPercent() {
  const progress = props.file.playback
  if (!progress) return 0
  return progress.completed ? 100 : Math.min(100, progress.position_seconds / progress.duration_seconds * 100)
}
function playLabel() {
  if (props.file.playback?.completed) return props.file.mime_type.startsWith('audio/') ? 'Play again' : 'Watch again'
  if ((props.file.playback?.position_seconds ?? 0) >= 10) return 'Resume'
  return props.file.mime_type.startsWith('audio/') ? 'Play' : 'Watch'
}
</script>

<template>
  <article class="media-card card min-w-0 w-full max-w-full overflow-hidden">
    <button v-if="file.browser_playable" class="media-cover group w-full" type="button" :aria-label="`Play ${file.title ?? file.filename}`" @click="emit('play', file)">
      <img v-if="file.thumbnail_available && !thumbnailFailed" class="size-full object-cover" :src="`/api/files/${file.id}/thumbnail`" alt="" loading="lazy" @error="thumbnailFailed = true" />
      <Music2 v-else-if="file.mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" />
      <span class="play-button"><Play class="ml-0.5" fill="currentColor" :size="20" /></span>
      <span v-if="file.playback" class="watch-progress" :class="{ completed: file.playback.completed }"><span :style="{ width: `${progressPercent()}%` }"></span></span>
    </button>
    <div v-else class="media-cover">
      <img v-if="file.thumbnail_available && !thumbnailFailed" class="size-full object-cover" :src="`/api/files/${file.id}/thumbnail`" alt="" loading="lazy" @error="thumbnailFailed = true" />
      <Music2 v-else-if="file.mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" />
      <span v-if="file.playback" class="watch-progress" :class="{ completed: file.playback.completed }"><span :style="{ width: `${progressPercent()}%` }"></span></span>
    </div>
    <div class="p-4">
      <div class="flex items-start gap-3">
        <div class="min-w-0 flex-1"><h3 class="truncate text-sm font-semibold" :title="file.title ?? file.filename">{{ file.title ?? file.filename }}</h3><p class="mt-1 truncate text-[11px] text-muted" :title="file.filename">{{ file.filename }} · {{ bytes(file.size_bytes) }}</p></div>
        <button class="icon-btn size-8 shrink-0 text-rose-300" type="button" :aria-label="`Delete ${file.title ?? file.filename}`" @click="emit('delete', file)"><Trash2 :size="14" /></button>
      </div>
      <div class="mt-4 grid grid-cols-1 gap-2 min-[360px]:grid-cols-2">
        <button v-if="file.browser_playable" class="secondary-btn min-w-0" type="button" @click="emit('play', file)"><Play class="shrink-0" :size="15" /><span class="min-w-0 truncate">{{ playLabel() }}</span></button>
        <a v-else class="secondary-btn min-w-0" :href="`/api/files/${file.id}/stream`" target="_blank" rel="noopener"><ExternalLink class="shrink-0" :size="15" /><span class="min-w-0 truncate">Open file</span></a>
        <button v-if="settings.loading || !settings.network" class="secondary-btn min-w-0" type="button" disabled><LoaderCircle class="shrink-0 animate-spin" :size="15" /><span class="min-w-0 truncate">Checking host…</span></button>
        <button v-else-if="settings.network.local_client" class="primary-btn min-w-0" type="button" :disabled="library.actingId === file.id" @click="library.reveal(file)"><LoaderCircle v-if="library.actingId === file.id" class="shrink-0 animate-spin" :size="15" /><FolderOpen v-else class="shrink-0" :size="15" /><span class="min-w-0 truncate">Open folder</span></button>
        <a v-else class="primary-btn min-w-0" :href="`/api/files/${file.id}/download`"><Download class="shrink-0" :size="15" /><span class="min-w-0 truncate">Download</span></a>
      </div>
      <p v-if="!file.browser_playable" class="mt-3 text-[11px] leading-5 text-muted">Preview is unavailable in this browser. Fetch serves the original file without transcoding.</p>
    </div>
  </article>
</template>
