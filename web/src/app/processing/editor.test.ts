import { expect, it } from 'vitest'
import { centeredCrop, moveCrop, resizeCrop } from './editor'
it('keeps dragged crop bounds even and inside the source', () => {
  const crop = { x: 20, y: 10, width: 80, height: 60 }
  expect(moveCrop(crop, 999, -999, 128, 96)).toEqual({ ...crop, x: 48, y: 0 })
  expect(moveCrop(crop, 3, 5, 128, 96)).toEqual({ ...crop, x: 24, y: 16 })
  expect(resizeCrop(crop, 999, 999, 128, 96)).toEqual({ ...crop, width: 108, height: 86 })
  expect(resizeCrop(crop, -999, -999, 128, 96)).toEqual({ ...crop, width: 2, height: 2 })
})
it('centers landscape and portrait presets without stretching or going outside the source', () => {
  expect(centeredCrop(1280, 720, 1)).toEqual({ x: 280, y: 0, width: 720, height: 720 })
  const portrait = centeredCrop(1280, 720, 9 / 16)
  expect(portrait.width % 2).toBe(0)
  expect(portrait.height % 2).toBe(0)
  expect(portrait.x + portrait.width).toBeLessThanOrEqual(1280)
  expect(portrait.y + portrait.height).toBeLessThanOrEqual(720)
})
