import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type Ref,
  type RefCallback,
} from "react";

// Match the browser's rounded scroll metrics. A few pixels of residual distance
// should still count as the bottom, while a real reader scroll must pause follow.
const DEFAULT_BOTTOM_THRESHOLD = 24;
const SCROLL_KEYS = new Set(["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "]);

type ScrollViewport = Pick<HTMLElement, "scrollHeight" | "scrollTop" | "clientHeight">;

export function isNearBottom(
  viewport: ScrollViewport,
  threshold = DEFAULT_BOTTOM_THRESHOLD,
): boolean {
  return viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= threshold;
}

export function shouldFollowAfterScroll(
  viewport: ScrollViewport,
  previousScrollTop: number,
  wasFollowing: boolean,
  threshold = DEFAULT_BOTTOM_THRESHOLD,
  userInitiated = false,
): boolean {
  const moved = viewport.scrollTop !== previousScrollTop;
  if (!moved) return wasFollowing;
  if (!wasFollowing) return userInitiated && isNearBottom(viewport, threshold);
  if (viewport.scrollTop < previousScrollTop) return false;
  return isNearBottom(viewport, threshold);
}

function isEditableEventTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || target.tagName === "INPUT" || target.tagName === "TEXTAREA";
}

interface UseFollowLatestOptions {
  enabled?: boolean;
  resetKey?: string | number | null;
  bottomThreshold?: number;
}

interface FollowLatestControls {
  viewportRef: Ref<HTMLDivElement>;
  isFollowingLatest: boolean;
  hasNewContent: boolean;
  scrollToLatest: () => void;
}

