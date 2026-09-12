import { useEffect, useRef, useState } from "react";
import { usePersistedState } from "./usePersistedState";
import {
  DEFAULT_STREAM_DISPLAY_RATE,
  normalizeStreamDisplayRate,
  parseStreamDisplayRate,
  serializeStreamDisplayRate,
  STREAM_DISPLAY_RATE_KEY,
} from "../utils/streamDisplaySettings";

export interface PacedStreamText {
  displayedText: string;
  hasPendingText: boolean;
  isThinking: boolean;
}

export const useStreamDisplayRate = () => usePersistedState<number>(
  STREAM_DISPLAY_RATE_KEY,
  DEFAULT_STREAM_DISPLAY_RATE,
  {
    parse: parseStreamDisplayRate,
    serialize: serializeStreamDisplayRate,
  },
);

const commonPrefix = (left: string, right: string): string => {
  if (right.startsWith(left)) return left;
  let index = 0;
  while (index < left.length && index < right.length) {
    const point = left.codePointAt(index)!;
    if (point !== right.codePointAt(index)) break;
    index += point > 0xffff ? 2 : 1;
  }
  return right.slice(0, index);
};

export const advancePacedStreamText = (displayedText: string, targetText: string): string => {
  if (displayedText === targetText) return displayedText;
  const prefix = commonPrefix(displayedText, targetText);
  if (prefix !== displayedText) return prefix;
  const width = targetText.codePointAt(displayedText.length)! > 0xffff ? 2 : 1;
  return targetText.slice(0, displayedText.length + width);
};

export const usePacedStreamText = (
  sourceText: string,
  active: boolean,
  drainWhenInactive = false,
): PacedStreamText => {
  const [rate] = useStreamDisplayRate();
  const [displayedText, setDisplayedText] = useState(() => (active ? "" : sourceText));
  const [drainRequested, setDrainRequested] = useState(0);
  const displayedTextRef = useRef(active ? "" : sourceText);
  const targetTextRef = useRef(sourceText);

  useEffect(() => {
    targetTextRef.current = sourceText;
    const reconciled = commonPrefix(displayedTextRef.current, sourceText);
    if (reconciled !== displayedTextRef.current) {
      displayedTextRef.current = reconciled;
      setDisplayedText(reconciled);
    }
    if (!active) {
      if (!drainWhenInactive && displayedTextRef.current !== sourceText) {
        displayedTextRef.current = sourceText;
        setDisplayedText(sourceText);
      } else if (drainWhenInactive && displayedTextRef.current !== sourceText) {
        setDrainRequested((value) => value + 1);
      }
    }
  }, [active, drainWhenInactive, sourceText]);

  useEffect(() => {
    if (!active && !drainWhenInactive) return undefined;
    if (!active && displayedTextRef.current === targetTextRef.current) return undefined;
    const interval = globalThis.setInterval(() => {
      if (!active && drainWhenInactive && displayedTextRef.current === targetTextRef.current) {
        globalThis.clearInterval(interval);
        return;
      }
      const next = advancePacedStreamText(displayedTextRef.current, targetTextRef.current);
      if (next === displayedTextRef.current) return;
      displayedTextRef.current = next;
      setDisplayedText(next);
    }, 1000 / normalizeStreamDisplayRate(rate));
    return () => globalThis.clearInterval(interval);
  }, [active, drainRequested, drainWhenInactive, rate]);

  const hasPendingText = sourceText.length > displayedText.length
    && sourceText.startsWith(displayedText);

  return {
    displayedText,
    hasPendingText,
    isThinking: active && !hasPendingText,
  };
};
