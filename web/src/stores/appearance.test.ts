import { beforeEach, describe, expect, it, vi } from 'vitest'

describe('appearance preference', () => {
  beforeEach(() => {
    vi.resetModules()
    const values = new Map<string, string>()
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
        clear: () => values.clear(),
      },
    })
    document.documentElement.removeAttribute('data-theme')
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn() }),
    })
  })

  it('follows the system and persists an explicit preference', async () => {
    const { initializeAppearance, useAppearance } = await import('./appearance')
    initializeAppearance()
    const appearance = useAppearance()
    expect(document.documentElement.dataset.theme).toBe('light')
    appearance.setTheme('dark')
    expect(document.documentElement.dataset.theme).toBe('dark')
    expect(localStorage.getItem('fetch.theme')).toBe('dark')
  })
})
