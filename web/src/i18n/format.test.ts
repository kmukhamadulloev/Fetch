import { describe, expect, it } from 'vitest'
import { formatBytes, formatDateTime, formatNumber } from './format'

describe('localized value formatting', () => {
  it('uses the active locale for numbers and byte sizes', () => {
    expect(formatNumber(12.5, 'en')).toBe('12.5')
    expect(formatNumber(12.5, 'ru')).toBe('12,5')
    expect(formatBytes(1536, 'ru')).toBe('1,5 KB')
  })

  it('formats timestamps through Intl for every supported locale', () => {
    const timestamp = '2026-08-20T12:00:00Z'
    for (const locale of ['en', 'ru', 'tg']) {
      expect(formatDateTime(timestamp, locale)).not.toBe(timestamp)
    }
  })
})
