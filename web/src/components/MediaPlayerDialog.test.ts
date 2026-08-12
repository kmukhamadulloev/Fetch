import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { afterEach, describe, expect, it, vi } from 'vitest'
import MediaPlayerDialog from './MediaPlayerDialog.vue'
import type { CompletedFile } from '@/app/api/client'
import { useLibraryStore } from '@/stores/library'

const file: CompletedFile = {
  id: 'af2bf705-8425-4178-9c5c-805e62db4f64',
  job_id: 'b64c93b5-55cb-4c33-9e65-c03cd7f85310',
  playlist: null,
  filename: 'fixture.mp4',
  thumbnail_available: false,
  size_bytes: 100,
  mime_type: 'video/mp4',
  title: 'Fixture media',
  browser_playable: true,
  playback: { file_id: 'af2bf705-8425-4178-9c5c-805e62db4f64', position_seconds: 20, duration_seconds: 100, completed: false, updated_at: '2026-08-09T00:00:00Z' },
  created_at: '2026-08-09T00:00:00Z',
}

afterEach(() => {
  Reflect.deleteProperty(document, 'fullscreenElement')
  Reflect.deleteProperty(document, 'exitFullscreen')
  document.body.innerHTML = ''
})

describe('MediaPlayerDialog keyboard controls', () => {
  it('seeks, changes volume, and toggles fullscreen', async () => {
    const pinia = createPinia()
    const wrapper = mount(MediaPlayerDialog, {
      props: { file, localClient: false },
      global: { plugins: [pinia] },
    })
    await flushPromises()

    const media = document.querySelector<HTMLVideoElement>('.player-video')!
    const stage = document.querySelector<HTMLElement>('.player-stage')!
    Object.defineProperty(media, 'duration', { configurable: true, value: 100 })
    media.dispatchEvent(new Event('loadedmetadata'))
    expect(media.currentTime).toBe(20)
    const saveProgress = vi.spyOn(useLibraryStore(pinia), 'saveProgress').mockResolvedValue()
    media.currentTime = 30
    media.dispatchEvent(new Event('pause'))
    await flushPromises()
    expect(saveProgress).toHaveBeenCalledWith(file, 30, 100)
    media.currentTime = 20
    media.volume = .5

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }))
    expect(media.currentTime).toBe(25)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }))
    expect(media.currentTime).toBe(20)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp' }))
    expect(media.volume).toBeCloseTo(.6)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }))
    expect(media.volume).toBeCloseTo(.5)

    const requestFullscreen = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(stage, 'requestFullscreen', { configurable: true, value: requestFullscreen })
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'f' }))
    await flushPromises()
    expect(requestFullscreen).toHaveBeenCalledOnce()

    const exitFullscreen = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(document, 'fullscreenElement', { configurable: true, value: stage })
    Object.defineProperty(document, 'exitFullscreen', { configurable: true, value: exitFullscreen })
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'F' }))
    await flushPromises()
    expect(exitFullscreen).toHaveBeenCalledOnce()
    expect(document.body.textContent).toContain('Exited fullscreen')
    expect(document.body.textContent).toContain('seek 5s')

    wrapper.unmount()
  })
})
