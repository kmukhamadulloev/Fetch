<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Download, ExternalLink, Film, FolderOpen, Music2, RotateCcw, X } from '@lucide/vue'
import type { CompletedFile } from '@/app/api/client'
import { useLibraryStore } from '@/stores/library'
import { formatBytes } from '@/i18n/format'

const props = defineProps<{ file: CompletedFile; localClient: boolean }>()
const emit = defineEmits<{ close: [] }>()
const library = useLibraryStore()
const { t, locale } = useI18n()
const closeButton = ref<HTMLButtonElement | null>(null)
const panel = ref<HTMLElement | null>(null)
const stage = ref<HTMLElement | null>(null)
const media = ref<HTMLMediaElement | null>(null)
const mediaFailed = ref(false)
const thumbnailFailed = ref(false)
const controlFeedback = ref('')
const previousOverflow = document.body.style.overflow
const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
let feedbackTimer: number | undefined
let lastSaveAt = 0
let pendingSave: { position: number; duration: number } | null = null
let saving = false

function close() { emit('close') }
function announce(message: string) {
  controlFeedback.value = message
  if (feedbackTimer !== undefined) window.clearTimeout(feedbackTimer)
  feedbackTimer = window.setTimeout(() => { controlFeedback.value = '' }, 1_200)
}
function editableTarget(target: EventTarget | null) {
  return target instanceof HTMLInputElement
    || target instanceof HTMLTextAreaElement
    || target instanceof HTMLSelectElement
    || (target instanceof HTMLElement && target.isContentEditable)
}
function seek(seconds: number) {
  const element = media.value
  if (!element) return
  try {
    const maximum = Number.isFinite(element.duration) ? element.duration : Number.POSITIVE_INFINITY
    element.currentTime = Math.min(maximum, Math.max(0, element.currentTime + seconds))
    announce(t(seconds > 0 ? 'player.forward' : 'player.back'))
  } catch {
    announce(t('player.seekingUnavailable'))
  }
}
function changeVolume(amount: number) {
  const element = media.value
  if (!element) return
  element.volume = Math.min(1, Math.max(0, element.volume + amount))
  if (amount > 0 && element.muted) element.muted = false
  announce(t('player.volume', { value: Math.round(element.volume * 100) }))
}
async function toggleFullscreen() {
  try {
    if (document.fullscreenElement) {
      await document.exitFullscreen()
      announce(t('player.exitedFullscreen'))
    } else if (stage.value) {
      await stage.value.requestFullscreen()
      announce(t('player.fullscreen'))
    }
  } catch {
    announce(t('player.fullscreenUnavailable'))
  }
}
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') return close()
  if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey || editableTarget(event.target)) return
  if (event.key === 'ArrowLeft') { event.preventDefault(); seek(-5); return }
  if (event.key === 'ArrowRight') { event.preventDefault(); seek(5); return }
  if (event.key === 'ArrowUp') { event.preventDefault(); changeVolume(.1); return }
  if (event.key === 'ArrowDown') { event.preventDefault(); changeVolume(-.1); return }
  if (event.key.toLowerCase() === 'f') {
    event.preventDefault()
    if (!event.repeat) void toggleFullscreen()
    return
  }
  if (event.key !== 'Tab' || !panel.value) return
  const focusable = [...panel.value.querySelectorAll<HTMLElement>('a[href], button:not([disabled]), video[controls], audio[controls], [tabindex]:not([tabindex="-1"])')]
  if (!focusable.length) return
  const first = focusable[0], last = focusable[focusable.length - 1]
  if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus() }
  else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus() }
}
function bytes(value: number) { return formatBytes(value, locale.value) }
async function flushProgress() {
  if (saving) return
  saving = true
  while (pendingSave) {
    const snapshot = pendingSave
    pendingSave = null
    await library.saveProgress(props.file, snapshot.position, snapshot.duration)
  }
  saving = false
}
function saveCurrent(force = false) {
  const element = media.value
  if (!element || !Number.isFinite(element.duration) || element.duration <= 0 || !Number.isFinite(element.currentTime)) return
  if (element.currentTime < 10 && !element.ended) return
  const now = Date.now()
  if (!force && now - lastSaveAt < 5_000) return
  lastSaveAt = now
  pendingSave = { position: Math.max(0, element.currentTime), duration: element.duration }
  void flushProgress()
}
function resumePlayback() {
  const element = media.value
  const progress = props.file.playback
  if (!element || !progress || progress.completed || progress.position_seconds < 10) return
  if (progress.position_seconds < element.duration - 15) {
    element.currentTime = progress.position_seconds
    announce(t('player.resumedAt', { time: `${Math.floor(progress.position_seconds / 60)}:${String(Math.floor(progress.position_seconds % 60)).padStart(2, '0')}` }))
  }
}
async function startOver() {
  if (!await library.resetProgress(props.file)) return
  if (media.value) media.value.currentTime = 0
  pendingSave = null
  lastSaveAt = Date.now()
  announce(t('player.startedOver'))
}

