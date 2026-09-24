<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { X, LoaderCircle } from '@lucide/vue'
const props = defineProps<{ title: string; submitLabel: string; busy: boolean; dirty: boolean; disabled: boolean; wide?: boolean }>()
const emit = defineEmits<{ close: []; submit: [] }>()
const { t } = useI18n()
const panel = ref<HTMLFormElement>()
const discard = ref(false)
const keepEditing = ref<HTMLButtonElement>()
let previous: HTMLElement | null = null
let overflow = ''
async function close() { if (!props.busy) { if (props.dirty) { discard.value = true; await nextTick(); keepEditing.value?.focus() } else emit('close') } }
function unload(event: BeforeUnloadEvent) { if (props.dirty || props.busy) event.preventDefault() }
onBeforeRouteLeave(() => { if (props.busy || props.dirty) { close(); return false } })
function keydown(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close() }
  if (event.key !== 'Tab') return
  const items = [...(panel.value?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), summary, a[href], [tabindex="0"]') ?? [])].filter(item => item.getClientRects().length)
  if (!items.length) { event.preventDefault(); panel.value?.focus(); return }
  const first = items[0], last = items.at(-1)
  if (event.shiftKey && (document.activeElement === first || document.activeElement === panel.value)) { event.preventDefault(); last?.focus() }
  else if (!event.shiftKey && (document.activeElement === last || document.activeElement === panel.value)) { event.preventDefault(); first?.focus() }
}
onMounted(async () => { previous = document.activeElement as HTMLElement; overflow = document.body.style.overflow; document.body.style.overflow = 'hidden'; window.addEventListener('beforeunload', unload); await nextTick(); panel.value?.focus() })
onUnmounted(() => { document.body.style.overflow = overflow; window.removeEventListener('beforeunload', unload); previous?.focus() })
</script>
<template>
  <Teleport to="body">
    <div class="modal metadata-modal" role="dialog" aria-modal="true" aria-labelledby="export-title" @keydown="keydown">
      <div class="modal-backdrop" @click="close"></div>
      <form ref="panel" class="modal-panel metadata-panel" :style="wide ? { maxWidth: '68rem' } : undefined" tabindex="-1" :aria-busy="busy" @submit.prevent="!disabled && !busy && emit('submit')">
        <header class="flex shrink-0 items-center justify-between gap-4 border-b border-white/10 p-5"><h2 id="export-title" class="text-lg font-semibold">{{ title }}</h2><button type="button" class="icon-btn" :aria-label="t('common.close')" :disabled="busy" @click="close"><X :size="18" /></button></header>
        <div class="min-h-0 space-y-4 overflow-y-auto p-5"><slot /><div v-if="discard" class="error-panel" role="alert"><p>{{ t('metadata.discardPrompt') }}</p><div class="mt-3 flex gap-2"><button ref="keepEditing" type="button" class="secondary-btn" @click="discard = false">{{ t('metadata.keepEditing') }}</button><button type="button" class="danger-btn" @click="emit('close')">{{ t('metadata.discard') }}</button></div></div></div>
        <footer class="flex shrink-0 justify-end gap-2 border-t border-white/10 p-5"><button type="button" class="secondary-btn" :disabled="busy" @click="close">{{ t('common.cancel') }}</button><button class="primary-btn" type="submit" :disabled="disabled || busy"><LoaderCircle v-if="busy" :size="16" class="animate-spin" />{{ submitLabel }}</button></footer>
      </form>
    </div>
  </Teleport>
</template>
