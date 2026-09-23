<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import AppShell from '@/layouts/AppShell.vue'
import RuntimeSetup from '@/components/RuntimeSetup.vue'
import { useStatusStore } from '@/stores/status'
import { useRuntimeStore } from '@/stores/runtime'
import { useDownloadsStore } from '@/stores/downloads'
import { useLibraryStore } from '@/stores/library'
import { useProcessesStore } from '@/stores/processes'
import { useRealtimeStore } from '@/stores/realtime'

const status = useStatusStore()
const runtime = useRuntimeStore()
const downloads = useDownloadsStore()
const library = useLibraryStore()
const processes = useProcessesStore()
const realtime = useRealtimeStore()
onMounted(async () => {
  realtime.start()
  await Promise.all([status.refresh(), runtime.refresh(), downloads.refresh(), library.refresh(), processes.refresh()])
})
onBeforeUnmount(() => realtime.stop())
</script>

<template><AppShell /><RuntimeSetup /></template>
