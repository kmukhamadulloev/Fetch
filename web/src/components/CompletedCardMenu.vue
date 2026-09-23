<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { EllipsisVertical } from '@lucide/vue'
defineProps<{ id: string; name: string }>()
const emit = defineEmits<{ metadata: []; convert: [] }>()
const { t } = useI18n()
const open = ref(false), root = ref<HTMLElement>(), trigger = ref<HTMLButtonElement>()
function close(focus = false) { open.value = false; if (focus) trigger.value?.focus() }
async function toggle() { open.value = !open.value; if (open.value) { await nextTick(); root.value?.querySelector<HTMLElement>('[role="menuitem"]')?.focus() } }
function outside(event: PointerEvent) { if (!root.value?.contains(event.target as Node)) close() }
function keydown(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(true) }
  if (!open.value || !['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const items = [...(root.value?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])]
  const current = items.indexOf(document.activeElement as HTMLElement)
  items[event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (current + (event.key === 'ArrowUp' ? -1 : 1) + items.length) % items.length]?.focus()
}
onMounted(() => document.addEventListener('pointerdown', outside))
onUnmounted(() => document.removeEventListener('pointerdown', outside))
</script>
<template>
  <div ref="root" @click.stop @keydown="keydown" @focusout="(event) => { if (!root?.contains(event.relatedTarget as Node)) close() }">
    <button ref="trigger" class="metadata-pencil" type="button" :aria-label="t('exports.actions', { name })" aria-haspopup="menu" :aria-controls="`file-menu-${id}`" :aria-expanded="open" @click="toggle" @keydown.down.stop.prevent="!open && toggle()"><EllipsisVertical :size="17" /></button>
    <div v-if="open" :id="`file-menu-${id}`" role="menu" class="absolute left-2.5 top-14 z-20 grid min-w-44 gap-1 rounded-xl border border-white/15 bg-[#171d27] p-1.5 shadow-xl">
      <button role="menuitem" tabindex="-1" class="rounded-lg px-3 py-2 text-left text-sm hover:bg-white/10" @click="close(true); emit('metadata')">{{ t('metadata.edit') }}</button>
      <button role="menuitem" tabindex="-1" class="rounded-lg px-3 py-2 text-left text-sm hover:bg-white/10" @click="close(true); emit('convert')">{{ t('exports.convert') }}</button>
    </div>
  </div>
</template>
