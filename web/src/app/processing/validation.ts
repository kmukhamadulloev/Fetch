export function safeOutputName(value: string): boolean {
  const name = value.trim()
  return !!name && new TextEncoder().encode(name).length <= 180
    && !name.startsWith('.') && !name.endsWith('.')
    && ![...name].some(character => character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127 || '/\\:*?"<>|'.includes(character))
    && !/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)/i.test(name)
}

export function evenDimension(value: number): boolean { return Number.isInteger(value) && value > 0 && value <= 7680 && value % 2 === 0 }
export function validCrop(crop: { x: number; y: number; width: number; height: number }, width: number, height: number): boolean {
  return [crop.x, crop.y].every(value => Number.isInteger(value) && value >= 0 && value % 2 === 0)
    && evenDimension(crop.width) && evenDimension(crop.height)
    && crop.x + crop.width <= width && crop.y + crop.height <= height
}
export function proportionalHeight(width: number, sourceWidth: number, sourceHeight: number): number {
  return sourceWidth > 0 ? Math.max(2, Math.round(width * sourceHeight / sourceWidth / 2) * 2) : 0
}
