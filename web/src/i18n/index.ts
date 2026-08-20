import { createI18n } from 'vue-i18n'
import { messages } from './messages'

export const supportedLocales = ['en', 'ru', 'tg'] as const
export type AppLocale = typeof supportedLocales[number]

const storageKey = 'fetch.locale'

export function isAppLocale(value: unknown): value is AppLocale {
  return typeof value === 'string' && supportedLocales.includes(value as AppLocale)
}

export function resolveLocale(value: unknown): AppLocale {
  return isAppLocale(value) ? value : 'en'
}

function savedLocale(): AppLocale {
  if (typeof window === 'undefined') return 'en'
  try { return resolveLocale(window.localStorage.getItem(storageKey)) }
  catch { return 'en' }
}

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: savedLocale(),
  fallbackLocale: 'en',
  messages,
  missingWarn: false,
  fallbackWarn: false,
})

function applyDocumentLocale(locale: AppLocale) {
  if (typeof document !== 'undefined') document.documentElement.lang = locale
}

export function initializeLocalization() {
  const locale = savedLocale()
  i18n.global.locale.value = locale
  applyDocumentLocale(locale)
}

export function setLocale(locale: AppLocale) {
  i18n.global.locale.value = locale
  applyDocumentLocale(locale)
  try { window.localStorage.setItem(storageKey, locale) }
  catch { /* Keep the live language when browser storage is unavailable. */ }
}

export function useLocalization() {
  return { locale: i18n.global.locale, setLocale, supportedLocales }
}
