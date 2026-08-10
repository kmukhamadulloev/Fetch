import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useLibraryStore } from './library'
import type { CompletedFile } from '@/app/api/client'

const file: CompletedFile = {
  id: 'af2bf705-8425-4178-9c5c-805e62db4f64',
  job_id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310',
  filename: 'media.mp4',
  thumbnail_available: true,
  size_bytes: 100,
  mime_type: 'video/mp4',
  title: 'Media',
  browser_playable: true,
  created_at: '2026-08-09T00:00:00Z',
}

beforeEach(() => setActivePinia(createPinia()))
afterEach(() => vi.unstubAllGlobals())

describe('library store', () => {
  it('reveals host files and removes successfully deleted records', async () => {
    const fetchMock = vi.fn((path: string, init?: RequestInit) => {
      if (path === '/api/completed') return Promise.resolve(new Response(JSON.stringify([file])))
      if (path === '/api/history') return Promise.resolve(new Response('[]'))
      if (path.endsWith('/reveal') && init?.method === 'POST') return Promise.resolve(new Response(null, { status: 204 }))
      if (path.endsWith(file.id) && init?.method === 'DELETE') return Promise.resolve(new Response(null, { status: 204 }))
      return Promise.resolve(new Response(null, { status: 404 }))
    })
    vi.stubGlobal('fetch', fetchMock)
    const library = useLibraryStore()
    await library.refresh()
    await library.reveal(file)
    expect(fetchMock).toHaveBeenCalledWith(`/api/files/${file.id}/reveal`, expect.objectContaining({ method: 'POST' }))
    expect(await library.remove(file)).toBe(true)
    expect(library.completed).toEqual([])
  })
})
