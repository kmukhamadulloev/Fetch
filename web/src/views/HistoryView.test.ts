import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { afterEach, expect, it, vi } from 'vitest'
import HistoryView from './HistoryView.vue'
import { useProcessesStore } from '@/stores/processes'
import { i18n, setLocale } from '@/i18n'
import type { ProcessingJob } from '@/app/api/client'

const base: ProcessingJob = { id: 'edit', kind: 'edit', source_file_id: 'source', output_file_id: null, title: 'Edited media', state: 'running', stage: 'processing', progress_percent: null, eta_seconds: null, created_at: '2026-09-24T00:00:00Z', started_at: null, updated_at: '2026-09-24T00:00:00Z', finished_at: null, error: null }
afterEach(() => vi.unstubAllGlobals())
it('merges persisted history and reacts to terminal process events without duplicating jobs', async () => {
  setLocale('en')
  const pinia = createPinia()
  vi.stubGlobal('fetch', vi.fn(async (url: string) => new Response(JSON.stringify(
    url === '/api/processes' ? [base, { ...base, id: 'metadata', kind: 'metadata', title: 'Saved tags', state: 'completed' }]
      : url === '/api/history' ? [{ id: 'download', title: 'Downloaded media', url: 'https://example.test/media', mode: 'video', status: 'completed', updated_at: '2026-09-23T00:00:00Z' }] : [],
  ))))
  const wrapper = mount(HistoryView, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  expect(wrapper.findAll('tbody tr')).toHaveLength(2)
  expect(wrapper.text()).toContain('Metadata updates')
  expect(wrapper.text()).not.toContain('Edited media')
  await wrapper.get('select').setValue('conversion')
  expect(wrapper.findAll('tbody tr')).toHaveLength(0)
  expect(wrapper.text()).not.toContain('No history yet')
  await wrapper.get('button').trigger('click')
  expect(wrapper.findAll('tbody tr')).toHaveLength(2)
  await wrapper.get('select').setValue('edit')
  const store = useProcessesStore(pinia)
  store.applyEvent({ ...base, state: 'completed', finished_at: '2026-09-24T01:00:00Z' })
  await flushPromises()
  expect(wrapper.findAll('tbody tr')).toHaveLength(1)
  expect(wrapper.find('tbody tr').text()).toContain('Edited media')
  store.applyEvent({ ...base, state: 'completed', finished_at: '2026-09-24T01:00:00Z' })
  await flushPromises()
  expect(wrapper.findAll('tbody tr')).toHaveLength(1)
  await wrapper.get('select').setValue('all')
  expect(wrapper.findAll('tbody tr')).toHaveLength(3)
  wrapper.unmount()
})
it('reports unavailable processing history instead of claiming history is empty', async () => {
  vi.stubGlobal('fetch', vi.fn(async (url: string) => url === '/api/processes'
    ? new Response(JSON.stringify({ error: { code: 'UNAVAILABLE', message: 'History unavailable' } }), { status: 503 })
    : new Response('[]')))
  const wrapper = mount(HistoryView, { global: { plugins: [createPinia(), i18n] } })
  await flushPromises()
  expect(wrapper.find('[role="alert"]').exists()).toBe(true)
  expect(wrapper.text()).not.toContain('No history yet')
  wrapper.unmount()
})
