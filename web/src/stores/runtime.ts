import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { getJavaScriptRuntimes, getRuntime, startRuntimeAction, type JavaScriptRuntime, type RuntimeComponent } from '@/app/api/client'
import { i18n } from '@/i18n'

export const useRuntimeStore = defineStore('runtime', () => {
  const components = ref<RuntimeComponent[]>([])
  const javascript = ref<JavaScriptRuntime[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const ytdlp = computed(() => components.value.find((item) => item.name === 'yt-dlp'))
  const needsSetup = computed(() => ytdlp.value?.status === 'missing' || ytdlp.value?.status === 'failed')

  function merge(component: RuntimeComponent) {
    const index = components.value.findIndex((item) => item.name === component.name)
    if (index === -1) components.value.push(component)
    else components.value[index] = component
  }

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      ;[components.value, javascript.value] = await Promise.all([getRuntime(), getJavaScriptRuntimes()])
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

  function applyEvent(payload: unknown) { merge(payload as RuntimeComponent) }

  return { components, javascript, ytdlp, needsSetup, loading, error, refresh, act, applyEvent }
})
