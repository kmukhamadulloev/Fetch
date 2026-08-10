import { mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import { describe, expect, it } from 'vitest'
import AppNavigation from './AppNavigation.vue'

describe('AppNavigation', () => {
  it('provides desktop destinations without fake counters', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/', component: { template: '<div />' } },
        { path: '/downloads', component: { template: '<div />' } },
        { path: '/completed', component: { template: '<div />' } },
        { path: '/history', component: { template: '<div />' } },
        { path: '/settings', component: { template: '<div />' } },
        { path: '/logs', component: { template: '<div />' } },
      ],
    })
    await router.push('/')
    await router.isReady()
    const wrapper = mount(AppNavigation, { props: { variant: 'desktop' }, global: { plugins: [router] } })
    expect(wrapper.text()).toContain('New download')
    expect(wrapper.text()).toContain('Downloads')
    expect(wrapper.text()).not.toContain('2')
    expect(wrapper.find('.mobile-bar').exists()).toBe(false)
  })

  it('provides the four primary mobile destinations', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/', component: { template: '<div />' } },
        { path: '/downloads', component: { template: '<div />' } },
        { path: '/completed', component: { template: '<div />' } },
        { path: '/settings', component: { template: '<div />' } },
      ],
    })
    await router.push('/')
    await router.isReady()
    const wrapper = mount(AppNavigation, { props: { variant: 'mobile' }, global: { plugins: [router] } })
    expect(wrapper.findAll('.mobile-nav')).toHaveLength(4)
    expect(wrapper.find('.desktop-nav').exists()).toBe(false)
  })
})
