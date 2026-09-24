import { ApiError } from '@/app/api/client'
export function processingErrorKey(cause: unknown): string {
  const code = typeof cause === 'string' ? cause : cause instanceof ApiError ? cause.code : undefined
  if (code === 'CONFLICT') return 'exports.conflict'
  if (code === 'RUNTIME_MISSING' || code === 'RUNTIME_CORRUPT') return 'exports.runtimeError'
  if (code === 'FILE_NOT_FOUND' || code === 'NOT_FOUND') return 'exports.missing'
  if (code === 'INVALID_REQUEST') return 'exports.invalid'
  if (code === 'OUTPUT_DIRECTORY_UNAVAILABLE') return 'exports.diskError'
  if (cause instanceof ApiError && cause.status === 0) return 'exports.connectionError'
  return 'exports.failed'
}
