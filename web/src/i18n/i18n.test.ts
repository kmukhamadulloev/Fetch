import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createI18n } from 'vue-i18n'
import { messages } from './messages'

function leafKeys(value: unknown, prefix = ''): string[] {
  if (!value || typeof value !== 'object') return [prefix]
  return Object.entries(value).flatMap(([key, child]) => leafKeys(child, prefix ? `${prefix}.${key}` : key)).sort()
}

describe('localization', () => {
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
    document.documentElement.lang = ''
  })

  it('keeps every locale catalog structurally complete', () => {
    const englishKeys = leafKeys(messages.en)
    expect(leafKeys(messages.ru)).toEqual(englishKeys)
    expect(leafKeys(messages.tg)).toEqual(englishKeys)
  })

  it('compiles every translated message used by the production runtime', () => {
    const parameters = {
      botFather: '@BotFather', count: 2, date: 'date', duration: 'duration', level: 'level', name: 'name',
      ready: 3, source: 'source', status: 200, time: 'time', total: 4,
      url: 'url', value: 'value', visible: 2, watched: 1,
    }
    const compilerErrors = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    try {
      for (const locale of ['en', 'ru', 'tg'] as const) {
        const runtime = createI18n({ legacy: false, locale, messages })
        for (const key of leafKeys(messages.en)) runtime.global.t(key, parameters)
      }
      expect(compilerErrors).not.toHaveBeenCalled()
    } finally {
      compilerErrors.mockRestore()
    }
  })

  it('defaults invalid and missing preferences to English', async () => {
    localStorage.setItem('fetch.locale', 'de')
    const { i18n, initializeLocalization } = await import('./index')
    initializeLocalization()
    expect(i18n.global.locale.value).toBe('en')
    expect(document.documentElement.lang).toBe('en')
  })

  it('applies and persists an explicit locale immediately', async () => {
    const { i18n, initializeLocalization, setLocale } = await import('./index')
    initializeLocalization()
    setLocale('tg')
    expect(i18n.global.locale.value).toBe('tg')
    expect(document.documentElement.lang).toBe('tg')
    expect(localStorage.getItem('fetch.locale')).toBe('tg')
  })
})
