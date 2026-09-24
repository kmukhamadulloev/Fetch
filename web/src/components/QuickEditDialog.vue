<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
import { Scissors, Crop, Volume2, RotateCw, RotateCcw, Undo2, SlidersHorizontal, Check, ArrowRight } from '@lucide/vue'
import { centeredCrop } from '@/app/processing/editor'
import QuickEditPreview from './QuickEditPreview.vue'
import { useI18n } from 'vue-i18n'
import { processingCapabilities, startExport, type CompletedFile, type ProcessingCapabilities, type OutputFormat, type ProcessingJob, type QuickEdits } from '@/app/api/client'
import { useProcessesStore } from '@/stores/processes'
import { safeOutputName, evenDimension, validCrop, proportionalHeight } from '@/app/processing/validation'
import { processingErrorKey } from '@/app/processing/errors'
import ExportDialogFrame from './ExportDialogFrame.vue'
const props = defineProps<{ file: CompletedFile }>()
const emit = defineEmits<{ close: []; accepted: [job: ProcessingJob] }>()
const { t } = useI18n()
const processes = useProcessesStore()
const caps = ref<ProcessingCapabilities>(), loading = ref(true), busy = ref(false), error = ref('')
const format = ref<OutputFormat | ''>(''), quality = ref<'compact' | 'balanced' | 'high'>('balanced'), ack = ref(false)
const filename = ref(props.file.filename.replace(/\.[^.]+$/, ''))
const trim = ref(false), start = ref(0), end = ref(0), rotate = ref(0), mute = ref(false)
const cropping = ref(false), resizing = ref(false), stretch = ref(false), volumeEnabled = ref(false), volume = ref(100)
const crop = reactive({ x: 0, y: 0, width: 0, height: 0 })
const resize = reactive({ width: 0, height: 0 })
const baseDimensions = computed(() => {
  const width = cropping.value ? crop.width : caps.value?.width ?? 0
  const height = cropping.value ? crop.height : caps.value?.height ?? 0
  return rotate.value % 180 ? { width: height, height: width } : { width, height }
})
watch([() => resize.width, baseDimensions, stretch], () => {
  if (!stretch.value) resize.height = proportionalHeight(resize.width, baseDimensions.value.width, baseDimensions.value.height)
})
watch(resizing, enabled => { if (enabled) { resize.width = baseDimensions.value.width; resize.height = baseDimensions.value.height } })
const tool = ref<'trim' | 'frame' | 'audio'>('trim')
const exportOpen = ref(false), position = ref(0), previewReady = ref(false)
const tools = computed(() => ['trim', ...(caps.value?.video ? ['frame'] : []), ...(caps.value?.audio ? ['audio'] : [])])
const toolIcons = { trim: Scissors, frame: Crop, audio: Volume2 }
const selectedDuration = computed(() => Math.max(0, (trim.value ? end.value - start.value : caps.value?.duration_seconds) ?? 0))
const outputDimensions = computed(() => resizing.value ? resize : baseDimensions.value)
const editCount = computed(() => [trim.value, rotate.value !== 0, mute.value, cropping.value, resizing.value, volumeEnabled.value && !mute.value].filter(Boolean).length)
function time(value: number) { return `${Math.floor(value / 60)}:${(value % 60).toFixed(2).padStart(5, '0')}` }
function setBoundary(which: 'start' | 'end', value: number) {
  trim.value = true
  if (which === 'start') start.value = Math.max(0, Math.min(end.value - 0.01, value))
  else end.value = Math.min(caps.value?.duration_seconds ?? 0, Math.max(start.value + 0.01, value))
}
function cropPreset(ratio: number) { cropping.value = true; Object.assign(crop, centeredCrop(caps.value?.width ?? 0, caps.value?.height ?? 0, ratio)) }
async function resizePreset(scale: number) {
  resizing.value = true
  await nextTick()
  stretch.value = false
  resize.width = Math.max(2, Math.min(7680, Math.round(baseDimensions.value.width * scale / 2) * 2))
}
function resetEdits() {
  trim.value = false; start.value = 0; end.value = caps.value?.duration_seconds ?? 0
  rotate.value = 0; mute.value = false; cropping.value = false; resizing.value = false; stretch.value = false; volumeEnabled.value = false; volume.value = 100; resize.width = 0; resize.height = 0
  Object.assign(crop, { x: 0, y: 0, width: caps.value?.width ?? 0, height: caps.value?.height ?? 0 })
}
function tabKey(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const index = tools.value.indexOf(tool.value)
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? tools.value.length - 1 : (index + (event.key === 'ArrowRight' ? 1 : -1) + tools.value.length) % tools.value.length
  tool.value = tools.value[next] as typeof tool.value
  document.getElementById(`editor-tab-${tool.value}`)?.focus()
}
async function reviewExport() {
  if (exportOpen.value) { await submit(); return }
  exportOpen.value = true
  await nextTick()
  document.getElementById('editor-output-name')?.focus()
}
const baseline = ref('')
const edits = computed<QuickEdits>(() => ({ rotate: rotate.value, mute: mute.value, ...(trim.value ? { start_seconds: start.value, end_seconds: end.value } : {}), ...(cropping.value ? { crop: { ...crop } } : {}), ...(resizing.value ? { resize: { ...resize } } : {}), ...(volumeEnabled.value && !mute.value ? { volume: volume.value / 100 } : {}) }))
const snapshot = computed(() => JSON.stringify([format.value, quality.value, ack.value, filename.value, trim.value, start.value, end.value, edits.value, cropping.value, resizing.value, stretch.value, volumeEnabled.value, volume.value, crop, resize]))
const formats = computed(() => (caps.value?.formats ?? []).filter(item => ['mp4', 'mkv'].includes(item) === !!caps.value?.video))
const sourceSupported = computed(() => formats.value.includes(props.file.filename.split('.').at(-1)?.toLowerCase() as OutputFormat))
const hasEdits = computed(() => trim.value || rotate.value !== 0 || mute.value || cropping.value || resizing.value || (volumeEnabled.value && !mute.value))
const validEdits = computed(() => (!trim.value || (Number.isFinite(start.value) && Number.isFinite(end.value) && start.value >= 0 && end.value > start.value && end.value <= (caps.value?.duration_seconds ?? 0)))
  && (!cropping.value || validCrop(crop, caps.value?.width ?? 0, caps.value?.height ?? 0))
  && (!resizing.value || (evenDimension(resize.width) && evenDimension(resize.height)))
  && (!volumeEnabled.value || mute.value || (Number.isFinite(volume.value) && volume.value >= 0 && volume.value <= 400)))
