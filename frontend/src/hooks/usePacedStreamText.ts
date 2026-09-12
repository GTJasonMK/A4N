import { useEffect, useRef, useState } from "react";
import {
  estimateAdaptiveStreamRate,
  normalizeStreamDisplayRate,
} from "../utils/streamDisplaySettings";
import {
  getActiveThemeStreamDisplaySettings,
  THEME_APPLIED_EVENT,
} from "../theme/activeTheme";

const useThemeStreamDisplaySettings = () => {
  const [settings, setSettings] = useState(getActiveThemeStreamDisplaySettings);

  useEffect(() => {
    if (typeof window === "undefined") return undefined;
    const syncSettings = () => setSettings(getActiveThemeStreamDisplaySettings());
    window.addEventListener(THEME_APPLIED_EVENT, syncSettings);
    syncSettings();
    return () => window.removeEventListener(THEME_APPLIED_EVENT, syncSettings);
  }, []);

  return settings;
};

export interface PacedStreamText {
  displayedText: string;
  hasPendingText: boolean;
  isThinking: boolean;
  isDrained: boolean;
}

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
  const { enabled, rate, mode } = useThemeStreamDisplaySettings();
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
    enabled,
    mode,
    rate,
  });
  const adaptiveRateRef = useRef(rate);

  useEffect(() => {
    const now = Date.now();
    const targetChars = Array.from(sourceText);
    const previousSample = sampleRef.current;
    const settingsChanged = previousSample.enabled !== enabled
      || previousSample.mode !== mode
      || previousSample.rate !== rate;
    const taskStarted = active && !previousSample.active;
    const sourceReplaced = sourceText !== previousSample.sourceText
      && !sourceText.startsWith(previousSample.sourceText);

    targetCharsRef.current = targetChars;
    targetTextRef.current = sourceText;

    if (!enabled) {
      if (displayedTextRef.current !== sourceText) {
        displayedTextRef.current = sourceText;
        displayedCharCountRef.current = targetChars.length;
        setDisplayedText(sourceText);
      }
      sampleRef.current = {
        sourceText,
        sourceLength: targetChars.length,
        sampledAt: now,
        estimatedRate: null,
        active,
        enabled,
        mode,
        rate,
      };
      return;
    }

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
        enabled,
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
        enabled,
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
  }, [active, drainWhenInactive, enabled, mode, rate, sourceText]);

  useEffect(() => {
    if (!enabled) return undefined;
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
  }, [active, drainRequested, drainWhenInactive, enabled, mode, rate]);

  const effectiveDisplayedText = enabled ? displayedText : sourceText;
  const hasPendingText = enabled
    && effectiveDisplayedText !== sourceText
    && sourceText.startsWith(effectiveDisplayedText);
  const isDrained = !enabled || effectiveDisplayedText === sourceText;

  return {
    displayedText: effectiveDisplayedText,
    hasPendingText,
    isThinking: active && (enabled ? !hasPendingText : sourceText.length === 0),
    isDrained,
  };
};
