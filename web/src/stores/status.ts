import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getStatus, type AppStatus } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useStatusStore = defineStore('status', () => {
  const value = ref<AppStatus | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      value.value = await getStatus()
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.serverUnavailable')
    } finally {
      loading.value = false
    }
  }

  return { value, loading, error, refresh }
})