const disabled = computed(() => loading.value || !caps.value || !format.value || !formats.value.includes(format.value) || !safeOutputName(filename.value) || (!!caps.value.notices.length && !ack.value) || !hasEdits.value || !validEdits.value)
async function load() {
  loading.value = true; error.value = ''
  try { caps.value = await processingCapabilities(props.file.id); end.value = caps.value.duration_seconds; crop.width = caps.value.width ?? 0; crop.height = caps.value.height ?? 0; const source = props.file.filename.split('.').at(-1)?.toLowerCase() as OutputFormat; format.value = formats.value.includes(source) ? source : ''; await nextTick(); baseline.value = snapshot.value }
  catch { error.value = t('exports.loadFailed') }
  finally { loading.value = false }
}
async function submit() {
  if (disabled.value || busy.value || !caps.value || !format.value) return
  busy.value = true; error.value = ''
  try { const job = await startExport(props.file.id, { revision: caps.value.revision, filename: filename.value.trim(), format: format.value, quality: quality.value, stream_copy: false, acknowledge_omissions: ack.value, edits: edits.value }); processes.applyEvent(job); emit('accepted', job); emit('close') }
  catch (cause) { error.value = t(processingErrorKey(cause)) }
  finally { busy.value = false }
}
onMounted(load)
</script>
<template>
  <ExportDialogFrame :title="t('quickEdit.title')" :submit-label="t(exportOpen ? 'quickEdit.export' : 'quickEdit.review')" :busy="busy" :disabled="exportOpen ? disabled : loading || !caps || !hasEdits || !validEdits" :dirty="!!baseline && baseline !== snapshot" wide @close="emit('close')" @submit="reviewExport">
    <div class="editor-source-heading"><div class="min-w-0"><p class="truncate text-sm font-semibold" :title="file.title ?? file.filename">{{ file.title ?? file.filename }}</p><p class="mt-1 text-[11px] text-muted">{{ t('quickEdit.originalSafe') }}</p></div><button type="button" class="secondary-btn shrink-0" :disabled="busy || !hasEdits" @click="resetEdits"><Undo2 :size="14" />{{ t('quickEdit.reset') }}</button></div>
    <p v-if="loading" role="status" class="text-sm text-muted">{{ t('metadata.loading') }}</p>
    <div v-if="error" class="error-panel" role="alert">{{ error }}<button v-if="!caps" type="button" class="secondary-btn mt-2" @click="load">{{ t('metadata.retry') }}</button></div>
    <div v-if="caps" class="editor-workspace">
      <div class="editor-viewer">
        <QuickEditPreview :file="file" :caps="caps" :crop="crop" :cropping="cropping && tool === 'frame' && validCrop(crop, caps.width ?? 0, caps.height ?? 0)" :busy="busy" :start="trim ? start : 0" :end="trim ? end : caps.duration_seconds" @update:crop="Object.assign(crop, $event)" @position="position = $event" @ready="previewReady = $event" />
        <div class="editor-summary" aria-live="polite"><span><Scissors :size="13" />{{ time(selectedDuration) }}</span><span v-if="caps.video">{{ outputDimensions.width }} × {{ outputDimensions.height }}</span><span v-if="rotate">{{ rotate }}°</span><span v-if="mute">{{ t('quickEdit.mute') }}</span><span>{{ t('quickEdit.editCount', { count: editCount }) }}</span></div>
      </div>
      <div class="editor-inspector">
        <div class="editor-tools" role="tablist" :aria-label="t('quickEdit.tools')" @keydown="tabKey"><button v-for="item in tools" :id="`editor-tab-${item}`" :key="item" type="button" role="tab" :aria-selected="tool === item" :aria-controls="`editor-panel-${item}`" :tabindex="tool === item ? 0 : -1" :disabled="busy" @click="tool = item as typeof tool"><component :is="toolIcons[item as keyof typeof toolIcons]" :size="16" />{{ t(`quickEdit.tool_${item}`) }}<span v-if="item === 'trim' ? trim : item === 'frame' ? cropping || resizing || rotate : mute || volumeEnabled" class="editor-tool-dot"></span></button></div>
        <fieldset :id="`editor-panel-${tool}`" :disabled="busy" class="editor-tool-panel" role="tabpanel" :aria-labelledby="`editor-tab-${tool}`">
          <template v-if="tool === 'trim'">
            <label class="editor-toggle"><span>{{ t('quickEdit.trim') }}</span><input v-model="trim" type="checkbox" /></label>
            <p class="editor-hint">{{ t('quickEdit.trimGesture') }}</p>
            <div class="editor-timeline" :class="{ inactive: !trim }"><div class="editor-timeline-track"><div :style="{ left: `${start / caps.duration_seconds * 100}%`, right: `${100 - end / caps.duration_seconds * 100}%` }"></div></div><input type="range" min="0" :max="caps.duration_seconds" step="0.01" :value="start" :aria-label="t('quickEdit.trimStart')" @input="setBoundary('start', Number(($event.target as HTMLInputElement).value))" /><input type="range" min="0" :max="caps.duration_seconds" step="0.01" :value="end" :aria-label="t('quickEdit.trimEnd')" @input="setBoundary('end', Number(($event.target as HTMLInputElement).value))" /></div>
            <div class="flex justify-between text-xs tabular-nums text-muted"><span>{{ time(start) }}</span><span>{{ time(end) }}</span></div>
            <div class="grid grid-cols-2 gap-2"><button class="secondary-btn" type="button" :disabled="!previewReady" @click="setBoundary('start', position)">{{ t('quickEdit.markIn') }}</button><button class="secondary-btn" type="button" :disabled="!previewReady" @click="setBoundary('end', position)">{{ t('quickEdit.markOut') }}</button></div>
            <div v-if="trim" class="grid grid-cols-2 gap-3"><label class="editor-field">{{ t('quickEdit.start') }}<input v-model.number="start" class="input" type="number" min="0" :max="caps.duration_seconds" step="any" required /></label><label class="editor-field">{{ t('quickEdit.end') }}<input v-model.number="end" class="input" type="number" min="0" :max="caps.duration_seconds" step="any" required /></label></div>
            <p class="editor-hint">{{ t('quickEdit.trimHelp') }}</p>
          </template>
          <template v-if="tool === 'frame'">
            <div class="flex items-center justify-between gap-2"><span class="text-xs font-semibold">{{ t('quickEdit.rotation') }} <span class="ml-2 text-accent">{{ rotate }}°</span></span><div class="flex gap-2"><button type="button" class="secondary-btn" :aria-label="t('quickEdit.rotateLeft')" @click="rotate = (rotate + 270) % 360"><RotateCcw :size="17" /></button><button type="button" class="secondary-btn" :aria-label="t('quickEdit.rotate')" @click="rotate = (rotate + 90) % 360"><RotateCw :size="17" /></button></div></div>
            <div class="editor-divider"></div>
            <label class="editor-toggle"><span>{{ t('quickEdit.crop') }}</span><input v-model="cropping" type="checkbox" /></label>
            <div class="editor-presets" :aria-label="t('quickEdit.cropPresets')" role="group"><button v-for="preset in [{ label: '16:9', ratio: 16 / 9 }, { label: '1:1', ratio: 1 }, { label: '9:16', ratio: 9 / 16 }, { label: '4:3', ratio: 4 / 3 }]" :key="preset.label" type="button" class="secondary-btn" @click="cropPreset(preset.ratio)">{{ preset.label }}</button></div>
            <template v-if="cropping"><p class="editor-hint">{{ t('quickEdit.cropGesture') }}</p><details class="editor-precision"><summary>{{ t('quickEdit.preciseCrop') }}</summary><div class="mt-3 grid grid-cols-2 gap-3"><label v-for="key in (['x', 'y', 'width', 'height'] as const)" :key="key" class="editor-field">{{ t(`quickEdit.${key}`) }}<input v-model.number="crop[key]" :aria-label="t('quickEdit.crop') + ' ' + t(`quickEdit.${key}`)" class="input" type="number" :min="['x', 'y'].includes(key) ? 0 : 2" step="2" required /></label></div></details></template>
            <div class="editor-divider"></div>
            <label class="editor-toggle"><span>{{ t('quickEdit.resize') }}</span><input v-model="resizing" type="checkbox" /></label>
            <div class="editor-presets" role="group" :aria-label="t('quickEdit.resizePresets')"><button v-for="scale in [1, 0.75, 0.5, 0.25]" :key="scale" class="secondary-btn" type="button" @click="resizePreset(scale)">{{ scale * 100 }}%</button></div>
            <details v-if="resizing" class="editor-precision"><summary>{{ t('quickEdit.customSize') }}</summary><div class="mt-3 space-y-3"><label class="flex items-start gap-2 text-xs"><input v-model="stretch" type="checkbox" />{{ t('quickEdit.stretch') }}</label><div class="grid grid-cols-2 gap-3"><label v-for="key in (['width', 'height'] as const)" :key="key" class="editor-field">{{ t(`quickEdit.${key}`) }}<input v-model.number="resize[key]" :aria-label="t('quickEdit.resize') + ' ' + t(`quickEdit.${key}`)" class="input" type="number" min="2" max="7680" step="2" :readonly="key === 'height' && !stretch" required /></label></div><p class="editor-hint">{{ t('quickEdit.resizeHelp') }}</p></div></details>
          </template>
          <template v-if="tool === 'audio'">
            <label v-if="caps.video" class="editor-toggle"><span>{{ t('quickEdit.mute') }}</span><input v-model="mute" type="checkbox" /></label>
            <template v-if="!mute"><label class="editor-toggle"><span>{{ t('quickEdit.volume') }}</span><input v-model="volumeEnabled" type="checkbox" /></label><div class="editor-volume-value">{{ volumeEnabled ? volume : 100 }}<span>%</span></div><input class="editor-range w-full" type="range" min="0" max="400" step="5" :value="volume" :aria-label="t('quickEdit.volumeSlider')" @input="volumeEnabled = true; volume = Number(($event.target as HTMLInputElement).value)" /><div class="flex justify-between text-[10px] text-muted"><span>0%</span><span>400%</span></div><div class="editor-presets"><button v-for="level in [50, 100, 150, 200]" :key="level" class="secondary-btn" type="button" @click="volumeEnabled = true; volume = level">{{ level }}%</button></div><label v-if="volumeEnabled" class="editor-field">{{ t('quickEdit.volumePercent') }}<input v-model.number="volume" class="input" type="number" min="0" max="400" step="any" required /></label><p class="editor-hint">{{ t('quickEdit.volumeHelp') }}</p></template>
            <p v-else class="editor-hint">{{ t('quickEdit.muteHelp') }}</p>
          </template>
        </fieldset>
        <p v-if="!hasEdits" class="editor-hint px-4 pb-4">{{ t('quickEdit.selectEdit') }}</p><p v-else-if="!validEdits" class="error-panel m-3" role="alert">{{ t('quickEdit.invalid') }}</p>
      </div>
    </div>
    <details v-if="caps" class="editor-export" :open="exportOpen" @toggle="exportOpen = ($event.target as HTMLDetailsElement).open"><summary><SlidersHorizontal :size="16" /><span>{{ t('quickEdit.exportSettings') }}</span><span class="ml-auto text-[11px] text-muted">{{ format.toUpperCase() || '—' }} · Edits/</span><ArrowRight :size="14" /></summary><fieldset :disabled="busy" class="space-y-4 p-4">
      <p v-if="!sourceSupported" class="editor-hint">{{ t('quickEdit.choose') }}</p>
      <div class="grid gap-3 sm:grid-cols-2"><label class="editor-field">{{ t('exports.format') }}<select v-model="format" class="select" :aria-label="t('exports.format')"><option disabled value="">{{ t('quickEdit.chooseFormat') }}</option><option v-for="item in formats" :key="item" :value="item">{{ item.toUpperCase() }}</option></select></label><label v-if="!['flac', 'wav'].includes(format)" class="editor-field">{{ t('exports.quality') }}<select v-model="quality" class="select" :aria-label="t('exports.quality')"><option v-for="item in ['compact', 'balanced', 'high']" :key="item" :value="item">{{ t(`exports.${item}`) }}</option></select></label></div>
      <p v-if="!formats.length" class="error-panel">{{ t('exports.unavailable') }}</p>
      <label class="editor-field">{{ t('exports.filename') }}<input id="editor-output-name" v-model="filename" class="input" :aria-label="t('exports.filename')" required maxlength="180" autocomplete="off" /><span class="editor-hint">.{{ format }} · {{ t('exports.nameHelp') }}</span></label>
      <p class="flex items-center gap-2 text-xs text-muted"><Check :size="14" class="text-accent" />{{ t('exports.destination', { folder: 'Edits/' }) }}</p>
      <template v-if="caps.notices.length"><p class="editor-hint">{{ t('exports.limits') }}</p><label class="flex items-start gap-2 text-xs"><input v-model="ack" type="checkbox" class="mt-0.5" />{{ t('exports.acknowledge') }}</label></template>
    </fieldset></details>
  </ExportDialogFrame>
</template>
