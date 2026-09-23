import { ref } from 'vue'
import { defineStore } from 'pinia'
import {
  deleteTelegramToken, getTelegramIntegration, putTelegramSettings, putTelegramToken,
  restartTelegram, testTelegramConnection, type TelegramIntegration, type TelegramSettings,
  type TelegramStatus,
} from '@/app/api/client'
import { i18n } from '@/i18n'

export const useTelegramStore = defineStore('telegram', () => {
  const value = ref<TelegramIntegration | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<string | null>(null)
  const saved = ref(false)
  let statusRevision = 0
  let latestStatus: TelegramStatus | null = null

  function applySnapshot(snapshot: TelegramIntegration, revision: number) {
    value.value = statusRevision !== revision && latestStatus
      ? { ...snapshot, status: latestStatus } : snapshot
  }

  async function run(operation: () => Promise<TelegramIntegration>) {
    const revision = statusRevision
    saving.value = true
    saved.value = false
    try {
      applySnapshot(await operation(), revision)
      error.value = null
      saved.value = true
      return true
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.telegramUpdate')
      return false
    } finally { saving.value = false }
  }

  let refreshInFlight: Promise<void> | null = null
  function refresh() {
    refreshInFlight ??= loadSnapshot().finally(() => { refreshInFlight = null })
    return refreshInFlight
  }

  async function loadSnapshot() {
    const revision = statusRevision
    loading.value = true
    try { applySnapshot(await getTelegramIntegration(), revision); error.value = null }
    catch (cause) { error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.telegramUnavailable') }
    finally { loading.value = false }
  }

  function applyStatus(status: TelegramStatus) {
    statusRevision += 1
    latestStatus = status
    if (value.value) value.value = { ...value.value, status }
  }

  const saveSettings = (settings: TelegramSettings) => run(() => putTelegramSettings(settings))
  const saveToken = (token: string) => run(() => putTelegramToken(token))
  const removeToken = () => run(deleteTelegramToken)
  const restart = () => run(restartTelegram)
  const test = () => run(testTelegramConnection)

  return { value, loading, saving, error, saved, refresh, applyStatus, saveSettings, saveToken, removeToken, test, restart }
})
