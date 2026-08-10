import { ref } from 'vue'
import { defineStore } from 'pinia'
import { analyzeMedia, type MediaInfo } from '@/app/api/client'

export const useMediaStore = defineStore('media', () => {
  const result = ref<MediaInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function analyze(url: string) {
    if (!url.trim()) {
      error.value = 'Paste a media URL first'
      return
    }
    loading.value = true
    error.value = null
    result.value = null
    try {
      result.value = await analyzeMedia(url.trim())
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Media analysis failed'
    } finally {
      loading.value = false
    }
  }

  return { result, loading, error, analyze }
})
