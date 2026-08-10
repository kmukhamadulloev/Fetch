import { ref } from 'vue'
import { defineStore } from 'pinia'
import { getNetworkInfo, getSettings, putSettings, type ApplicationSettings, type NetworkInfo } from '@/app/api/client'

export const useSettingsStore = defineStore('settings', () => {
  const value = ref<ApplicationSettings | null>(null)
  const network = ref<NetworkInfo | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<string | null>(null)
  const saved = ref(false)
  async function refresh() {
    loading.value = true
    try { [value.value, network.value] = await Promise.all([getSettings(), getNetworkInfo()]); error.value = null }
    catch (cause) { error.value = cause instanceof Error ? cause.message : 'Settings are unavailable' }
    finally { loading.value = false }
  }
  async function save(settings: ApplicationSettings) {
    saving.value = true; saved.value = false
    try { value.value = await putSettings(settings); network.value = await getNetworkInfo(); error.value = null; saved.value = true }
    catch (cause) { error.value = cause instanceof Error ? cause.message : 'Settings could not be saved' }
    finally { saving.value = false }
  }
  return { value, network, loading, saving, error, saved, refresh, save }
})
