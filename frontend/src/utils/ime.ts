const IME_COMPOSITION_SETTLE_MS = 80;

export function isImeCompositionActive(
  event: KeyboardEvent,
  composing = false,
  lastCompositionEnd = 0,
): boolean {
  return (
    event.isComposing ||
    event.keyCode === 229 ||
    composing ||
    (lastCompositionEnd > 0 && Date.now() - lastCompositionEnd < IME_COMPOSITION_SETTLE_MS)
  );
}
