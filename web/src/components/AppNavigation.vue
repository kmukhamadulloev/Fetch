<script setup lang="ts">
import { Activity, CircleCheck, History, PlusCircle, Settings, SquareTerminal } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

defineProps<{
  variant: 'desktop' | 'mobile'
}>()

const { t } = useI18n()
const items = [
  { to: '/', label: 'nav.newDownload', short: 'nav.newShort', icon: PlusCircle },
  { to: '/processes', label: 'processing.title', short: 'processing.title', icon: Activity },
  { to: '/completed', label: 'nav.completed', short: 'nav.files', icon: CircleCheck },
  { to: '/history', label: 'nav.history', icon: History, desktopOnly: true },
  { to: '/settings', label: 'nav.settings', short: 'nav.settings', icon: Settings, system: true },
  { to: '/logs', label: 'nav.logs', icon: SquareTerminal, desktopOnly: true },
]
</script>

<template>
  <nav v-if="variant === 'desktop'" class="desktop-nav" :aria-label="t('nav.primary')">
    <template v-for="item in items" :key="item.to">
      <div v-if="item.system" class="nav-heading">{{ t('nav.system') }}</div>
      <RouterLink :to="item.to" class="nav-item"><component :is="item.icon" :size="16" /><span>{{ t(item.label) }}</span></RouterLink>
    </template>
  </nav>
  <nav v-else :aria-label="t('nav.primary')" class="mobile-bar">
    <RouterLink v-for="item in items.filter((entry) => !entry.desktopOnly)" :key="item.to" :to="item.to" class="mobile-nav">
      <component :is="item.icon" :size="18" /><span>{{ t(item.short ?? item.label) }}</span>
    </RouterLink>
  </nav>
</template>