onMounted(async () => {
  document.body.style.overflow = 'hidden'
  document.addEventListener('keydown', onKeydown)
  await nextTick()
  closeButton.value?.focus()
})
onBeforeUnmount(() => {
  saveCurrent(true)
  document.body.style.overflow = previousOverflow
  document.removeEventListener('keydown', onKeydown)
  if (feedbackTimer !== undefined) window.clearTimeout(feedbackTimer)
  previousFocus?.focus()
})
</script>

<template>
  <Teleport to="body">
    <div class="player-dialog" role="dialog" aria-modal="true" :aria-labelledby="`player-title-${file.id}`">
      <div class="player-backdrop" aria-hidden="true" @click="close"></div>
      <section ref="panel" class="player-panel">
        <header class="player-header">
          <div class="min-w-0"><p class="eyebrow">{{ t('player.nowPlaying') }}</p><h2 :id="`player-title-${file.id}`" class="mt-1 truncate text-sm font-semibold sm:text-base">{{ file.title ?? file.filename }}</h2></div>
          <button ref="closeButton" class="icon-btn shrink-0" type="button" :aria-label="t('player.close')" @click="close"><X :size="18" /></button>
        </header>

        <div ref="stage" class="player-stage">
          <video v-if="!mediaFailed && file.mime_type.startsWith('video/')" ref="media" class="player-video" controls autoplay playsinline preload="metadata" :poster="file.thumbnail_available && !thumbnailFailed ? `/api/files/${file.id}/thumbnail` : undefined" :src="`/api/files/${file.id}/stream`" @loadedmetadata="resumePlayback" @timeupdate="saveCurrent()" @pause="saveCurrent(true)" @seeked="saveCurrent(true)" @ended="saveCurrent(true)" @error="mediaFailed = true"></video>
          <div v-else class="player-audio-stage">
            <div class="player-art"><img v-if="file.thumbnail_available && !thumbnailFailed" class="size-full rounded-[inherit] object-cover" :src="`/api/files/${file.id}/thumbnail`" alt="" @error="thumbnailFailed = true" /><Music2 v-else-if="file.mime_type.startsWith('audio/')" :size="52" /><Film v-else :size="52" /></div>
            <audio v-if="!mediaFailed && file.mime_type.startsWith('audio/')" ref="media" class="w-full" controls autoplay preload="metadata" :src="`/api/files/${file.id}/stream`" @loadedmetadata="resumePlayback" @timeupdate="saveCurrent()" @pause="saveCurrent(true)" @seeked="saveCurrent(true)" @ended="saveCurrent(true)" @error="mediaFailed = true"></audio>
            <p v-if="mediaFailed" class="max-w-md text-center text-xs leading-5 text-muted">{{ t('player.playbackFailed') }}</p>
          </div>
          <div v-if="controlFeedback" class="player-feedback" role="status" aria-live="polite">{{ controlFeedback }}</div>
        </div>

        <footer class="player-footer">
          <div class="min-w-0 flex-1"><div class="truncate text-xs font-medium">{{ file.filename }}</div><div class="mt-1 text-[11px] text-muted">{{ file.mime_type }} · {{ bytes(file.size_bytes) }}</div><p v-if="library.playbackError" class="mt-1 text-[11px] text-rose-300" role="status">{{ library.playbackError }}</p><div class="player-shortcuts" :aria-label="t('player.shortcuts')"><span><kbd>←</kbd><kbd>→</kbd> {{ t('player.seekShortcut') }}</span><span><kbd>↑</kbd><kbd>↓</kbd> {{ t('player.volumeShortcut') }}</span><span><kbd>F</kbd> {{ t('player.fullscreenShortcut') }}</span></div></div>
          <div class="flex shrink-0 flex-wrap justify-end gap-2"><button v-if="file.playback" class="secondary-btn" type="button" :aria-label="t('player.startOver')" @click="startOver"><RotateCcw :size="15" /><span class="hidden sm:inline">{{ t('player.startOver') }}</span></button><a class="secondary-btn" :href="`/api/files/${file.id}/stream`" target="_blank" rel="noopener" :aria-label="t('common.openFile')"><ExternalLink :size="15" /><span class="hidden sm:inline">{{ t('common.openFile') }}</span></a><button v-if="localClient" class="primary-btn" type="button" :aria-label="t('common.openFolder')" @click="library.reveal(props.file)"><FolderOpen :size="15" /><span class="hidden sm:inline">{{ t('common.openFolder') }}</span></button><a v-else class="primary-btn" :href="`/api/files/${file.id}/download`" :aria-label="t('common.download')"><Download :size="15" /><span class="hidden sm:inline">{{ t('common.download') }}</span></a></div>
        </footer>
      </section>
    </div>
  </Teleport>
</template>
