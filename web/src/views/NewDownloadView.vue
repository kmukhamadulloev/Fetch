<script setup lang="ts">
import { computed, ref } from 'vue'
import { Clock3, Cpu, Link2, ListVideo, ScanSearch, ShieldCheck } from '@lucide/vue'
import { useMediaStore } from '@/stores/media'
import { useRuntimeStore } from '@/stores/runtime'
import { useDownloadsStore } from '@/stores/downloads'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

const url = ref('')
const media = useMediaStore()
const runtime = useRuntimeStore()
const downloads = useDownloadsStore()
const router = useRouter()
const { t } = useI18n()
const mode = ref<'video' | 'audio'>('video')
const formatId = ref('')
const container = ref('auto')
const quality = ref('best')
const videoCodec = ref('auto')
const audioCodec = ref('auto')
const outputDirectory = ref('')
const embedMetadata = ref(true)
const embedThumbnail = ref(true)
const subtitles = ref(false)
const adding = ref(false)
const addError = ref<string | null>(null)
const duration = computed(() => {
  const seconds = media.result?.duration_seconds
  if (seconds == null) return null
  const minutes = Math.floor(seconds / 60)
  return `${minutes}:${String(Math.floor(seconds % 60)).padStart(2, '0')}`
})
const videoFormats = computed(() => media.result?.formats.filter((format) => format.has_video) ?? [])
const audioFormats = computed(() => media.result?.formats.filter((format) => format.has_audio) ?? [])
const selectableFormats = computed(() => mode.value === 'video' ? videoFormats.value : audioFormats.value)

function fallbackPlaylistId() {
  return globalThis.crypto?.randomUUID?.() ?? `playlist-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`
}

async function addDownload() {
  if (!media.result) return
  adding.value = true
  addError.value = null
  try {
    const common = {
      mode: mode.value,
      format_id: formatId.value || null,
      quality: quality.value,
      container: container.value,
      video_codec: videoCodec.value,
      audio_codec: audioCodec.value,
      embed_metadata: embedMetadata.value,
      embed_thumbnail: embedThumbnail.value,
      subtitles: subtitles.value,
      output_directory: outputDirectory.value || null,
    }
    if (media.result.kind === 'playlist') {
      const entries = media.result.entries.filter((entry) => entry.url)
      if (entries.length === 0) throw new Error(t('newDownload.noPlaylistUrls'))
      const playlistId = media.result.id ?? fallbackPlaylistId()
      for (const [index, entry] of entries.entries()) {
        await downloads.add({
          ...common,
          url: entry.url!,
          title: entry.title,
          duration_seconds: entry.duration_seconds,
          playlist: { id: playlistId, title: media.result.title, index: index + 1 },
        })
      }
    } else {
      await downloads.add({
        ...common,
        url: media.result.webpage_url ?? url.value,
        title: media.result.title,
        duration_seconds: media.result.duration_seconds,
      })
    }
    await router.push('/processes')
  } catch (cause) {
    addError.value = cause instanceof Error ? cause.message : t('newDownload.addError')
  } finally { adding.value = false }
}
</script>

