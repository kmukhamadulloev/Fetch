import { createRouter, createWebHistory } from 'vue-router'
import NewDownloadView from '@/views/NewDownloadView.vue'
import SettingsView from '@/views/SettingsView.vue'
import DownloadsView from '@/views/DownloadsView.vue'
import CompletedView from '@/views/CompletedView.vue'
import HistoryView from '@/views/HistoryView.vue'
import LogsView from '@/views/LogsView.vue'

export const routeMeta = {
  new: ['New download', 'Paste any URL supported by yt-dlp.'],
  downloads: ['Downloads', 'Monitor active jobs and queued items.'],
  completed: ['Completed', 'Browse and download finished files.'],
  history: ['History', 'Completed, failed and stopped jobs.'],
  settings: ['Settings', 'Application, network and runtime preferences.'],
  logs: ['Logs', 'Application, yt-dlp and FFmpeg output.'],
} as const

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'new', component: NewDownloadView, meta: { ...meta('new') } },
    { path: '/downloads', name: 'downloads', component: DownloadsView, meta: { ...meta('downloads') } },
    { path: '/completed', name: 'completed', component: CompletedView, meta: { ...meta('completed') } },
    { path: '/history', name: 'history', component: HistoryView, meta: { ...meta('history') } },
    { path: '/settings', name: 'settings', component: SettingsView, meta: { ...meta('settings') } },
    { path: '/logs', name: 'logs', component: LogsView, meta: { ...meta('logs') } },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
})

function meta(name: keyof typeof routeMeta) {
  return { title: routeMeta[name][0], subtitle: routeMeta[name][1] }
}

export default router
