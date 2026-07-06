const IME_COMPOSITION_SETTLE_MS = 80;

export function isImeCompositionActive(
  event: KeyboardEvent,
  composing: boolean,
  lastCompositionEnd: number,
): boolean {
  return (
    event.isComposing ||
    composing ||
    Date.now() - lastCompositionEnd < IME_COMPOSITION_SETTLE_MS
  );
}
