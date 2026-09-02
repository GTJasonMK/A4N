const DEFAULT_SCROLL_PADDING = 12;

function scrollPadding(rail: HTMLElement, side: "left" | "right"): number {
  if (typeof window === "undefined") return DEFAULT_SCROLL_PADDING;
  const property = side === "left" ? "scrollPaddingLeft" : "scrollPaddingRight";
  const parsed = Number.parseFloat(window.getComputedStyle(rail)[property]);
  return Number.isFinite(parsed) ? parsed : DEFAULT_SCROLL_PADDING;
}

/** Scroll only enough to reveal an item; visible tabs stay in place. */
export function scrollRailItemIntoView(
  rail: HTMLElement | null,
  item: HTMLElement | null,
  behavior: ScrollBehavior,
): void {
  if (!rail || !item) return;
  const railRect = rail.getBoundingClientRect();
  const itemRect = item.getBoundingClientRect();
  const maxPadding = Math.max(0, rail.clientWidth / 2);
  const leftEdge = railRect.left + Math.min(scrollPadding(rail, "left"), maxPadding);
  const rightEdge = railRect.right - Math.min(scrollPadding(rail, "right"), maxPadding);
  let delta = 0;

  if (itemRect.left < leftEdge) {
    delta = itemRect.left - leftEdge;
  } else if (itemRect.right > rightEdge) {
    delta = itemRect.right - rightEdge;
  }

  if (delta === 0) return;

  const maxScrollLeft = Math.max(0, rail.scrollWidth - rail.clientWidth);
  const left = Math.min(
    maxScrollLeft,
    Math.max(0, rail.scrollLeft + delta),
  );
  if (Math.abs(left - rail.scrollLeft) < 1) return;
  rail.scrollTo({ left, behavior });
}
