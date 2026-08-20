import { ref } from 'vue'
import { defineStore } from 'pinia'
import { analyzeMedia, type MediaInfo } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useMediaStore = defineStore('media', () => {
  const result = ref<MediaInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function analyze(url: string) {
    if (!url.trim()) {
      error.value = i18n.global.t('errors.mediaUrl')
      return
    }
    loading.value = true
    error.value = null
    result.value = null
    try {
      result.value = await analyzeMedia(url.trim())
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.mediaAnalysis')
    } finally {
      loading.value = false
    }
  }

  return { result, loading, error, analyze }
})
