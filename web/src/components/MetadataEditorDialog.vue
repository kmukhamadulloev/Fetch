<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ChevronDown, ImagePlus, LoaderCircle, Trash2, X } from '@lucide/vue'
import { getMetadata, getMetadataStatus, saveMetadata, type ArtworkUpdate, type CompletedFile, type MediaMetadata, type MetadataSaveStatus } from '@/app/api/client'
import { useLibraryStore } from '@/stores/library'

const props = defineProps<{ file: CompletedFile }>()
const emit = defineEmits<{ close: [] }>()
const { t } = useI18n()
const library = useLibraryStore()
const panel = ref<HTMLElement | null>(null)
const metadata = ref<MediaMetadata | null>(null)
const fields = ref<Record<string, string>>({})
const artwork = ref<ArtworkUpdate>({ action: 'keep' })
const uploadPreview = ref('')
const previewFailed = ref(false)
const loading = ref(true)
const saving = ref(false)
const readingArtwork = ref(false)
const error = ref('')
const advanced = ref(false)
const discard = ref(false)
const operationId = ref<string | null>(null)
let timer: ReturnType<typeof setTimeout> | undefined
let disposed = false
let previousFocus: HTMLElement | null = null
let previousOverflow = ''
let completing = false
const basicNames = computed(() => metadata.value?.media_type === 'audio' ? ['title', 'artist', 'album', 'album_artist', 'track'] : ['title', 'artist', 'description'])
const basic = computed(() => basicNames.value.filter((key) => metadata.value?.supported_fields.includes(key)))
const additional = computed(() => (metadata.value?.supported_fields ?? []).filter((key) => !basicNames.value.includes(key)))
const dirty = computed(() => artwork.value.action !== 'keep' || Object.entries(fields.value).some(([key, value]) => value !== (metadata.value?.fields[key] ?? '')))
const preview = computed(() => artwork.value.action === 'remove' ? '' : uploadPreview.value || (metadata.value?.artwork_available ? `/api/files/${props.file.id}/metadata/artwork?v=${encodeURIComponent(metadata.value.revision)}` : props.file.thumbnail_available ? library.thumbnailUrl(props.file) : ''))
watch(preview, () => { previewFailed.value = false })
function fieldLabel(key: string) { return t(`metadata.fields.${key === 'artist' && metadata.value?.media_type === 'video' ? 'creator' : key}`) }
function numeric(key: string) { return ['track', 'track_total', 'disc', 'disc_total'].includes(key) }
function close() {
  if (saving.value || readingArtwork.value) return
  if (dirty.value) { discard.value = true; return }
  emit('close')
}
function beforeUnload(event: BeforeUnloadEvent) {
  if (dirty.value || saving.value) { event.preventDefault(); event.returnValue = '' }
}
onBeforeRouteLeave(() => {
  if (saving.value) return false
  if (dirty.value) { discard.value = true; return false }
  return true
})
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close() }
  if (event.key === 'Tab' && panel.value) {
    const controls = [...panel.value.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), textarea:not(:disabled), summary, [tabindex="0"]')].filter((node) => node.getClientRects().length)
    if (!controls.length) { event.preventDefault(); panel.value.focus(); return }
    const first = controls[0]; const last = controls.at(-1)
    if (event.shiftKey && (document.activeElement === first || document.activeElement === panel.value)) { event.preventDefault(); last?.focus() }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus() }
  }
}
async function load() {
  loading.value = true; error.value = ''
  try {
    const status = await getMetadataStatus(props.file.id)
    if (status.state === 'saving') { saving.value = true; operationId.value = status.operation_id; schedulePoll(); return }
    metadata.value = await getMetadata(props.file.id)
    fields.value = { ...metadata.value.fields }
  } catch (cause) { error.value = cause instanceof Error ? cause.message : t('metadata.loadFailed') }
  finally { loading.value = false }
}
async function upload(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  error.value = ''
  if (!['image/jpeg', 'image/png'].includes(file.type) || file.size > 8 * 1024 * 1024) { error.value = t('metadata.artworkLimit'); input.value = ''; return }
  readingArtwork.value = true
  try {
    const data = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(String(reader.result))
      reader.onerror = () => reject(new Error(t('metadata.artworkFailed')))
      reader.readAsDataURL(file)
    })
    if (disposed) return
    uploadPreview.value = data
    artwork.value = { action: 'replace', data: data.slice(data.indexOf(',') + 1) }
  } catch (cause) { error.value = cause instanceof Error ? cause.message : t('metadata.artworkFailed') }
  finally { readingArtwork.value = false; input.value = '' }
}
function removeArtwork() { artwork.value = { action: 'remove' }; uploadPreview.value = '' }
async function handleStatus(status: MetadataSaveStatus) {
  if (!saving.value || completing || status.operation_id !== operationId.value) return
  if (status.state === 'completed') {
    completing = true
    if (timer) clearTimeout(timer)
    await library.refresh()
    library.thumbnailVersions[props.file.id] = Date.now()
    saving.value = false
    if (!disposed) emit('close')
  } else if (status.state === 'failed') {
    saving.value = false; error.value = status.error ?? t('metadata.saveFailed')
    if (timer) clearTimeout(timer)
  } else if (status.state === 'idle') {
    saving.value = false; error.value = t('metadata.interrupted')
  }
}
function schedulePoll() {
  if (disposed || !saving.value) return
  timer = setTimeout(async () => {
    try {
      const status = await getMetadataStatus(props.file.id)
      // A restarted server no longer has the previous in-memory operation.
      if (status.state === 'idle') { saving.value = false; error.value = t('metadata.interrupted') }
      else await handleStatus(status)
    } catch (cause) { error.value = cause instanceof Error ? cause.message : t('metadata.saveFailed') }
    schedulePoll()
  }, 1000)
}
watch(() => library.metadataStatuses[props.file.id], (status) => { if (status) void handleStatus(status) })
async function save() {
  if (!metadata.value || saving.value || readingArtwork.value || !dirty.value) return
  saving.value = true; error.value = ''
  try {
    const changed = Object.fromEntries(Object.entries(fields.value).filter(([key, value]) => value !== (metadata.value?.fields[key] ?? '')))
    const status = await saveMetadata(props.file.id, { revision: metadata.value.revision, fields: changed, artwork: artwork.value })
    operationId.value = status.operation_id
    const latest = library.metadataStatuses[props.file.id]
    if (latest) await handleStatus(latest)
    schedulePoll()
  } catch (cause) {
    // The request may have reached the server even if its response was lost.
    try {
      const status = await getMetadataStatus(props.file.id)
      if (status.state === 'saving') { operationId.value = status.operation_id; schedulePoll(); return }
    } catch { /* Keep the original request failure visible. */ }
    saving.value = false; error.value = cause instanceof Error ? cause.message : t('metadata.saveFailed')
  }
}
onMounted(async () => {
  previousFocus = document.activeElement as HTMLElement | null
  previousOverflow = document.body.style.overflow
  document.body.style.overflow = 'hidden'
  window.addEventListener('beforeunload', beforeUnload)
  await nextTick(); panel.value?.focus()
  await load()
})
onBeforeUnmount(() => {
  disposed = true
  if (timer) clearTimeout(timer)
  window.removeEventListener('beforeunload', beforeUnload)
  document.body.style.overflow = previousOverflow
  previousFocus?.focus()
})
</script>

