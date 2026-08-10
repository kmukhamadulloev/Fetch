import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { getRuntime, startRuntimeAction, type RuntimeComponent } from '@/app/api/client'

export const useRuntimeStore = defineStore('runtime', () => {
  const components = ref<RuntimeComponent[]>([])
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
      components.value = await getRuntime()
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Runtime status is unavailable'
    } finally {
      loading.value = false
    }
  }

  async function act(component: RuntimeComponent['name'], action: 'install' | 'update' | 'repair') {
    error.value = null
    try {
      await startRuntimeAction(component, action)
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Runtime operation failed'
    }
  }

  function applyEvent(payload: unknown) { merge(payload as RuntimeComponent) }

  return { components, ytdlp, needsSetup, loading, error, refresh, act, applyEvent }
})
