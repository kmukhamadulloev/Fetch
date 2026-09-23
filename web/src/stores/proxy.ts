import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getProxySettings, putProxySettings, type ProxySettings } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useProxyStore = defineStore('proxy', () => {
  const value = ref<ProxySettings | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const saved = ref(false)
  const error = ref<string | null>(null)

  let refreshInFlight: Promise<void> | null = null
  function refresh() {
    refreshInFlight ??= loadSnapshot().finally(() => { refreshInFlight = null })
    return refreshInFlight
  }

  async function loadSnapshot() {
    loading.value = true
    try {
      value.value = await getProxySettings()
      error.value = null
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.proxyLoad')
    } finally {
      loading.value = false
    }
  }

  async function save(settings: ProxySettings): Promise<boolean> {
    saving.value = true
    saved.value = false
    try {
      value.value = await putProxySettings(settings)
      error.value = null
      saved.value = true
      window.setTimeout(() => { saved.value = false }, 1800)
      return true
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.proxySave')
      return false
    } finally {
      saving.value = false
    }
  }

  return { value, loading, saving, saved, error, refresh, save }
})
