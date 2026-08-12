import type { DiagnosticLogEntry } from '@/app/api/client'

export type LogLevelFilter = 'all' | 'info' | 'warn' | 'error'

export const logLevelFilters: { value: LogLevelFilter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'info', label: 'Info' },
  { value: 'warn', label: 'Warnings' },
  { value: 'error', label: 'Errors' },
]

export function normalizedLogLevel(level: string): Exclude<LogLevelFilter, 'all'> {
  const normalized = level.trim().toLowerCase()
  if (normalized === 'error') return 'error'
  if (normalized === 'warn' || normalized === 'warning') return 'warn'
  return 'info'
}

export function filterDiagnosticLogs(logs: DiagnosticLogEntry[], filter: LogLevelFilter): DiagnosticLogEntry[] {
  if (filter === 'all') return logs
  return logs.filter((entry) => normalizedLogLevel(entry.level) === filter)
}

export function countDiagnosticLogs(logs: DiagnosticLogEntry[], filter: LogLevelFilter): number {
  return filterDiagnosticLogs(logs, filter).length
}
