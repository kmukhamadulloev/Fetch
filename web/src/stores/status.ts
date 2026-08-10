import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getStatus, type AppStatus } from '@/app/api/client'

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
      error.value = cause instanceof Error ? cause.message : 'Fetch server is unavailable'
    } finally {
      loading.value = false
    }
  }

  return { value, loading, error, refresh }
})
