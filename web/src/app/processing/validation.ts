export function safeOutputName(value: string): boolean {
  const name = value.trim()
  return !!name && new TextEncoder().encode(name).length <= 180
    && !name.startsWith('.') && !name.endsWith('.')
    && ![...name].some(character => character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127 || '/\\:*?"<>|'.includes(character))
    && !/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)/i.test(name)
}