<template>
  <section class="mx-auto max-w-5xl">
    <div class="mb-7">
      <p class="eyebrow">{{ t('newDownload.eyebrow') }}</p>
      <h2 class="mt-2 text-2xl font-semibold tracking-tight sm:text-3xl">{{ t('newDownload.title') }}</h2>
      <p class="mt-2 max-w-2xl text-sm leading-6 text-muted">{{ t('newDownload.description') }}</p>
    </div>
    <form class="card p-4 sm:p-5" @submit.prevent="media.analyze(url)">
      <div class="flex flex-col gap-3 sm:flex-row">
        <label class="relative flex-1"><span class="sr-only">{{ t('newDownload.mediaUrl') }}</span><Link2 class="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-zinc-600" :size="16" /><input v-model="url" class="input h-12 pl-10 font-mono text-xs sm:text-sm" placeholder="https://example.com/video-or-playlist" /></label>
        <button class="primary-btn h-12 px-5" type="submit" :disabled="media.loading || runtime.ytdlp?.status !== 'ready'"><ScanSearch :size="16" />{{ media.loading ? t('newDownload.analyzing') : t('newDownload.analyze') }}</button>
      </div>
      <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-[11px] text-muted">
        <span class="helper"><ShieldCheck :size="14" />{{ t('newDownload.handled') }}</span>
        <span class="helper"><ListVideo :size="14" />{{ t('newDownload.kinds') }}</span>
        <span class="helper"><Cpu :size="14" />{{ t('newDownload.runtime', { value: runtime.ytdlp?.version ?? runtime.ytdlp?.status ?? t('newDownload.checking') }) }}</span>
      </div>
    </form>
    <p v-if="media.error" class="error-panel mt-5">{{ media.error }}</p>
    <div v-if="media.loading" class="card mt-5 animate-pulse p-5"><div class="h-28 rounded-xl bg-zinc-800/70"></div></div>
    <div v-else-if="media.result" class="mt-5 space-y-5">
      <article class="card overflow-hidden">
        <div class="flex flex-col sm:flex-row">
          <div class="relative aspect-video w-full shrink-0 overflow-hidden bg-zinc-900 sm:w-[300px]">
            <img v-if="media.result.thumbnail_url" class="size-full object-cover" :src="media.result.thumbnail_url" alt="" />
          </div>
          <div class="min-w-0 flex-1 p-5">
            <div class="flex flex-wrap gap-2"><span class="badge">{{ media.result.extractor ?? 'yt-dlp' }}</span><span class="badge muted">{{ media.result.kind === 'playlist' ? t('newDownload.itemCount', { count: media.result.playlist_count ?? media.result.entries.length }) : t('newDownload.singleMedia') }}</span></div>
            <h3 class="mt-3 text-lg font-semibold">{{ media.result.title }}</h3>
            <div class="mt-4 flex flex-wrap gap-4 text-xs text-muted"><span v-if="duration" class="helper"><Clock3 :size="14" />{{ duration }}</span><span>{{ t('newDownload.videoFormats', { count: videoFormats.length }) }}</span><span>{{ t('newDownload.audioFormats', { count: audioFormats.length }) }}</span></div>
          </div>
        </div>
      </article>
      <article v-if="media.result.kind === 'playlist'" class="card p-5"><div class="flex flex-wrap items-center justify-between gap-3"><div><h3 class="text-sm font-semibold">{{ t('newDownload.playlistEntries') }}</h3><p class="mt-1 text-[11px] text-muted">{{ t('newDownload.playlistHelp') }}</p></div><span class="badge muted">{{ t('newDownload.itemCount', { count: media.result.entries.length }) }}</span></div><div class="mt-4 max-h-72 divide-y divide-border overflow-auto"><div v-for="(entry, index) in media.result.entries" :key="entry.id ?? index" class="flex items-center gap-3 py-3 text-xs"><span class="w-7 text-muted">{{ String(index + 1).padStart(3, '0') }}</span><img v-if="entry.thumbnail_url" class="size-9 rounded-md object-cover" :src="entry.thumbnail_url" alt="" referrerpolicy="no-referrer" /><span>{{ entry.title }}</span></div></div></article>
      <article class="card p-5"><h3 class="text-sm font-semibold">{{ t('newDownload.availableFormats') }}</h3><div class="mt-4 grid gap-2 sm:grid-cols-2"><div v-for="format in media.result.formats.slice(0, 20)" :key="format.id" class="format-row"><span class="font-medium">{{ format.label || format.id }}</span><span class="text-muted">{{ [format.extension, format.video_codec, format.audio_codec].filter(Boolean).join(' · ') }}</span></div></div><p v-if="media.result.formats.length > 20" class="mt-3 text-xs text-muted">{{ t('newDownload.additionalFormats', { count: media.result.formats.length - 20 }) }}</p></article>
      <article class="card p-5">
        <div class="segmented"><button type="button" class="segment" :class="{ active: mode === 'video' }" @click="mode = 'video'; formatId = ''">{{ t('common.video') }}</button><button type="button" class="segment" :class="{ active: mode === 'audio' }" @click="mode = 'audio'; formatId = ''">{{ t('common.audio') }}</button></div>
        <div class="mt-5 grid gap-4 sm:grid-cols-2">
          <label class="field"><span>{{ t('newDownload.format') }}</span><select v-model="formatId" class="select"><option value="">{{ t('newDownload.bestAvailable') }}</option><option v-for="format in selectableFormats" :key="format.id" :value="format.id">{{ format.label || format.id }}</option></select></label>
          <label v-if="mode === 'video'" class="field"><span>{{ t('newDownload.container') }}</span><select v-model="container" class="select"><option value="auto">{{ t('newDownload.auto') }}</option><option value="mp4">MP4</option><option value="mkv">MKV</option><option value="webm">WebM</option></select></label>
          <label v-if="mode === 'video'" class="field"><span>{{ t('newDownload.maximumQuality') }}</span><select v-model="quality" class="select"><option value="best">{{ t('newDownload.bestAvailable') }}</option><option value="2160p">2160p</option><option value="1440p">1440p</option><option value="1080p">1080p</option><option value="720p">720p</option><option value="480p">480p</option></select></label>
          <label v-if="mode === 'video'" class="field"><span>{{ t('newDownload.videoCodec') }}</span><select v-model="videoCodec" class="select"><option value="auto">{{ t('newDownload.auto') }}</option><option value="h264">H.264</option><option value="av1">AV1</option><option value="vp9">VP9</option></select></label>
          <label v-else class="field"><span>{{ t('newDownload.audioFormat') }}</span><select v-model="audioCodec" class="select"><option value="auto">{{ t('newDownload.bestSource') }}</option><option value="m4a">M4A</option><option value="mp3">MP3</option><option value="opus">Opus</option><option value="flac">FLAC</option></select></label>
          <label class="field sm:col-span-2"><span>{{ t('newDownload.outputDirectory') }} <span class="text-muted">{{ t('newDownload.outputBlank') }}</span></span><input v-model="outputDirectory" class="input font-mono text-xs" :placeholder="t('newDownload.applicationDefault')" /></label>
        </div>
        <div class="mt-5 grid gap-2 sm:grid-cols-3"><label class="check-row"><input v-model="embedMetadata" type="checkbox" />{{ t('newDownload.embedMetadata') }}</label><label class="check-row"><input v-model="embedThumbnail" type="checkbox" />{{ t('newDownload.embedThumbnail') }}</label><label class="check-row"><input v-model="subtitles" type="checkbox" />{{ t('newDownload.subtitles') }}</label></div>
        <p v-if="addError || downloads.error" class="error-panel mt-4">{{ addError ?? downloads.error }}</p>
        <div class="mt-5 flex justify-end"><button class="primary-btn" type="button" :disabled="adding" @click="addDownload">{{ adding ? t('newDownload.adding') : media.result.kind === 'playlist' ? t('newDownload.addItems', { count: media.result.entries.length }) : t('newDownload.addDownload') }}</button></div>
      </article>
    </div>
  </section>
</template>
