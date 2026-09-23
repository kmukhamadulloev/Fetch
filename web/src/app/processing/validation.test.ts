import { describe, expect, it } from 'vitest'
import { safeOutputName } from './validation'
describe('export filenames', () => {
  it('rejects traversal, reserved names, control characters and overlong UTF-8 names', () => {
    for (const name of ['../file', 'a/b', 'a\\b', 'CON', 'com1.txt', '.hidden', 'end.', 'a\u0000b', 'я'.repeat(91)]) expect(safeOutputName(name)).toBe(false)
    for (const name of ['My edit', 'Видео 2', 'CONcert', 'я'.repeat(90)]) expect(safeOutputName(name)).toBe(true)
  })
})
