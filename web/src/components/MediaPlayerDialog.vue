<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { Download, ExternalLink, Film, FolderOpen, Music2, X } from '@lucide/vue'
import type { CompletedFile } from '@/app/api/client'
import { useLibraryStore } from '@/stores/library'

const props = defineProps<{ file: CompletedFile; localClient: boolean }>()
const emit = defineEmits<{ close: [] }>()
const library = useLibraryStore()
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
    announce(seconds > 0 ? 'Forward 5 seconds' : 'Back 5 seconds')
  } catch {
    announce('Seeking is unavailable')
  }
}
function changeVolume(amount: number) {
  const element = media.value
  if (!element) return
  element.volume = Math.min(1, Math.max(0, element.volume + amount))
  if (amount > 0 && element.muted) element.muted = false
  announce(`Volume ${Math.round(element.volume * 100)}%`)
}
async function toggleFullscreen() {
  try {
    if (document.fullscreenElement) {
      await document.exitFullscreen()
      announce('Exited fullscreen')
    } else if (stage.value) {
      await stage.value.requestFullscreen()
      announce('Fullscreen')
    }
  } catch {
    announce('Fullscreen is unavailable')
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
function bytes(value: number) {
  const units = ['B', 'KB', 'MB', 'GB']
  let size = value, unit = 0
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++ }
  return `${size.toFixed(unit ? 1 : 0)} ${units[unit]}`
}

onMounted(async () => {
  document.body.style.overflow = 'hidden'
  document.addEventListener('keydown', onKeydown)
  await nextTick()
  closeButton.value?.focus()
})
onBeforeUnmount(() => {
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
          <div class="min-w-0"><p class="eyebrow">Now playing</p><h2 :id="`player-title-${file.id}`" class="mt-1 truncate text-sm font-semibold sm:text-base">{{ file.title ?? file.filename }}</h2></div>
          <button ref="closeButton" class="icon-btn shrink-0" type="button" aria-label="Close player" @click="close"><X :size="18" /></button>
        </header>

        <div ref="stage" class="player-stage">
          <video v-if="!mediaFailed && file.mime_type.startsWith('video/')" ref="media" class="player-video" controls autoplay playsinline preload="metadata" :poster="file.thumbnail_available && !thumbnailFailed ? `/api/files/${file.id}/thumbnail` : undefined" :src="`/api/files/${file.id}/stream`" @error="mediaFailed = true"></video>
          <div v-else class="player-audio-stage">
            <div class="player-art"><img v-if="file.thumbnail_available && !thumbnailFailed" class="size-full rounded-[inherit] object-cover" :src="`/api/files/${file.id}/thumbnail`" alt="" @error="thumbnailFailed = true" /><Music2 v-else-if="file.mime_type.startsWith('audio/')" :size="52" /><Film v-else :size="52" /></div>
            <audio v-if="!mediaFailed && file.mime_type.startsWith('audio/')" ref="media" class="w-full" controls autoplay preload="metadata" :src="`/api/files/${file.id}/stream`" @error="mediaFailed = true"></audio>
            <p v-if="mediaFailed" class="max-w-md text-center text-xs leading-5 text-muted">This browser could not play the file. You can still open or download the original without conversion.</p>
          </div>
          <div v-if="controlFeedback" class="player-feedback" role="status" aria-live="polite">{{ controlFeedback }}</div>
        </div>

        <footer class="player-footer">
          <div class="min-w-0 flex-1"><div class="truncate text-xs font-medium">{{ file.filename }}</div><div class="mt-1 text-[11px] text-muted">{{ file.mime_type }} · {{ bytes(file.size_bytes) }}</div><div class="player-shortcuts" aria-label="Keyboard shortcuts"><span><kbd>←</kbd><kbd>→</kbd> seek 5s</span><span><kbd>↑</kbd><kbd>↓</kbd> volume</span><span><kbd>F</kbd> fullscreen</span></div></div>
          <div class="flex shrink-0 gap-2"><a class="secondary-btn" :href="`/api/files/${file.id}/stream`" target="_blank" rel="noopener" aria-label="Open file"><ExternalLink :size="15" /><span class="hidden sm:inline">Open file</span></a><button v-if="localClient" class="primary-btn" type="button" aria-label="Open folder" @click="library.reveal(props.file)"><FolderOpen :size="15" /><span class="hidden sm:inline">Open folder</span></button><a v-else class="primary-btn" :href="`/api/files/${file.id}/download`" aria-label="Download"><Download :size="15" /><span class="hidden sm:inline">Download</span></a></div>
        </footer>
      </section>
    </div>
  </Teleport>
</template>
