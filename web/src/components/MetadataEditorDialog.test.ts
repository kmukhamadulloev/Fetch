import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter, matchedRouteKey } from 'vue-router'
import { shallowRef } from 'vue'
import MetadataEditorDialog from './MetadataEditorDialog.vue'
import { i18n, setLocale } from '@/i18n'
import * as api from '@/app/api/client'
import { useLibraryStore } from '@/stores/library'

const file: api.CompletedFile = { id: 'file-id', job_id: 'job-id', playlist: null, filename: 'audio.mp3', thumbnail_available: true, size_bytes: 100, mime_type: 'audio/mpeg', title: 'Title', browser_playable: true, playback: null, created_at: '2026-09-23T00:00:00Z' }
const snapshot: api.MediaMetadata = { revision: 'v1', editable: true, media_type: 'audio', container: 'mp3', fields: { title: 'Title', artist: '', album: '', album_artist: '', track: '1', genre: '' }, supported_fields: ['title', 'artist', 'album', 'album_artist', 'track', 'genre'], artwork_available: true, artwork_editable: true, information: { filename: file.filename } }

async function setup() {
  setLocale('en')
  vi.spyOn(api, 'getMetadata').mockResolvedValue(structuredClone(snapshot))
  vi.spyOn(api, 'getMetadataStatus').mockResolvedValue({ file_id: file.id, operation_id: null, state: 'idle', error: null })
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: { template: '<div />' } }] })
  await router.push('/'); await router.isReady()
  const pinia = createPinia()
  const wrapper = mount(MetadataEditorDialog, { props: { file }, global: { plugins: [pinia, i18n, router], provide: { [matchedRouteKey as symbol]: shallowRef(router.currentRoute.value.matched[0]) }, stubs: { Teleport: true } } })
  await flushPromises()
  return { wrapper, library: useLibraryStore(pinia) }
}
afterEach(() => { vi.restoreAllMocks(); document.body.innerHTML = '' })

describe('MetadataEditorDialog', () => {
  it('shows basic audio fields, expands advanced and guards dirty dismissal', async () => {
    const { wrapper } = await setup()
    expect(wrapper.text()).toContain('Album artist')
    expect(wrapper.text()).not.toContain('Genre')
    await wrapper.findAll('button').find((button) => button.text() === 'Advanced')!.trigger('click')
    expect(wrapper.text()).toContain('Genre')
    await wrapper.find('input[type="text"]').setValue('Changed')
    await wrapper.find('form').trigger('keydown', { key: 'Escape' })
    expect(wrapper.text()).toContain('Discard your unsaved changes?')
    expect(wrapper.emitted('close')).toBeUndefined()
    wrapper.unmount()
  })

  it('sends only changed fields and consumes SSE completion without waiting for polling', async () => {
    const { wrapper, library } = await setup()
    const save = vi.spyOn(api, 'saveMetadata').mockResolvedValue({ file_id: file.id, operation_id: 'op', state: 'saving', error: null })
    vi.spyOn(library, 'refresh').mockResolvedValue()
    await wrapper.find('input[type="text"]').setValue('Changed')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(save).toHaveBeenCalledWith(file.id, { revision: 'v1', fields: { title: 'Changed' }, artwork: { action: 'keep' } })
    library.applyMetadata({ file_id: file.id, operation_id: 'other-operation', state: 'completed', error: null })
    await flushPromises()
    expect(wrapper.emitted('close')).toBeUndefined()
    library.applyMetadata({ file_id: file.id, operation_id: 'op', state: 'completed', error: null })
    await flushPromises()
    expect(wrapper.emitted('close')).toHaveLength(1)
    expect(library.refresh).toHaveBeenCalledOnce()
    wrapper.unmount()
  })
})
