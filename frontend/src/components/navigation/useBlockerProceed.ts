import { useCallback, useRef } from "react";
import type { Blocker } from "react-router-dom";

export function useBlockerProceed(blocker: Blocker): () => void {
  const current = useRef(blocker);
  const continued = useRef<(() => void) | undefined>(undefined);
  current.current = blocker;
  return useCallback(() => {
    const proceed = current.current.proceed;
    // 自动放行和异步放弃草稿可能同时完成；每次阻塞只能推进一次。
    if (!proceed || continued.current === proceed) return;
    continued.current = proceed;
    proceed();
  }, []);
}
