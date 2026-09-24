<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Play, Pause, Music2, Film, Maximize2 } from '@lucide/vue'
import type { CompletedFile, ProcessingCapabilities } from '@/app/api/client'
import { moveCrop, resizeCrop, type CropRect } from '@/app/processing/editor'
const props = defineProps<{ file: CompletedFile; caps: ProcessingCapabilities; crop: CropRect; cropping: boolean; busy: boolean; start: number; end: number }>()
const emit = defineEmits<{ 'update:crop': [crop: CropRect]; position: [seconds: number]; ready: [ready: boolean] }>()
const { t } = useI18n()
const media = ref<HTMLMediaElement>(), stage = ref<HTMLElement>()
const playError = ref(false)
const failed = ref(false), ready = ref(false), playing = ref(false), position = ref(0), selection = ref(false)
const available = computed(() => props.file.browser_playable && !failed.value)
const ratio = computed(() => (props.caps.width ?? 16) / (props.caps.height ?? 9))
const cropStyle = computed(() => ({ left: `${props.crop.x / (props.caps.width || 1) * 100}%`, top: `${props.crop.y / (props.caps.height || 1) * 100}%`, width: `${props.crop.width / (props.caps.width || 1) * 100}%`, height: `${props.crop.height / (props.caps.height || 1) * 100}%` }))
function time(seconds: number) { return `${Math.floor(seconds / 60)}:${(seconds % 60).toFixed(1).padStart(4, '0')}` }
function seek(seconds: number) { if (media.value && ready.value) { media.value.currentTime = Math.max(0, Math.min(props.caps.duration_seconds, seconds)); position.value = media.value.currentTime; emit('position', position.value) } }
async function play(selected = false) {
  if (!media.value || !ready.value || props.busy) return
  if (playing.value && !selected) { media.value.pause(); return }
  selection.value = selected
  if (selected) seek(props.start)
  playError.value = false
  try { await media.value.play() } catch { playing.value = false; playError.value = true }
}
function update() {
  position.value = media.value?.currentTime ?? 0
  if (selection.value && position.value >= props.end) { media.value?.pause(); selection.value = false; seek(props.end) }
  emit('position', position.value)
}
function loaded() { ready.value = true; emit('ready', true) }
function fail() { failed.value = true; ready.value = false; playing.value = false; emit('ready', false) }
let drag: { x: number; y: number; crop: CropRect; mode: 'move' | 'resize'; width: number; height: number } | null = null
function begin(event: PointerEvent, mode: 'move' | 'resize') {
  if (props.busy || event.button !== 0 || !stage.value) return
  event.preventDefault(); event.stopPropagation()
  const box = stage.value.getBoundingClientRect()
  drag = { x: event.clientX, y: event.clientY, crop: { ...props.crop }, mode, width: box.width, height: box.height }
  ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
}
function move(event: PointerEvent) {
  if (!drag || props.busy) return
  const dx = (event.clientX - drag.x) / drag.width * (props.caps.width ?? 0)
  const dy = (event.clientY - drag.y) / drag.height * (props.caps.height ?? 0)
  emit('update:crop', (drag.mode === 'move' ? moveCrop : resizeCrop)(drag.crop, dx, dy, props.caps.width ?? 0, props.caps.height ?? 0))
}
function keyboard(event: KeyboardEvent, mode: 'move' | 'resize') {
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key) || props.busy) return
  event.preventDefault(); event.stopPropagation()
  const step = event.shiftKey ? 20 : 2
  const dx = event.key === 'ArrowLeft' ? -step : event.key === 'ArrowRight' ? step : 0
  const dy = event.key === 'ArrowUp' ? -step : event.key === 'ArrowDown' ? step : 0
  emit('update:crop', (mode === 'move' ? moveCrop : resizeCrop)(props.crop, dx, dy, props.caps.width ?? 0, props.caps.height ?? 0))
}
</script>
<template>
  <div class="editor-preview">
    <div class="editor-preview-label"><span class="flex items-center gap-2"><Film v-if="caps.video" :size="14" /><Music2 v-else :size="14" />{{ t('quickEdit.sourcePreview') }}</span><span>{{ caps.video ? `${caps.width} × ${caps.height}` : t('exports.audio') }}</span></div>
    <div class="editor-stage">
      <template v-if="available">
        <div v-if="caps.video" ref="stage" class="editor-video-frame" :style="{ aspectRatio: ratio, width: `min(100%, calc(var(--editor-preview-height) * ${ratio}))` }">
          <video ref="media" playsinline preload="metadata" :src="`/api/files/${file.id}/stream`" @loadedmetadata="loaded" @timeupdate="update" @play="playing = true" @pause="playing = false" @ended="playing = false" @error="fail"></video>
          <div v-if="cropping" class="editor-crop-box" :style="cropStyle" @pointermove="move" @pointerup="drag = null" @pointercancel="drag = null" @lostpointercapture="drag = null">
            <button type="button" class="editor-crop-move" :disabled="busy" :aria-label="t('quickEdit.moveCrop')" @pointerdown="begin($event, 'move')" @keydown="keyboard($event, 'move')"></button>
            <span class="editor-crop-grid"></span>
            <button type="button" class="editor-crop-handle" :disabled="busy" :aria-label="t('quickEdit.resizeCrop')" @pointerdown="begin($event, 'resize')" @pointermove.stop="move" @pointerup="drag = null" @pointercancel="drag = null" @lostpointercapture="drag = null" @keydown="keyboard($event, 'resize')"><Maximize2 :size="16" /></button>
          </div>
        </div>
        <div v-else class="editor-audio-stage"><Music2 :size="48" /><audio ref="media" preload="metadata" :src="`/api/files/${file.id}/stream`" @loadedmetadata="loaded" @timeupdate="update" @play="playing = true" @pause="playing = false" @ended="playing = false" @error="fail"></audio></div>
      </template>
      <div v-else class="editor-preview-fallback"><Film v-if="caps.video" :size="32" /><Music2 v-else :size="32" /><p>{{ t('mediaCard.previewUnavailable') }}</p><div class="flex flex-wrap justify-center gap-2"><a class="secondary-btn" :href="`/api/files/${file.id}/stream`" target="_blank" rel="noopener">{{ t('common.openFile') }}</a><a class="secondary-btn" :href="`/api/files/${file.id}/download`">{{ t('common.download') }}</a></div></div>
    </div>
    <div v-if="available" class="editor-playback">
      <button type="button" class="icon-btn" :disabled="!ready || busy" :aria-label="t(playing ? 'quickEdit.pause' : 'quickEdit.play')" @click="play()"><Pause v-if="playing" :size="18" /><Play v-else :size="18" /></button>
      <input class="editor-range min-w-0 flex-1" type="range" min="0" :max="caps.duration_seconds" step="0.01" :value="position" :disabled="!ready || busy" :aria-label="t('quickEdit.playhead')" @input="seek(Number(($event.target as HTMLInputElement).value))" />
      <span class="shrink-0 text-[10px] tabular-nums text-muted">{{ time(position) }} / {{ time(caps.duration_seconds) }}</span>
    </div>
    <p v-if="playError" class="error-panel m-3" role="alert">{{ t('quickEdit.playError') }}</p>
    <div class="flex flex-wrap items-center justify-between gap-2 p-3"><p class="max-w-sm text-[11px] leading-4 text-muted">{{ t('quickEdit.preview') }}</p><button v-if="available" class="secondary-btn text-xs" type="button" :disabled="!ready || busy || start >= end" @click="play(true)"><Play :size="13" />{{ t('quickEdit.playSelection') }}</button></div>
  </div>
</template>