<template>
  <Teleport to="body">
    <div class="modal metadata-modal" role="dialog" aria-modal="true" aria-labelledby="metadata-title" @keydown="onKeydown">
      <div class="modal-backdrop" @click="close"></div>
      <form ref="panel" class="modal-panel metadata-panel" tabindex="-1" :aria-busy="saving || loading" @submit.prevent="save">
        <header class="flex shrink-0 items-start justify-between gap-4 border-b border-border p-5">
          <div class="min-w-0"><h2 id="metadata-title" class="text-lg font-semibold">{{ t('metadata.edit') }}</h2><p class="mt-1 truncate text-xs text-muted">{{ file.filename }}</p></div>
          <button class="icon-btn shrink-0" type="button" :disabled="saving || readingArtwork" :aria-label="t('common.close')" @click="close"><X :size="18" /></button>
        </header>
        <div class="min-h-0 overflow-y-auto p-5">
          <p v-if="loading" class="flex items-center gap-2 text-sm" role="status"><LoaderCircle class="animate-spin" :size="18" />{{ t('metadata.loading') }}</p>
          <template v-else-if="metadata">
            <p v-if="!metadata.editable" class="warning-panel mb-4">{{ t('metadata.unsupported') }}</p>
            <fieldset :disabled="saving || readingArtwork || !metadata.editable" class="min-w-0">
              <div class="mb-5 flex flex-wrap items-center gap-4">
                <img v-if="preview && !previewFailed" :src="preview" :alt="t('metadata.artwork')" class="size-28 rounded-xl border border-border object-contain" @error="previewFailed = true; error = t('metadata.artworkFailed')" />
                <div v-else class="flex size-28 items-center justify-center rounded-xl border border-border bg-panel-2"><ImagePlus :size="32" class="text-muted" /></div>
                <div class="min-w-0 flex-1"><p class="text-sm font-medium">{{ t('metadata.artwork') }}</p><p class="mt-1 text-xs text-muted">{{ t('metadata.artworkLimit') }}</p><p v-if="!metadata.artwork_available && artwork.action === 'keep' && file.thumbnail_available" class="mt-1 text-xs text-muted">{{ t('metadata.cachedArtwork') }}</p><div v-if="metadata.artwork_editable" class="mt-3 flex flex-wrap gap-2">
                  <label class="secondary-btn relative cursor-pointer"><ImagePlus :size="15" />{{ t('metadata.upload') }}<input class="absolute inset-0 w-full cursor-pointer opacity-0" type="file" accept="image/jpeg,image/png" :aria-label="t('metadata.upload')" @change="upload" /></label>
                  <button class="secondary-btn" type="button" :disabled="!preview" @click="removeArtwork"><Trash2 :size="15" />{{ t('metadata.remove') }}</button>
                </div></div>
              </div>
              <div class="grid min-w-0 gap-4 sm:grid-cols-2">
                <label v-for="key in basic" :key="key" class="min-w-0 text-xs" :class="{ 'sm:col-span-2': key === 'description' }"><span class="mb-2 block text-muted">{{ fieldLabel(key) }}</span><textarea v-if="key === 'description'" v-model="fields[key]" class="input min-h-24 w-full" maxlength="16384"></textarea><input v-else v-model="fields[key]" class="input w-full" type="text" :inputmode="numeric(key) ? 'numeric' : undefined" :pattern="numeric(key) ? '[0-9]+' : undefined" :min="numeric(key) ? 1 : undefined" :max="numeric(key) ? 65535 : undefined" maxlength="16384" /></label>
              </div>
              <button v-if="additional.length" class="mt-5 flex w-full items-center justify-between rounded-xl border border-border p-3 text-sm" type="button" :aria-expanded="advanced" aria-controls="metadata-advanced" @click="advanced = !advanced">{{ t('metadata.advanced') }}<ChevronDown :size="17" :class="{ 'rotate-180': advanced }" /></button>
              <div v-if="advanced" id="metadata-advanced" class="mt-4 grid min-w-0 gap-4 sm:grid-cols-2">
                <label v-for="key in additional" :key="key" class="min-w-0 text-xs"><span class="mb-2 block text-muted">{{ fieldLabel(key) }}</span><textarea v-if="['comment', 'description'].includes(key)" v-model="fields[key]" class="input min-h-20 w-full" maxlength="16384"></textarea><input v-else v-model="fields[key]" class="input w-full" type="text" :inputmode="numeric(key) ? 'numeric' : undefined" :pattern="numeric(key) ? '[0-9]+' : undefined" :min="numeric(key) ? 1 : undefined" :max="numeric(key) ? 65535 : undefined" maxlength="16384" /></label>
              </div>
            </fieldset>
            <details class="mt-5 rounded-xl border border-border p-3 text-xs"><summary class="cursor-pointer font-medium">{{ t('metadata.information') }}</summary><dl class="mt-3 space-y-2 break-words text-muted"><div>{{ t('metadata.container') }}: {{ metadata.container }}</div><div v-for="(value, key) in metadata.information" :key="key"><dt class="font-medium">{{ String(key).startsWith('stream_') ? t('metadata.stream', { number: String(key).slice(7) }) : t(`metadata.info.${key}`) }}</dt><dd>{{ value }}</dd></div></dl></details>
          </template>
          <p v-if="error" class="error-panel mt-4" role="alert">{{ error }}</p>
          <button v-if="!metadata && !loading && !saving" class="secondary-btn mt-3" type="button" @click="load">{{ t('metadata.retry') }}</button>
          <p v-if="saving" class="mt-4 flex items-center gap-2 text-sm text-muted" role="status"><LoaderCircle class="shrink-0 animate-spin" :size="18" />{{ t('metadata.saving') }}</p>
          <div v-if="discard" class="warning-panel mt-4 flex-wrap" role="alert"><p class="w-full">{{ t('metadata.discardPrompt') }}</p><button class="secondary-btn" type="button" @click="discard = false">{{ t('metadata.keepEditing') }}</button><button class="danger-btn" type="button" @click="emit('close')">{{ t('metadata.discard') }}</button></div>
        </div>
        <footer class="flex shrink-0 justify-end gap-2 border-t border-border p-4">
          <button class="secondary-btn" type="button" :disabled="saving || readingArtwork" @click="close">{{ t('common.cancel') }}</button>
          <button class="primary-btn" type="submit" :disabled="loading || saving || readingArtwork || !metadata?.editable || !dirty"><LoaderCircle v-if="saving" class="animate-spin" :size="16" />{{ t('metadata.save') }}</button>
        </footer>
      </form>
    </div>
  </Teleport>
</template>
