import { useCallback, useLayoutEffect, useState, type RefObject } from "react";

export function firstEnabledIndex(
  options: readonly { disabled?: boolean }[],
  start = 0,
  step = 1,
): number {
  for (let offset = 0; offset < options.length; offset += 1) {
    const index = (start + offset * step + options.length * 2) % options.length;
    if (!options[index].disabled) return index;
  }
  return -1;
}

export function useListboxPosition(
  anchorRef: RefObject<HTMLElement>,
  listboxRef: RefObject<HTMLDivElement>,
  isOpen: boolean,
  optionCount: number,
) {
  const [position, setPosition] = useState({
    top: 0, left: 0, width: 0, maxHeight: 0, ready: false,
  });
  const updatePosition = useCallback(() => {
    const anchor = anchorRef.current;
    const listbox = listboxRef.current;
    if (!anchor || !listbox) return;

    const rect = anchor.getBoundingClientRect();
    const viewport = window.visualViewport;
    const viewportLeft = viewport?.offsetLeft ?? 0;
    const viewportTop = viewport?.offsetTop ?? 0;
    const viewportWidth = viewport?.width ?? window.innerWidth;
    const viewportHeight = viewport?.height ?? window.innerHeight;
    const padding = 8;
    const gap = 4;
    const width = Math.min(Math.max(rect.width, 180), Math.max(1, viewportWidth - padding * 2));
    const spaceBelow = viewportTop + viewportHeight - rect.bottom - gap - padding;
    const spaceAbove = rect.top - viewportTop - gap - padding;
    const opensAbove = spaceAbove > spaceBelow;
    const maxHeight = Math.max(1, Math.min(320, opensAbove ? spaceAbove : spaceBelow));
    // Wrapped labels need their final width before measuring the popup height.
    listbox.style.width = `${width}px`;
    const height = Math.min(listbox.scrollHeight, maxHeight);
    const preferredTop = opensAbove ? rect.top - height - gap : rect.bottom + gap;
    const top = Math.max(viewportTop + padding, Math.min(preferredTop, viewportTop + viewportHeight - height - padding));
    const left = Math.max(viewportLeft + padding, Math.min(rect.left, viewportLeft + viewportWidth - width - padding));

    setPosition((current) => (
      current.top === top && current.left === left && current.width === width &&
      current.maxHeight === maxHeight && current.ready
        ? current
        : { top, left, width, maxHeight, ready: true }
    ));
  }, [anchorRef, listboxRef]);

  useLayoutEffect(() => {
    if (!isOpen) {
      setPosition((current) => current.ready ? { ...current, ready: false } : current);
      return;
    }
    updatePosition();
    const viewport = window.visualViewport;
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    viewport?.addEventListener("resize", updatePosition);
    viewport?.addEventListener("scroll", updatePosition);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
      viewport?.removeEventListener("resize", updatePosition);
      viewport?.removeEventListener("scroll", updatePosition);
    };
  }, [isOpen, optionCount, updatePosition]);

  return position;
}
