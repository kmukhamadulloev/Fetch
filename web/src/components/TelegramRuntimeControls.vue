<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { RotateCcw } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import { useTelegramStore } from '@/stores/telegram'

const telegram = useTelegramStore()
const { t } = useI18n()
const now = ref(Date.now())
let timer: ReturnType<typeof setInterval> | undefined
onMounted(() => { timer = setInterval(() => { now.value = Date.now() }, 1000) })
onUnmounted(() => { clearInterval(timer) })
const status = computed(() => telegram.value?.status)
const seconds = computed(() => Math.max(0, Math.ceil(
  (Date.parse(status.value?.retry_at ?? '') - now.value) / 1000,
)) || 0)
</script>

<template>
  <div v-if="status" class="rounded-xl bg-[var(--app-surface-2)] p-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="min-w-0 text-xs" role="status">
        <p v-if="status.state === 'backing_off'">{{ t('settings.telegramRetrying', { seconds }) }}</p>
        <p v-else-if="status.state === 'error' && status.failed_attempts >= 3">{{ t('settings.telegramExhausted') }}</p>
        <p v-else-if="status.state === 'error'">{{ t('settings.actionNeeded') }}</p>
        <p v-else>{{ t(`statuses.${status.state}`) }}</p>
        <p v-if="status.error" class="mt-2 break-words text-[11px] text-red-500">{{ status.error }}</p>
      </div>
      <button
        class="secondary-btn justify-center" type="button"
        :disabled="telegram.saving || !telegram.value?.settings.enabled || !status.token_configured"
        :aria-busy="telegram.saving" @click="telegram.restart"
      >
        <RotateCcw :size="14" />{{ t('settings.restartTelegram') }}
      </button>
    </div>
    <p class="mt-2 text-[11px] text-muted">{{ t('settings.telegramRestartHelp') }}</p>
  </div>
</template>
