import { computed, ref } from 'vue'

export type ThemePreference = 'system' | 'light' | 'dark'
type ResolvedTheme = Exclude<ThemePreference, 'system'>

const storageKey = 'fetch.theme'
const preference = ref<ThemePreference>('system')
const systemTheme = ref<ResolvedTheme>('dark')
let initialized = false
let mediaQuery: MediaQueryList | null = null

function isTheme(value: string | null): value is ThemePreference {
  return value === 'system' || value === 'light' || value === 'dark'
}

function applyTheme() {
  const resolved = preference.value === 'system' ? systemTheme.value : preference.value
  document.documentElement.dataset.theme = resolved
  document.documentElement.style.colorScheme = resolved
  document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')?.setAttribute('content', resolved === 'dark' ? '#0b0d10' : '#f4f6f8')
}

export function initializeAppearance() {
  if (initialized) return
  initialized = true
  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  systemTheme.value = mediaQuery.matches ? 'dark' : 'light'
  let stored: string | null = null
  try { stored = window.localStorage.getItem(storageKey) } catch { /* Continue with the system preference. */ }
  if (isTheme(stored)) preference.value = stored
  mediaQuery.addEventListener('change', (event) => {
    systemTheme.value = event.matches ? 'dark' : 'light'
    if (preference.value === 'system') applyTheme()
  })
  applyTheme()
}

export function useAppearance() {
  const resolvedTheme = computed<ResolvedTheme>(() => preference.value === 'system' ? systemTheme.value : preference.value)

  function setTheme(theme: ThemePreference) {
    preference.value = theme
    try { window.localStorage.setItem(storageKey, theme) } catch { /* The live preference still applies. */ }
    applyTheme()
  }

  return { preference, resolvedTheme, setTheme }
}
