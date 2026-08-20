import { ref } from 'vue'
import { defineStore } from 'pinia'
import {
  deleteTelegramToken, getTelegramIntegration, putTelegramSettings, putTelegramToken,
  testTelegramConnection, type TelegramIntegration, type TelegramSettings,
  type TelegramStatus,
} from '@/app/api/client'

export const useTelegramStore = defineStore('telegram', () => {
  const value = ref<TelegramIntegration | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<string | null>(null)
  const saved = ref(false)

  async function run(operation: () => Promise<TelegramIntegration>) {
    saving.value = true
    saved.value = false
    try {
      value.value = await operation()
      error.value = null
      saved.value = true
      return true
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Telegram settings could not be updated'
      return false
    } finally { saving.value = false }
  }

  async function refresh() {
    loading.value = true
    try { value.value = await getTelegramIntegration(); error.value = null }
    catch (cause) { error.value = cause instanceof Error ? cause.message : 'Telegram integration is unavailable' }
    finally { loading.value = false }
  }

  function applyStatus(status: TelegramStatus) {
    if (value.value) value.value = { ...value.value, status }
  }

  const saveSettings = (settings: TelegramSettings) => run(() => putTelegramSettings(settings))
  const saveToken = (token: string) => run(() => putTelegramToken(token))
  const removeToken = () => run(deleteTelegramToken)
  const test = () => run(testTelegramConnection)

  return { value, loading, saving, error, saved, refresh, applyStatus, saveSettings, saveToken, removeToken, test }
})
