import { createRouter, createWebHistory } from 'vue-router'
import NewDownloadView from '@/views/NewDownloadView.vue'
import SettingsView from '@/views/SettingsView.vue'
import ProcessesView from '@/views/ProcessesView.vue'
import CompletedView from '@/views/CompletedView.vue'
import HistoryView from '@/views/HistoryView.vue'
import LogsView from '@/views/LogsView.vue'

export const routeMeta = {
  new: ['routes.newTitle', 'routes.newSubtitle'],
  downloads: ['processing.title', 'processing.subtitle'],
  completed: ['routes.completedTitle', 'routes.completedSubtitle'],
  history: ['routes.historyTitle', 'routes.historySubtitle'],
  settings: ['routes.settingsTitle', 'routes.settingsSubtitle'],
  logs: ['routes.logsTitle', 'routes.logsSubtitle'],
} as const

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'new', component: NewDownloadView, meta: { ...meta('new') } },
    { path: '/processes', name: 'downloads', component: ProcessesView, meta: { ...meta('downloads') } },
    { path: '/downloads', redirect: to => ({ path: '/processes', query: to.query, hash: to.hash }) },
    { path: '/completed', name: 'completed', component: CompletedView, meta: { ...meta('completed') } },
    { path: '/history', name: 'history', component: HistoryView, meta: { ...meta('history') } },
    { path: '/settings', name: 'settings', component: SettingsView, meta: { ...meta('settings') } },
    { path: '/logs', name: 'logs', component: LogsView, meta: { ...meta('logs') } },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
})

function meta(name: keyof typeof routeMeta) {
  return { titleKey: routeMeta[name][0], subtitleKey: routeMeta[name][1] }
}

export default router
