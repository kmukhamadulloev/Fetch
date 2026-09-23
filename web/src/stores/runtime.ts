import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { getJavaScriptRuntimes, getRuntime, startRuntimeAction, type JavaScriptRuntime, type RuntimeComponent } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useRuntimeStore = defineStore('runtime', () => {
  const components = ref<RuntimeComponent[]>([])
  const javascript = ref<JavaScriptRuntime[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  let eventRevision = 0
  const componentRevisions = new Map<string, number>()

  const ytdlp = computed(() => components.value.find((item) => item.name === 'yt-dlp'))
  const needsSetup = computed(() => ytdlp.value?.status === 'missing' || ytdlp.value?.status === 'failed')

  function merge(component: RuntimeComponent) {
    const index = components.value.findIndex((item) => item.name === component.name)
    if (index === -1) components.value.push(component)
    else components.value[index] = component
  }

  let refreshInFlight: Promise<void> | null = null
  function refresh() {
    refreshInFlight ??= loadSnapshot().finally(() => { refreshInFlight = null })
    return refreshInFlight
  }

  async function loadSnapshot() {
    const revision = eventRevision
    loading.value = true
    error.value = null
    try {
      const [snapshot, detected] = await Promise.all([getRuntime(), getJavaScriptRuntimes()])
      for (const component of snapshot) {
        if ((componentRevisions.get(component.name) ?? 0) <= revision) merge(component)
      }
      javascript.value = detected
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.runtimeUnavailable')
    } finally {
      loading.value = false
    }
  }

  async function act(component: RuntimeComponent['name'], action: 'install' | 'update' | 'repair') {
    error.value = null
    try {
      await startRuntimeAction(component, action)
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : i18n.global.t('errors.runtimeOperation')
    }
  }

  function applyEvent(payload: unknown) {
    const component = payload as RuntimeComponent
    componentRevisions.set(component.name, ++eventRevision)
    merge(component)
  }

  return { components, javascript, ytdlp, needsSetup, loading, error, refresh, act, applyEvent }
})
