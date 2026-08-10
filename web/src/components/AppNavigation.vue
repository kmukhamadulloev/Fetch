<script setup lang="ts">
import { Activity, CircleCheck, History, PlusCircle, Settings, SquareTerminal } from '@lucide/vue'

defineProps<{
  variant: 'desktop' | 'mobile'
}>()

const items = [
  { to: '/', label: 'New download', short: 'New', icon: PlusCircle },
  { to: '/downloads', label: 'Downloads', short: 'Downloads', icon: Activity },
  { to: '/completed', label: 'Completed', short: 'Files', icon: CircleCheck },
  { to: '/history', label: 'History', icon: History, desktopOnly: true },
  { to: '/settings', label: 'Settings', short: 'Settings', icon: Settings, system: true },
  { to: '/logs', label: 'Logs', icon: SquareTerminal, desktopOnly: true },
]
</script>

<template>
  <nav v-if="variant === 'desktop'" class="desktop-nav" aria-label="Primary navigation">
    <template v-for="item in items" :key="item.to">
      <div v-if="item.system" class="nav-heading">System</div>
      <RouterLink :to="item.to" class="nav-item"><component :is="item.icon" :size="16" /><span>{{ item.label }}</span></RouterLink>
    </template>
  </nav>
  <nav v-else aria-label="Primary navigation" class="mobile-bar">
    <RouterLink v-for="item in items.filter((entry) => !entry.desktopOnly)" :key="item.to" :to="item.to" class="mobile-nav">
      <component :is="item.icon" :size="18" /><span>{{ item.short }}</span>
    </RouterLink>
  </nav>
</template>
