<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import QrcodeVue from 'qrcode.vue'
import { Check, Copy, QrCode, Smartphone, TriangleAlert, X } from '@lucide/vue'
import { useSettingsStore } from '@/stores/settings'

const emit = defineEmits<{ close: [] }>()
const settings = useSettingsStore()
const panel = ref<HTMLElement | null>(null)
const selected = ref('')
const copied = ref(false)

const lanUrls = computed(() => (settings.network?.urls ?? []).filter((value) => {
  try {
    const hostname = new URL(value).hostname
    return hostname !== 'localhost' && hostname !== '127.0.0.1' && hostname !== '::1' && hostname !== '[::1]'
  } catch { return false }
}))

watch(lanUrls, (urls) => {
  if (urls.length === 0) selected.value = ''
  else if (!urls.includes(selected.value)) {
    selected.value = urls.find((url) => new URL(url).hostname === window.location.hostname) ?? urls[0]
  }
}, { immediate: true })

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close')
}

async function copyUrl() {
  if (!selected.value) return
  try {
    await navigator.clipboard.writeText(selected.value)
    copied.value = true
    window.setTimeout(() => { copied.value = false }, 1_500)
  } catch {
    settings.error = 'Could not copy the LAN address. Select and copy it manually.'
  }
}

function hostname(url: string) {
  try { return new URL(url).hostname }
  catch { return url }
}

onMounted(async () => {
  document.addEventListener('keydown', onKeydown)
  if (!settings.network) await settings.refresh()
  await nextTick()
  panel.value?.focus()
})
onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="network-qr-title">
    <button class="modal-backdrop" type="button" aria-label="Close mobile access" @click="emit('close')"></button>
    <div ref="panel" class="modal-panel max-w-md" tabindex="-1">
      <div class="flex items-start justify-between gap-4">
        <div class="flex items-center gap-3"><div class="empty-icon"><QrCode :size="20" /></div><div><h2 id="network-qr-title" class="text-base font-semibold">Open Fetch on mobile</h2><p class="mt-1 text-xs text-muted">Scan while connected to the same local network.</p></div></div>
        <button class="icon-btn shrink-0" type="button" aria-label="Close mobile access" @click="emit('close')"><X :size="16" /></button>
      </div>

      <div v-if="settings.loading" class="mt-6 flex min-h-64 items-center justify-center text-xs text-muted">Finding reachable LAN addresses…</div>
      <div v-else-if="selected" class="mt-6">
        <div class="mx-auto w-fit rounded-2xl bg-white p-4 shadow-xl">
          <QrcodeVue :value="selected" :size="220" level="M" render-as="svg" foreground="#071015" background="#ffffff" />
        </div>
        <div class="mt-5 flex gap-2">
          <code class="input min-w-0 flex-1 select-all truncate text-xs">{{ selected }}</code>
          <button class="secondary-btn shrink-0" type="button" @click="copyUrl"><Check v-if="copied" :size="15" /><Copy v-else :size="15" />{{ copied ? 'Copied' : 'Copy' }}</button>
        </div>
        <div v-if="lanUrls.length > 1" class="mt-3 flex flex-wrap gap-2">
          <button v-for="url in lanUrls" :key="url" class="badge" :class="{ muted: selected !== url }" type="button" @click="selected = url">{{ hostname(url) }}</button>
        </div>
        <div class="warning-panel mt-5"><TriangleAlert class="shrink-0 text-amber-500" :size="18" /><p class="text-[11px] leading-5">Fetch has no application login. Anyone allowed on this network can control downloads and access completed media.</p></div>
      </div>
      <div v-else class="mt-6 rounded-xl border border-border bg-panel-2 p-6 text-center">
        <Smartphone class="mx-auto text-muted" :size="36" />
        <h3 class="mt-4 text-sm font-semibold">No reachable LAN address</h3>
        <p class="mt-2 text-xs leading-5 text-muted">Bind Fetch to a LAN interface and allow your local CIDR in Network settings. Fetch applies the new listener when you save.</p>
        <RouterLink class="secondary-btn mt-4" to="/settings#network" @click="emit('close')">Open Network settings</RouterLink>
      </div>
      <p v-if="settings.error" class="error-panel mt-4" role="alert">{{ settings.error }}</p>
    </div>
  </div>
</template>
