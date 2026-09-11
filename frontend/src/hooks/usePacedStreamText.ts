import { useEffect, useRef, useState } from "react";
import { usePersistedState } from "./usePersistedState";
import {
  DEFAULT_STREAM_DISPLAY_RATE,
  DEFAULT_STREAM_DISPLAY_MODE,
  estimateAdaptiveStreamRate,
  normalizeStreamDisplayRate,
  parseStreamDisplayMode,
  parseStreamDisplayRate,
  serializeStreamDisplayMode,
  serializeStreamDisplayRate,
  STREAM_DISPLAY_MODE_KEY,
  STREAM_DISPLAY_RATE_KEY,
  type StreamDisplayMode,
} from "../utils/streamDisplaySettings";

export interface PacedStreamText {
  displayedText: string;
  hasPendingText: boolean;
  isThinking: boolean;
  isDrained: boolean;
}

export const useStreamDisplayRate = () => usePersistedState<number>(
  STREAM_DISPLAY_RATE_KEY,
  DEFAULT_STREAM_DISPLAY_RATE,
  {
    parse: parseStreamDisplayRate,
    serialize: serializeStreamDisplayRate,
  },
);

export const useStreamDisplayMode = () => usePersistedState<StreamDisplayMode>(
  STREAM_DISPLAY_MODE_KEY,
  DEFAULT_STREAM_DISPLAY_MODE,
  {
    parse: parseStreamDisplayMode,
    serialize: serializeStreamDisplayMode,
  },
);

const commonPrefix = (left: string, right: string): string => {
  const leftChars = Array.from(left);
  const rightChars = Array.from(right);
  let index = 0;
  while (index < leftChars.length && index < rightChars.length && leftChars[index] === rightChars[index]) {
    index += 1;
  }
  return rightChars.slice(0, index).join("");
};

export const advancePacedStreamText = (displayedText: string, targetText: string): string => {
  const displayedChars = Array.from(displayedText);
  const targetChars = Array.from(targetText);
  const prefix = commonPrefix(displayedText, targetText);
  if (prefix !== displayedText) return prefix;
  return targetChars.slice(0, displayedChars.length + 1).join("");
};

export const usePacedStreamText = (
  sourceText: string,
  active: boolean,
  drainWhenInactive = false,
): PacedStreamText => {
  const [rate] = useStreamDisplayRate();
  const [mode] = useStreamDisplayMode();
  const [displayedText, setDisplayedText] = useState(() => (active ? "" : sourceText));
  const [drainRequested, setDrainRequested] = useState(0);
  const initialSourceChars = Array.from(sourceText);
  const displayedTextRef = useRef(active ? "" : sourceText);
  const displayedCharCountRef = useRef(active ? 0 : initialSourceChars.length);
  const targetTextRef = useRef(sourceText);
  const targetCharsRef = useRef(initialSourceChars);
  const sampleRef = useRef({
    sourceText,
    sourceLength: initialSourceChars.length,
    sampledAt: Date.now(),
    estimatedRate: null as number | null,
    active,
    mode,
    rate,
  });
  const adaptiveRateRef = useRef(rate);

  useEffect(() => {
    const now = Date.now();
    const targetChars = Array.from(sourceText);
    const previousSample = sampleRef.current;
    const settingsChanged = previousSample.mode !== mode || previousSample.rate !== rate;
    const taskStarted = active && !previousSample.active;
    const sourceReplaced = sourceText !== previousSample.sourceText
      && !sourceText.startsWith(previousSample.sourceText);

    targetCharsRef.current = targetChars;
    targetTextRef.current = sourceText;
    if (
      targetChars.length < displayedCharCountRef.current
      || !sourceText.startsWith(displayedTextRef.current)
    ) {
      const reconciled = commonPrefix(displayedTextRef.current, sourceText);
      if (reconciled !== displayedTextRef.current) {
        displayedTextRef.current = reconciled;
        displayedCharCountRef.current = Array.from(reconciled).length;
        setDisplayedText(reconciled);
      }
    }

    if (settingsChanged || taskStarted || sourceReplaced) {
      sampleRef.current = {
        sourceText,
        sourceLength: targetChars.length,
        sampledAt: now,
        estimatedRate: null,
        active,
        mode,
        rate,
      };
      adaptiveRateRef.current = rate;
    } else if (mode === "adaptive" && targetChars.length > previousSample.sourceLength) {
      const nextRate = estimateAdaptiveStreamRate(
        previousSample.estimatedRate,
        targetChars.length - previousSample.sourceLength,
        now - previousSample.sampledAt,
        rate,
      );
      sampleRef.current = {
        sourceText,
        sourceLength: targetChars.length,
        sampledAt: now,
        estimatedRate: nextRate,
        active,
        mode,
        rate,
      };
      adaptiveRateRef.current = nextRate;
    } else {
      sampleRef.current = { ...previousSample, sourceText, active, mode, rate };
    }

    if (!active) {
      if (!drainWhenInactive && displayedTextRef.current !== sourceText) {
        displayedTextRef.current = sourceText;
        displayedCharCountRef.current = targetChars.length;
        setDisplayedText(sourceText);
      } else if (drainWhenInactive && displayedTextRef.current !== sourceText) {
        setDrainRequested((value) => value + 1);
      }
    }
  }, [active, drainWhenInactive, mode, rate, sourceText]);

  useEffect(() => {
    if (!active && !drainWhenInactive) return undefined;
    if (!active && displayedTextRef.current === targetTextRef.current) return undefined;
    let timeout: ReturnType<typeof globalThis.setTimeout> | null = null;
    const nextDelay = () => 1000 / normalizeStreamDisplayRate(
      mode === "adaptive" ? adaptiveRateRef.current : rate,
    );
    const tick = () => {
      if (!active && drainWhenInactive && displayedTextRef.current === targetTextRef.current) {
        return;
      }
      const nextChar = targetCharsRef.current[displayedCharCountRef.current];
      if (nextChar != null) {
        const next = `${displayedTextRef.current}${nextChar}`;
        displayedTextRef.current = next;
        displayedCharCountRef.current += 1;
        setDisplayedText(next);
      }
      timeout = globalThis.setTimeout(tick, nextDelay());
    };
    timeout = globalThis.setTimeout(tick, nextDelay());
    return () => {
      if (timeout != null) globalThis.clearTimeout(timeout);
    };
  }, [active, drainRequested, drainWhenInactive, mode, rate]);

  const hasPendingText = displayedText !== sourceText
    && sourceText.startsWith(displayedText);
  const isDrained = displayedText === sourceText;

  return {
    displayedText,
    hasPendingText,
    isThinking: active && !hasPendingText,
    isDrained,
  };
};