export function useFollowLatest<T>(
  value: T,
  {
    enabled = true,
    resetKey,
    bottomThreshold = DEFAULT_BOTTOM_THRESHOLD,
  }: UseFollowLatestOptions = {},
): FollowLatestControls {
  const viewportElementRef = useRef<HTMLDivElement | null>(null);
  const [viewportElement, setViewportElement] = useState<HTMLDivElement | null>(null);
  const viewportRef = useCallback<RefCallback<HTMLDivElement>>((element) => {
    viewportElementRef.current = element;
    setViewportElement(element);
  }, []);
  const shouldFollowRef = useRef(true);
  const enabledRef = useRef(enabled);
  const bottomThresholdRef = useRef(bottomThreshold);
  const previousResetKeyRef = useRef(resetKey);
  const previousViewportElementRef = useRef<HTMLDivElement | null>(null);
  const previousEnabledRef = useRef(enabled);
  const previousScrollTopRef = useRef(0);
  const previousScrollHeightRef = useRef(0);
  const userScrollIntentRef = useRef(false);
  const userPointerActiveRef = useRef(false);
  const [isFollowingLatest, setIsFollowingLatest] = useState(true);
  const [hasNewContent, setHasNewContent] = useState(false);
  enabledRef.current = enabled;
  bottomThresholdRef.current = bottomThreshold;

  const scrollToLatest = useCallback(() => {
    shouldFollowRef.current = true;
    setIsFollowingLatest(true);
    setHasNewContent(false);
    const viewport = viewportElementRef.current;
    if (!viewport) return;
    viewport.scrollTop = Math.max(0, viewport.scrollHeight - viewport.clientHeight);
    previousScrollTopRef.current = viewport.scrollTop;
    previousScrollHeightRef.current = viewport.scrollHeight;
  }, []);

  const updateFollowing = useCallback((next: boolean) => {
    shouldFollowRef.current = next;
    setIsFollowingLatest((current) => current === next ? current : next);
    if (next) setHasNewContent(false);
  }, []);

  useLayoutEffect(() => {
    const reset = !Object.is(previousResetKeyRef.current, resetKey);
    const becameEnabled = enabled && !previousEnabledRef.current;
    const viewportChanged = previousViewportElementRef.current !== viewportElement;
    previousEnabledRef.current = enabled;
    previousViewportElementRef.current = viewportElement;

    if (reset || becameEnabled) {
      previousResetKeyRef.current = resetKey;
      shouldFollowRef.current = enabled;
      setIsFollowingLatest(enabled);
      setHasNewContent(false);
    }
    if (!enabled || !viewportElement) return;
    if (reset || becameEnabled || viewportChanged) {
      const userInteractionActive = userScrollIntentRef.current || userPointerActiveRef.current;
      if (shouldFollowRef.current && !userInteractionActive) {
        scrollToLatest();
      }
    }
  }, [enabled, resetKey, scrollToLatest, viewportElement]);

  useLayoutEffect(() => {
    if (!enabled || !viewportElement) return;
    if (shouldFollowRef.current) {
      scrollToLatest();
    } else if (isNearBottom(viewportElement, bottomThresholdRef.current)) {
      updateFollowing(true);
    }
  }, [enabled, scrollToLatest, updateFollowing, value, viewportElement]);

  useEffect(() => {
    const viewport = viewportElement;
    if (!viewport) return;
    previousScrollTopRef.current = viewport.scrollTop;
    previousScrollHeightRef.current = viewport.scrollHeight;
    userScrollIntentRef.current = false;
    userPointerActiveRef.current = false;
    let followFrame: number | null = null;
    let userIntentResetTimer: number | null = null;

    const markUserScrollIntent = () => {
      userScrollIntentRef.current = true;
      if (userIntentResetTimer != null) {
        window.clearTimeout(userIntentResetTimer);
      }
      // Keep the source marker until the browser emits the matching scroll
      // event. The timeout covers wheel/key presses that do not move at an
      // edge, without treating later programmatic scrolling as user input.
      userIntentResetTimer = window.setTimeout(() => {
        userScrollIntentRef.current = false;
        userIntentResetTimer = null;
      }, 500);
    };
    const clearUserScrollIntent = () => {
      userScrollIntentRef.current = false;
      if (userIntentResetTimer != null) {
        window.clearTimeout(userIntentResetTimer);
        userIntentResetTimer = null;
      }
    };
    const handleScroll = () => {
      const userInitiated = userScrollIntentRef.current || userPointerActiveRef.current;
      const next = userInitiated
        ? shouldFollowAfterScroll(
            viewport,
            previousScrollTopRef.current,
            shouldFollowRef.current,
            bottomThresholdRef.current,
            true,
          )
        : shouldFollowRef.current || isNearBottom(viewport, bottomThresholdRef.current);
      previousScrollTopRef.current = viewport.scrollTop;
      previousScrollHeightRef.current = viewport.scrollHeight;
      clearUserScrollIntent();
      if (!enabledRef.current) return;
      updateFollowing(next);
    };
    const scheduleFollow = () => {
      if (
        !enabledRef.current
        || !shouldFollowRef.current
        || userScrollIntentRef.current
        || userPointerActiveRef.current
        || followFrame != null
      ) return;
      followFrame = window.requestAnimationFrame(() => {
        followFrame = null;
        if (viewport.scrollTop !== previousScrollTopRef.current) {
          handleScroll();
        }
        if (
          enabledRef.current
          && shouldFollowRef.current
          && !userScrollIntentRef.current
          && !userPointerActiveRef.current
          && viewportElementRef.current === viewport
        ) {
          viewport.scrollTop = Math.max(0, viewport.scrollHeight - viewport.clientHeight);
          previousScrollTopRef.current = viewport.scrollTop;
          previousScrollHeightRef.current = viewport.scrollHeight;
        }
      });
    };
    const handleWheel = (event: WheelEvent) => {
      if (!enabledRef.current || event.deltaY === 0) return;
      markUserScrollIntent();
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!enabledRef.current || isEditableEventTarget(event.target)) return;
      if (!SCROLL_KEYS.has(event.key)) return;
      markUserScrollIntent();
    };
    const handlePointerDown = (event: PointerEvent) => {
      if (!enabledRef.current) return;
      userPointerActiveRef.current = false;
      const rect = viewport.getBoundingClientRect();
      if (
        event.clientX - rect.left >= viewport.clientWidth
        || event.clientY - rect.top >= viewport.clientHeight
      ) {
        // Scrollbar drags do not reliably expose a separate DOM target.
        userPointerActiveRef.current = true;
        markUserScrollIntent();
      }
    };
    const handlePointerEnd = () => {
      userPointerActiveRef.current = false;
      clearUserScrollIntent();
    };
    const handleTouchMove = () => {
      if (!enabledRef.current) return;
      markUserScrollIntent();
    };
    viewport.addEventListener("scroll", handleScroll, { passive: true });
    viewport.addEventListener("wheel", handleWheel, { passive: true, capture: true });
    window.addEventListener("keydown", handleKeyDown, true);
    viewport.addEventListener("pointerdown", handlePointerDown, true);
    viewport.addEventListener("pointerup", handlePointerEnd, true);
    viewport.addEventListener("pointercancel", handlePointerEnd, true);
    viewport.addEventListener("touchmove", handleTouchMove, { passive: true, capture: true });
    viewport.addEventListener("touchend", handlePointerEnd, true);
    viewport.addEventListener("touchcancel", handlePointerEnd, true);
    window.addEventListener("pointerup", handlePointerEnd, true);
    window.addEventListener("pointercancel", handlePointerEnd, true);

    // The paced renderer grows text without changing the messages array.
    const observer = typeof MutationObserver === "undefined"
      ? null
      : new MutationObserver(() => {
        if (!enabledRef.current) return;
        const previousScrollHeight = previousScrollHeightRef.current;
        const userInitiated = userScrollIntentRef.current || userPointerActiveRef.current;
        // Content/layout changes can emit a scroll event without user input.
        // Only reconcile that movement as a pause when the user supplied the
        // scroll source; otherwise keep the existing follow decision.
        if (userInitiated && viewport.scrollTop !== previousScrollTopRef.current) {
          handleScroll();
        } else {
          previousScrollTopRef.current = viewport.scrollTop;
        }
        const grew = viewport.scrollHeight > previousScrollHeight;
        previousScrollHeightRef.current = viewport.scrollHeight;
        if (!shouldFollowRef.current || userScrollIntentRef.current || userPointerActiveRef.current) {
          if (!grew) {
            if (isNearBottom(viewport, bottomThresholdRef.current)) {
              updateFollowing(true);
            }
            return;
          }
          if (isNearBottom(viewport, bottomThresholdRef.current)) {
            updateFollowing(true);
          }
          if (!shouldFollowRef.current) setHasNewContent(true);
          return;
        }
        scheduleFollow();
      });
    observer?.observe(viewport, {
      characterData: true,
      childList: true,
      subtree: true,
    });

    return () => {
      viewport.removeEventListener("scroll", handleScroll);
      viewport.removeEventListener("wheel", handleWheel, true);
      window.removeEventListener("keydown", handleKeyDown, true);
      viewport.removeEventListener("pointerdown", handlePointerDown, true);
      viewport.removeEventListener("pointerup", handlePointerEnd, true);
      viewport.removeEventListener("pointercancel", handlePointerEnd, true);
      viewport.removeEventListener("touchmove", handleTouchMove, true);
      viewport.removeEventListener("touchend", handlePointerEnd, true);
      viewport.removeEventListener("touchcancel", handlePointerEnd, true);
      window.removeEventListener("pointerup", handlePointerEnd, true);
      window.removeEventListener("pointercancel", handlePointerEnd, true);
      if (followFrame != null) window.cancelAnimationFrame(followFrame);
      clearUserScrollIntent();
      observer?.disconnect();
    };
  }, [scrollToLatest, updateFollowing, viewportElement]);

  return {
    viewportRef,
    isFollowingLatest,
    hasNewContent,
    scrollToLatest,
  };
}
