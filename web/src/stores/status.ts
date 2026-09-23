import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getStatus, type AppStatus } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useStatusStore = defineStore('status', () => {
  const value = ref<AppStatus | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  let refreshInFlight: Promise<void> | null = null
  function refresh() {
    refreshInFlight ??= loadSnapshot().finally(() => { refreshInFlight = null })
    return refreshInFlight
  }

  async function loadSnapshot() {
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
