export function centerScrollRailItem(
  rail: HTMLElement | null,
  item: HTMLElement | null,
  behavior: ScrollBehavior,
): void {
  if (!rail || !item) return;
  const railRect = rail.getBoundingClientRect();
  const itemRect = item.getBoundingClientRect();
  rail.scrollTo({
    left: Math.max(
      0,
      rail.scrollLeft +
        itemRect.left -
        railRect.left -
        (rail.clientWidth - itemRect.width) / 2,
    ),
    behavior,
  });
}
