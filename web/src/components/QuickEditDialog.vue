<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
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
const caps = ref<ProcessingCapabilities>(), loading = ref(true), busy = ref(false), error = ref(''), previewFailed = ref(false)
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
  <ExportDialogFrame :title="t('quickEdit.title')" :submit-label="t('quickEdit.export')" :busy="busy" :disabled="disabled" :dirty="!!baseline && baseline !== snapshot" wide @close="emit('close')" @submit="submit">
    <p class="break-words text-sm font-semibold">{{ file.title ?? file.filename }}</p>
    <p v-if="loading" role="status" class="text-sm text-muted">{{ t('metadata.loading') }}</p>
    <div v-if="error" class="error-panel" role="alert">{{ error }}<button v-if="!caps" type="button" class="secondary-btn mt-2" @click="load">{{ t('metadata.retry') }}</button></div>
    <div v-if="caps" class="grid min-w-0 gap-5 md:grid-cols-2">
      <div class="min-w-0 space-y-3">
        <template v-if="file.browser_playable && !previewFailed"><video v-if="caps.video" class="max-h-80 w-full rounded-xl bg-black" controls playsinline preload="metadata" tabindex="0" :src="`/api/files/${file.id}/stream`" @error="previewFailed = true"></video><audio v-else class="w-full" controls preload="metadata" tabindex="0" :src="`/api/files/${file.id}/stream`" @error="previewFailed = true"></audio><p class="text-xs leading-5 text-muted">{{ t('quickEdit.preview') }}</p></template>
        <div v-else class="card space-y-3 p-4"><p class="text-sm text-muted">{{ t('mediaCard.previewUnavailable') }}</p><div class="flex flex-wrap gap-2"><a class="secondary-btn" :href="`/api/files/${file.id}/stream`" target="_blank" rel="noopener">{{ t('common.openFile') }}</a><a class="secondary-btn" :href="`/api/files/${file.id}/download`">{{ t('common.download') }}</a></div></div>
        <p class="text-xs text-muted">{{ t('exports.source', { duration: caps.duration_seconds.toFixed(2), dimensions: caps.video ? `${caps.width} × ${caps.height}` : t('exports.audio') }) }}</p>
        <p class="text-xs leading-5 text-muted">{{ t('quickEdit.trimHelp') }}</p>
      </div>
      <fieldset :disabled="busy" class="min-w-0 space-y-4">
        <label class="flex items-center gap-2 text-sm"><input v-model="trim" type="checkbox" />{{ t('quickEdit.trim') }}</label>
        <div v-if="trim" class="grid grid-cols-2 gap-3"><label class="grid min-w-0 gap-2 text-sm">{{ t('quickEdit.start') }}<input v-model.number="start" class="input min-w-0 w-full" type="number" min="0" :max="caps.duration_seconds" step="any" required /></label><label class="grid min-w-0 gap-2 text-sm">{{ t('quickEdit.end') }}<input v-model.number="end" class="input min-w-0 w-full" type="number" min="0" :max="caps.duration_seconds" step="any" required /></label></div>
        <label v-if="caps.video" class="grid gap-2 text-sm">{{ t('quickEdit.rotate') }}<select v-model.number="rotate" class="input min-w-0 w-full" :aria-label="t('quickEdit.rotate')"><option v-for="angle in [0, 90, 180, 270]" :key="angle" :value="angle">{{ angle }}°</option></select></label>
        <label v-if="caps.video && caps.audio" class="flex items-center gap-2 text-sm"><input v-model="mute" type="checkbox" />{{ t('quickEdit.mute') }}</label>
        <template v-if="caps.video">
          <label class="flex items-center gap-2 text-sm"><input v-model="cropping" type="checkbox" />{{ t('quickEdit.crop') }}</label>
          <div v-if="cropping" class="space-y-3"><p class="text-xs text-muted">{{ t('quickEdit.cropHelp') }}</p><div class="grid grid-cols-2 gap-3"><label v-for="key in (['x', 'y', 'width', 'height'] as const)" :key="key" class="grid min-w-0 gap-2 text-sm">{{ t(`quickEdit.${key}`) }}<input v-model.number="crop[key]" :aria-label="t('quickEdit.crop') + ' ' + t(`quickEdit.${key}`)" class="input min-w-0 w-full" type="number" :min="['x', 'y'].includes(key) ? 0 : 2" step="2" required /></label></div></div>
          <label class="flex items-center gap-2 text-sm"><input v-model="resizing" type="checkbox" />{{ t('quickEdit.resize') }}</label>
          <div v-if="resizing" class="space-y-3"><label class="flex items-center gap-2 text-sm"><input v-model="stretch" type="checkbox" />{{ t('quickEdit.stretch') }}</label><p class="text-xs text-muted">{{ t('quickEdit.resizeHelp') }}</p><div class="grid grid-cols-2 gap-3"><label v-for="key in (['width', 'height'] as const)" :key="key" class="grid min-w-0 gap-2 text-sm">{{ t(`quickEdit.${key}`) }}<input v-model.number="resize[key]" :aria-label="t('quickEdit.resize') + ' ' + t(`quickEdit.${key}`)" class="input min-w-0 w-full" type="number" min="2" max="7680" step="2" :readonly="key === 'height' && !stretch" required /></label></div></div>
        </template>
        <template v-if="caps.audio && !mute"><label class="flex items-center gap-2 text-sm"><input v-model="volumeEnabled" type="checkbox" />{{ t('quickEdit.volume') }}</label><label v-if="volumeEnabled" class="grid gap-2 text-sm">{{ t('quickEdit.volumePercent') }}<input v-model.number="volume" class="input min-w-0 w-full" type="number" min="0" max="400" step="any" required /><span class="text-xs text-muted">{{ t('quickEdit.volumeHelp') }}</span></label></template>
        <p v-if="!hasEdits" class="text-xs text-muted">{{ t('quickEdit.selectEdit') }}</p><p v-else-if="!validEdits" class="error-panel" role="alert">{{ t('quickEdit.invalid') }}</p>
        <p v-if="!sourceSupported" class="text-xs text-muted">{{ t('quickEdit.choose') }}</p>
        <label class="grid gap-2 text-sm">{{ t('exports.format') }}<select v-model="format" class="input min-w-0 w-full" :aria-label="t('exports.format')"><option disabled value="">{{ t('quickEdit.chooseFormat') }}</option><option v-for="item in formats" :key="item" :value="item">{{ item.toUpperCase() }}</option></select></label>
        <p v-if="!formats.length" class="error-panel">{{ t('exports.unavailable') }}</p>
        <label v-if="!['flac', 'wav'].includes(format)" class="grid gap-2 text-sm">{{ t('exports.quality') }}<select v-model="quality" class="input min-w-0 w-full" :aria-label="t('exports.quality')"><option v-for="item in ['compact', 'balanced', 'high']" :key="item" :value="item">{{ t(`exports.${item}`) }}</option></select></label>
        <label class="grid gap-2 text-sm">{{ t('exports.filename') }}<input v-model="filename" class="input min-w-0 w-full" :aria-label="t('exports.filename')" required maxlength="180" autocomplete="off" /><span class="text-xs text-muted">.{{ format }} · {{ t('exports.nameHelp') }}</span></label>
        <p class="text-xs text-muted">{{ t('exports.destination', { folder: 'Edits/' }) }}</p>
        <template v-if="caps.notices.length"><p class="text-xs leading-5 text-muted">{{ t('exports.limits') }}</p><label class="flex items-start gap-2 text-sm"><input v-model="ack" type="checkbox" class="mt-1" />{{ t('exports.acknowledge') }}</label></template>
      </fieldset>
    </div>
  </ExportDialogFrame>
</template>
