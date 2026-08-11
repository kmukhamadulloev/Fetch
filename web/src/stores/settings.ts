import { ref } from 'vue'
import { defineStore } from 'pinia'
import { getNetworkInfo, getSettings, putSettings, type ApplicationSettings, type NetworkInfo } from '@/app/api/client'

export function reconnectUrl(settings: ApplicationSettings, currentUrl: string) {
  const destination = new URL(currentUrl)
  destination.port = String(settings.port)
  if (!['0.0.0.0', '::'].includes(settings.bind_address)) {
    destination.hostname = settings.bind_address.includes(':') ? `[${settings.bind_address}]` : settings.bind_address
  }
  return destination.toString()
}

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
    try {
      const { listener_changed: listenerChanged, ...savedSettings } = await putSettings(settings)
      value.value = savedSettings
      error.value = null
      saved.value = true
      if (listenerChanged) {
        window.location.replace(reconnectUrl(savedSettings, window.location.href))
        return
      }
      network.value = await getNetworkInfo()
    }
    catch (cause) { error.value = cause instanceof Error ? cause.message : 'Settings could not be saved' }
    finally { saving.value = false }
  }
  return { value, network, loading, saving, error, saved, refresh, save }
})
