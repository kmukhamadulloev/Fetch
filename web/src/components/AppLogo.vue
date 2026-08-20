<script setup lang="ts">
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
</script>

<template>
  <div class="logo-mark"><img src="/logo.png" alt="" /></div>
  <div><div class="text-sm font-semibold tracking-wide">Fetch</div><div class="text-[11px] text-muted">{{ t('shell.interface') }}</div></div>
</template>
