import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useLibraryStore } from './library'
import type { CompletedFile } from '@/app/api/client'

const file: CompletedFile = {
  id: 'af2bf705-8425-4178-9c5c-805e62db4f64',
  job_id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310',
  playlist: null,
  filename: 'media.mp4',
  thumbnail_available: true,
  size_bytes: 100,
  mime_type: 'video/mp4',
  title: 'Media',
  browser_playable: true,
  playback: null,
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
      if (path.endsWith('/progress') && init?.method === 'PUT') return Promise.resolve(new Response(JSON.stringify({ file_id: file.id, position_seconds: 25, duration_seconds: 100, completed: false, updated_at: '2026-08-09T01:00:00Z' })))
      if (path.endsWith('/progress') && init?.method === 'DELETE') return Promise.resolve(new Response(null, { status: 204 }))
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

  it('applies and clears persisted playback progress', async () => {
    const fetchMock = vi.fn((path: string, init?: RequestInit) => {
      if (path.endsWith('/progress') && init?.method === 'PUT') return Promise.resolve(new Response(JSON.stringify({ file_id: file.id, position_seconds: 25, duration_seconds: 100, completed: false, updated_at: '2026-08-09T01:00:00Z' })))
      if (path.endsWith('/progress') && init?.method === 'DELETE') return Promise.resolve(new Response(null, { status: 204 }))
      return Promise.resolve(new Response(null, { status: 404 }))
    })
    vi.stubGlobal('fetch', fetchMock)
    const library = useLibraryStore()
    library.applyCompleted({ ...file })
    await library.saveProgress(file, 25, 100)
    expect(library.completed[0].playback?.position_seconds).toBe(25)
    expect(await library.resetProgress(file)).toBe(true)
    expect(library.completed[0].playback).toBeNull()
  })

  it('groups playlist files by identity and preserves playlist order', () => {
    const library = useLibraryStore()
    library.applyCompleted({
      ...file,
      id: 'playlist-second',
      playlist: { id: 'fixture-list', title: 'Fixture playlist', index: 2 },
      title: 'Second item',
      size_bytes: 60,
    })
    library.applyCompleted({
      ...file,
      id: 'playlist-first',
      playlist: { id: 'fixture-list', title: 'Fixture playlist', index: 1 },
      title: 'First item',
      size_bytes: 40,
    })
    library.applyCompleted(file)

    expect(library.standalone).toEqual([file])
    expect(library.playlists).toHaveLength(1)
    expect(library.playlists[0].title).toBe('Fixture playlist')
    expect(library.playlists[0].size_bytes).toBe(100)
    expect(library.playlists[0].files.map((item) => item.title)).toEqual(['First item', 'Second item'])
  })
})
