import { describe, expect, it } from 'vitest'
import type { DiagnosticLogEntry } from '@/app/api/client'
import { countDiagnosticLogs, filterDiagnosticLogs, normalizedLogLevel } from './logs'

const entry = (id: number, level: string): DiagnosticLogEntry => ({
  id,
  level,
  subsystem: 'runtime',
  message: `message ${id}`,
  details: null,
  created_at: '2026-08-12T19:18:08Z',
})

describe('diagnostic log filters', () => {
  const logs = [entry(1, 'info'), entry(2, 'warn'), entry(3, 'warning'), entry(4, 'error')]

  it('normalizes retained severity values', () => {
    expect(normalizedLogLevel('ERROR')).toBe('error')
    expect(normalizedLogLevel('warning')).toBe('warn')
    expect(normalizedLogLevel('debug')).toBe('info')
  })

  it('filters and counts each visible severity without changing order', () => {
    expect(filterDiagnosticLogs(logs, 'all').map(({ id }) => id)).toEqual([1, 2, 3, 4])
    expect(filterDiagnosticLogs(logs, 'warn').map(({ id }) => id)).toEqual([2, 3])
    expect(countDiagnosticLogs(logs, 'info')).toBe(1)
    expect(countDiagnosticLogs(logs, 'error')).toBe(1)
  })
})
