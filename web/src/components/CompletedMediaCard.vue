<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Download, ExternalLink, FileVideo, FolderOpen, LoaderCircle, Music2, Pencil, Play, Trash2 } from '@lucide/vue'
import { useLibraryStore } from '@/stores/library'
import { useSettingsStore } from '@/stores/settings'
import type { CompletedFile } from '@/app/api/client'
import { formatBytes } from '@/i18n/format'

const props = defineProps<{ file: CompletedFile }>()
const emit = defineEmits<{ play: [file: CompletedFile]; delete: [file: CompletedFile]; edit: [file: CompletedFile] }>()
const library = useLibraryStore()
const settings = useSettingsStore()
const { t, locale } = useI18n()
const thumbnailFailed = ref(false)
watch(() => library.thumbnailVersions[props.file.id], () => { thumbnailFailed.value = false })

function bytes(value: number) { return formatBytes(value, locale.value) }
function progressPercent() {
  const progress = props.file.playback
  if (!progress) return 0
  return progress.completed ? 100 : Math.min(100, progress.position_seconds / progress.duration_seconds * 100)
}
function playLabel() {
  if (props.file.playback?.completed) return props.file.mime_type.startsWith('audio/') ? t('mediaCard.playAgain') : t('mediaCard.watchAgain')
  if ((props.file.playback?.position_seconds ?? 0) >= 10) return t('mediaCard.resume')
  return props.file.mime_type.startsWith('audio/') ? t('mediaCard.play') : t('mediaCard.watch')
}
</script>

<template>
  <article :id="`completed-${file.id}`" tabindex="-1" class="media-card card min-w-0 w-full max-w-full overflow-hidden">
    <div class="relative">
      <button v-if="file.browser_playable" class="media-cover group w-full" type="button" :aria-label="t('mediaCard.playNamed', { name: file.title ?? file.filename })" @click="emit('play', file)">
        <img v-if="file.thumbnail_available && !thumbnailFailed" class="size-full object-cover" :src="library.thumbnailUrl(file)" alt="" loading="lazy" @error="thumbnailFailed = true" />
        <Music2 v-else-if="file.mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" />
        <span class="play-button"><Play class="ml-0.5" fill="currentColor" :size="20" /></span>
        <span v-if="file.playback" class="watch-progress" :class="{ completed: file.playback.completed }"><span :style="{ width: `${progressPercent()}%` }"></span></span>
      </button>
      <div v-else class="media-cover">
        <img v-if="file.thumbnail_available && !thumbnailFailed" class="size-full object-cover" :src="library.thumbnailUrl(file)" alt="" loading="lazy" @error="thumbnailFailed = true" />
        <Music2 v-else-if="file.mime_type.startsWith('audio/')" class="text-muted" :size="48" /><FileVideo v-else class="text-muted" :size="48" />
        <span v-if="file.playback" class="watch-progress" :class="{ completed: file.playback.completed }"><span :style="{ width: `${progressPercent()}%` }"></span></span>
      </div>
      <button class="metadata-pencil" type="button" :aria-label="t('metadata.editNamed', { name: file.title ?? file.filename })" :title="t('metadata.edit')" @click.stop="emit('edit', file)"><Pencil :size="17" /></button>
    </div>
    <div class="p-4">
      <div class="flex items-start gap-3">
        <div class="min-w-0 flex-1"><h3 class="truncate text-sm font-semibold" :title="file.title ?? file.filename">{{ file.title ?? file.filename }}</h3><p class="mt-1 truncate text-[11px] text-muted" :title="file.filename">{{ file.filename }} · {{ bytes(file.size_bytes) }}</p></div>
        <button class="icon-btn size-8 shrink-0 text-rose-300" type="button" :aria-label="t('mediaCard.deleteNamed', { name: file.title ?? file.filename })" @click="emit('delete', file)"><Trash2 :size="14" /></button>
      </div>
      <div class="mt-4 grid grid-cols-1 gap-2 min-[360px]:grid-cols-2">
        <button v-if="file.browser_playable" class="secondary-btn min-w-0" type="button" @click="emit('play', file)"><Play class="shrink-0" :size="15" /><span class="min-w-0 truncate">{{ playLabel() }}</span></button>
        <a v-else class="secondary-btn min-w-0" :href="`/api/files/${file.id}/stream`" target="_blank" rel="noopener"><ExternalLink class="shrink-0" :size="15" /><span class="min-w-0 truncate">{{ t('common.openFile') }}</span></a>
        <button v-if="settings.loading || !settings.network" class="secondary-btn min-w-0" type="button" disabled><LoaderCircle class="shrink-0 animate-spin" :size="15" /><span class="min-w-0 truncate">{{ t('mediaCard.checkingHost') }}</span></button>
        <button v-else-if="settings.network.local_client" class="primary-btn min-w-0" type="button" :disabled="library.actingId === file.id" @click="library.reveal(file)"><LoaderCircle v-if="library.actingId === file.id" class="shrink-0 animate-spin" :size="15" /><FolderOpen v-else class="shrink-0" :size="15" /><span class="min-w-0 truncate">{{ t('common.openFolder') }}</span></button>
        <a v-else class="primary-btn min-w-0" :href="`/api/files/${file.id}/download`"><Download class="shrink-0" :size="15" /><span class="min-w-0 truncate">{{ t('common.download') }}</span></a>
      </div>
      <p v-if="!file.browser_playable" class="mt-3 text-[11px] leading-5 text-muted">{{ t('mediaCard.previewUnavailable') }}</p>
    </div>
  </article>
</template>
