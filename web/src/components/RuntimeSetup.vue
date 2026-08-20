<script setup lang="ts">
import { computed } from 'vue'
import { CheckCircle2, Download, RefreshCw, TriangleAlert } from '@lucide/vue'
import { useRuntimeStore } from '@/stores/runtime'
import { useI18n } from 'vue-i18n'

const runtime = useRuntimeStore()
const { t } = useI18n()
const current = computed(() => runtime.ytdlp)
const busy = computed(() => current.value?.status === 'installing' || current.value?.status === 'updating')
</script>

<template>
  <div v-if="runtime.needsSetup || busy" class="modal" role="dialog" aria-modal="true" aria-labelledby="runtime-setup-title">
    <div class="modal-backdrop"></div>
    <div class="modal-panel max-w-lg">
      <div class="logo-mark"><img src="/logo.png" alt="" /></div>
      <h2 id="runtime-setup-title" class="mt-5 text-xl font-semibold">{{ t('runtimeSetup.title') }}</h2>
      <p class="mt-2 text-sm leading-6 text-muted">{{ t('runtimeSetup.description') }}</p>
      <div class="runtime-card mt-6">
        <div class="flex items-center gap-3">
          <div class="empty-icon"><Download :size="19" /></div>
          <div class="min-w-0 flex-1"><div class="text-sm font-semibold">yt-dlp</div><div class="mt-1 text-xs text-muted">{{ current?.version ?? current?.status ?? t('runtimeSetup.checking') }}</div></div>
          <CheckCircle2 v-if="current?.status === 'ready'" class="text-emerald-300" :size="20" />
          <RefreshCw v-else-if="busy" class="animate-spin text-accent" :size="20" />
          <TriangleAlert v-else class="text-amber-300" :size="20" />
        </div>
        <div v-if="busy && current?.progress_percent != null" class="progress mt-4"><div :style="{ width: `${current.progress_percent}%` }"></div></div>
      </div>
      <p v-if="runtime.error || current?.error" class="error-panel mt-4">{{ runtime.error ?? current?.error }}</p>
      <button class="primary-btn mt-5 w-full py-3" type="button" :disabled="busy" @click="runtime.act('yt-dlp', current?.status === 'failed' ? 'repair' : 'install')">
        <RefreshCw v-if="busy" class="animate-spin" :size="16" />{{ busy ? t('runtimeSetup.installing') : current?.status === 'failed' ? t('runtimeSetup.repair') : t('runtimeSetup.install') }}
      </button>
    </div>
  </div>
</template>
