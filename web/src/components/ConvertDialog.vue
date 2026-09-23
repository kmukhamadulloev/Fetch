<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { processingCapabilities, startExport, type CompletedFile, type ProcessingCapabilities, type OutputFormat, type ProcessingJob } from '@/app/api/client'
import { useProcessesStore } from '@/stores/processes'
import { safeOutputName } from '@/app/processing/validation'
import ExportDialogFrame from './ExportDialogFrame.vue'
const props = defineProps<{ file: CompletedFile }>()
const emit = defineEmits<{ close: []; accepted: [job: ProcessingJob] }>()
const { t } = useI18n()
const processes = useProcessesStore()
const caps = ref<ProcessingCapabilities>(), loading = ref(true), busy = ref(false), error = ref('')
const type = ref('video'), format = ref<OutputFormat>('mp4'), quality = ref<'compact' | 'balanced' | 'high'>('balanced'), copy = ref(false), ack = ref(false)
const filename = ref(props.file.filename.replace(/\.[^.]+$/, ''))
const baseline = ref('')
const snapshot = computed(() => JSON.stringify([type.value, format.value, quality.value, copy.value, ack.value, filename.value]))
const formats = computed(() => [...new Set([...(caps.value?.formats ?? []), ...(caps.value?.copy_formats ?? [])])].filter(item => ['mp4', 'mkv'].includes(item) === (type.value === 'video')))
const canCopy = computed(() => caps.value?.copy_formats.includes(format.value))
const disabled = computed(() => loading.value || !caps.value || !formats.value.includes(format.value) || !safeOutputName(filename.value) || (!!caps.value.notices.length && !ack.value) || (!copy.value && !caps.value.formats.includes(format.value)))
watch(type, () => { if (!formats.value.includes(format.value)) format.value = formats.value[0] ?? 'mp4' })
watch(format, () => { copy.value = !!caps.value && !caps.value.formats.includes(format.value) && !!canCopy.value })
async function load() {
  loading.value = true; error.value = ''
  try { caps.value = await processingCapabilities(props.file.id); type.value = caps.value.video ? 'video' : 'audio'; format.value = formats.value[0] ?? 'mp4'; await Promise.resolve(); baseline.value = snapshot.value }
  catch { error.value = t('exports.loadFailed') }
  finally { loading.value = false }
}
async function submit() {
  if (disabled.value || busy.value || !caps.value) return
  busy.value = true; error.value = ''
  try { const job = await startExport(props.file.id, { revision: caps.value.revision, filename: filename.value.trim(), format: format.value, quality: quality.value, stream_copy: copy.value, acknowledge_omissions: ack.value }); processes.applyEvent(job); emit('accepted', job); emit('close') }
  catch { error.value = t('exports.failed') }
  finally { busy.value = false }
}
onMounted(load)
</script>
<template>
  <ExportDialogFrame :title="t('exports.convert')" :submit-label="t('exports.start')" :busy="busy" :disabled="disabled" :dirty="!!baseline && baseline !== snapshot" @close="emit('close')" @submit="submit">
    <p class="break-words text-sm font-semibold">{{ file.title ?? file.filename }}</p>
    <p v-if="loading" role="status" class="text-sm text-muted">{{ t('metadata.loading') }}</p>
    <div v-if="error" class="error-panel" role="alert">{{ error }}<button v-if="!caps" type="button" class="secondary-btn mt-2" @click="load">{{ t('metadata.retry') }}</button></div>
    <fieldset v-if="caps" :disabled="busy" class="space-y-4">
      <p class="text-xs text-muted">{{ t('exports.source', { duration: caps.duration_seconds.toFixed(2), dimensions: caps.video ? `${caps.width} × ${caps.height}` : t('exports.audio') }) }}</p>
      <div class="grid grid-cols-2 gap-3">
        <label class="grid gap-2 text-sm">{{ t('exports.type') }}<select v-model="type" :aria-label="t('exports.type')" class="input min-w-0 w-full"><option v-if="caps.video" value="video">{{ t('exports.video') }}</option><option v-if="caps.audio" value="audio">{{ t('exports.audio') }}</option></select></label>
        <label class="grid gap-2 text-sm">{{ t('exports.format') }}<select v-model="format" :aria-label="t('exports.format')" class="input min-w-0 w-full"><option v-for="item in formats" :key="item" :value="item">{{ item.toUpperCase() }}</option></select></label>
      </div>
      <p v-if="!formats.length" class="error-panel">{{ t('exports.unavailable') }}</p>
      <label v-if="canCopy" class="flex items-center gap-2 text-sm"><input v-model="copy" type="checkbox" :disabled="!caps.formats.includes(format)" />{{ t('exports.copy') }}</label>
      <p class="text-xs leading-5 text-muted">{{ t(copy ? 'exports.copyHelp' : 'exports.encode') }}</p>
      <label v-if="!copy && !['flac', 'wav'].includes(format)" class="grid gap-2 text-sm">{{ t('exports.quality') }}<select v-model="quality" :aria-label="t('exports.quality')" class="input min-w-0 w-full"><option v-for="item in ['compact', 'balanced', 'high']" :key="item" :value="item">{{ t(`exports.${item}`) }}</option></select></label>
      <label class="grid gap-2 text-sm">{{ t('exports.filename') }}<input v-model="filename" :aria-label="t('exports.filename')" class="input min-w-0 w-full" required maxlength="180" autocomplete="off" /><span class="text-xs text-muted">.{{ format }}</span></label>
      <p class="text-xs text-muted">{{ t('exports.destination', { folder: 'Converted/' }) }}</p>
      <template v-if="caps.notices.length"><p class="text-xs leading-5 text-muted">{{ t('exports.limits') }}</p><label class="flex items-start gap-2 text-sm"><input v-model="ack" type="checkbox" class="mt-1" />{{ t('exports.acknowledge') }}</label></template>
    </fieldset>
  </ExportDialogFrame>
</template>
