<script setup lang="ts">
import { ref, useId } from 'vue'
import { ArrowDownUp, CalendarDays, CaseSensitive, ChevronDown } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

const sortBy = defineModel<string>('sortBy', { required: true })
const reverse = defineModel<boolean>('reverse', { required: true })
const { t } = useI18n()
const id = useId()
const trigger = ref<HTMLButtonElement | null>(null)
const open = ref(false)
const position = ref({ top: '0px', left: '0px' })

function positionPanel(event: Event) {
  const opening = (event as ToggleEvent).newState === 'open'
  open.value = opening
  if (!opening || !trigger.value) return
  const rect = trigger.value.getBoundingClientRect()
  position.value = {
    top: `${Math.max(8, Math.min(rect.bottom + 8, window.innerHeight - 190))}px`,
    left: `${Math.max(8, Math.min(rect.right - 208, window.innerWidth - 216))}px`,
  }
}
</script>

<template>
  <button ref="trigger" type="button" class="secondary-btn min-h-10 whitespace-nowrap" :popovertarget="id" :aria-expanded="open" :aria-controls="id">
    <ArrowDownUp :size="15" aria-hidden="true" />
    {{ t('browse.sortBy') }}: {{ t(`browse.${sortBy}`) }}
    <ChevronDown :size="14" aria-hidden="true" :class="{ 'rotate-180': open }" />
  </button>
  <div :id="id" popover class="sort-popover" :style="position" @beforetoggle="positionPanel">
    <fieldset>
      <legend class="px-3 pb-2 pt-1 text-[10px] font-semibold uppercase tracking-wider text-muted">{{ t('browse.sortBy') }}</legend>
      <label v-for="option in ['name', 'date']" :key="option" class="sort-option" :class="{ selected: sortBy === option }">
        <component :is="option === 'name' ? CaseSensitive : CalendarDays" :size="15" aria-hidden="true" />
        <span class="flex-1">{{ t(`browse.${option}`) }}</span>
        <input v-model="sortBy" type="radio" :name="id" :value="option" class="size-4 accent-[var(--app-accent)]" />
      </label>
    </fieldset>
    <div class="mt-1 border-t border-border pt-1">
      <label class="sort-option">
        <ArrowDownUp :size="15" aria-hidden="true" />
        <span class="flex-1">{{ t('browse.reverse') }}</span>
        <input v-model="reverse" type="checkbox" class="size-4 accent-[var(--app-accent)]" />
      </label>
    </div>
  </div>
</template>

<style scoped>
.sort-popover { position: fixed; margin: 0; width: 208px; max-width: calc(100vw - 16px); padding: .4rem; border: 1px solid var(--app-border-strong); border-radius: .8rem; background: var(--app-input); color: var(--app-text); box-shadow: 0 12px 35px #0006; }
.sort-option { display: flex; min-height: 2.5rem; align-items: center; gap: .6rem; padding: .5rem .65rem; border-radius: .5rem; font-size: .75rem; cursor: pointer; }
.sort-option:hover, .sort-option:focus-within, .sort-option.selected { background: color-mix(in srgb, var(--app-accent) 12%, transparent); }
.sort-option.selected { color: var(--app-accent); }
</style>
