export interface CropRect { x: number; y: number; width: number; height: number }
const even = (value: number) => Math.round(value / 2) * 2
const clampEven = (value: number, min: number, max: number) => Math.max(min, Math.min(Math.floor(max / 2) * 2, even(value)))
export function moveCrop(crop: CropRect, dx: number, dy: number, width: number, height: number): CropRect {
  return { ...crop, x: clampEven(crop.x + dx, 0, width - crop.width), y: clampEven(crop.y + dy, 0, height - crop.height) }
}
export function resizeCrop(crop: CropRect, dx: number, dy: number, width: number, height: number): CropRect {
  return { ...crop, width: clampEven(crop.width + dx, 2, Math.min(7680, width - crop.x)), height: clampEven(crop.height + dy, 2, Math.min(7680, height - crop.y)) }
}
export function centeredCrop(width: number, height: number, ratio: number): CropRect {
  const w = Math.floor(Math.min(width, height * ratio, 7680, 7680 * ratio) / 2) * 2
  const h = Math.floor(Math.min(height, w / ratio) / 2) * 2
  return { x: Math.floor((width - w) / 4) * 2, y: Math.floor((height - h) / 4) * 2, width: w, height: h }
}
