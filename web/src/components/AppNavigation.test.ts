import { mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import { describe, expect, it } from 'vitest'
import AppNavigation from './AppNavigation.vue'
import { i18n, setLocale } from '@/i18n'

describe('AppNavigation', () => {
  it('provides desktop destinations without fake counters', async () => {
    setLocale('en')
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/', component: { template: '<div />' } },
        { path: '/processes', component: { template: '<div />' } },
        { path: '/completed', component: { template: '<div />' } },
        { path: '/history', component: { template: '<div />' } },
        { path: '/settings', component: { template: '<div />' } },
        { path: '/logs', component: { template: '<div />' } },
      ],
    })
    await router.push('/')
    await router.isReady()
    const wrapper = mount(AppNavigation, { props: { variant: 'desktop' }, global: { plugins: [router, i18n] } })
    expect(wrapper.text()).toContain('New download')
    expect(wrapper.text()).toContain('Processes')
    expect(wrapper.text()).not.toContain('2')
    expect(wrapper.find('.mobile-bar').exists()).toBe(false)
  })

  it('provides the four primary mobile destinations', async () => {
    setLocale('en')
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/', component: { template: '<div />' } },
        { path: '/processes', component: { template: '<div />' } },
        { path: '/completed', component: { template: '<div />' } },
        { path: '/settings', component: { template: '<div />' } },
      ],
    })
    await router.push('/')
    await router.isReady()
    const wrapper = mount(AppNavigation, { props: { variant: 'mobile' }, global: { plugins: [router, i18n] } })
    expect(wrapper.findAll('.mobile-nav')).toHaveLength(4)
    expect(wrapper.find('.desktop-nav').exists()).toBe(false)
  })
})
