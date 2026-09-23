import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { useProcessesStore } from './processes'
import type { ProcessingJob } from '@/app/api/client'
const job: ProcessingJob = { id: 'process', kind: 'conversion', source_file_id: 'source', output_file_id: null, title: 'Media', state: 'queued', stage: 'queued', progress_percent: null, eta_seconds: null, created_at: '2026-09-23T00:00:00Z', started_at: null, updated_at: '2026-09-23T00:00:00Z', finished_at: null, error: null }
beforeEach(() => setActivePinia(createPinia()))
afterEach(() => vi.unstubAllGlobals())
it('deduplicates reads and preserves newer realtime state over a late snapshot', async () => {
  let respond!: (response: Response) => void
  const fetch = vi.fn(() => new Promise<Response>(resolve => { respond = resolve }))
  vi.stubGlobal('fetch', fetch)
  const store = useProcessesStore()
  const first = store.refresh(); const second = store.refresh()
  store.applyEvent({ ...job, state: 'running', stage: 'processing', progress_percent: 42 })
  respond(new Response(JSON.stringify([job])))
  await Promise.all([first, second])
  expect(fetch).toHaveBeenCalledTimes(1)
  expect(store.jobs[0].state).toBe('running')
  store.applyEvent({ ...job, state: 'completed', progress_percent: 100 })
  store.applyEvent({ ...job, state: 'running' })
  expect(store.jobs[0].state).toBe('completed')
})
it('retries as a separate process and retains failed history', async () => {
  const store = useProcessesStore()
  store.applyEvent({ ...job, state: 'failed' })
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({ ...job, id: 'retry' }))))
  await store.act(store.jobs[0], 'retry')
  expect(store.jobs).toHaveLength(2)
  expect(store.jobs.find(item => item.id === 'process')?.state).toBe('failed')
})
