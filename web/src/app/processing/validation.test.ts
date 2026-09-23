import { describe, expect, it } from 'vitest'
import { safeOutputName } from './validation'
describe('export filenames', () => {
  it('rejects traversal, reserved names, control characters and overlong UTF-8 names', () => {
    for (const name of ['../file', 'a/b', 'a\\b', 'CON', 'com1.txt', '.hidden', 'end.', 'a\u0000b', 'я'.repeat(91)]) expect(safeOutputName(name)).toBe(false)
    for (const name of ['My edit', 'Видео 2', 'CONcert', 'я'.repeat(90)]) expect(safeOutputName(name)).toBe(true)
  })
})

import { evenDimension, validCrop, proportionalHeight } from './validation'
it('validates crop boundaries and encoder dimensions; preserves resize proportions', () => {
  expect(validCrop({ x: 2, y: 4, width: 94, height: 60 }, 96, 64)).toBe(true)
  for (const crop of [{ x: 3, y: 0, width: 80, height: 60 }, { x: 20, y: 0, width: 80, height: 60 }, { x: 0, y: 0, width: 0, height: 60 }]) expect(validCrop(crop, 96, 64)).toBe(false)
  for (const dimension of [NaN, Infinity, 0, -2, 3, 7682]) expect(evenDimension(dimension)).toBe(false)
  expect(proportionalHeight(640, 1280, 720)).toBe(360)
  expect(proportionalHeight(360, 720, 1280)).toBe(640)
})
