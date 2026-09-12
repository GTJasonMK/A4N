import type { ThemeStreamDisplayMode, ThemeStreamDisplaySettings } from "../api/theme";

export const DEFAULT_STREAM_DISPLAY_ENABLED = false;
export const DEFAULT_STREAM_DISPLAY_RATE = 30;
export const DEFAULT_STREAM_DISPLAY_MODE = "fixed" as const;
export const MIN_STREAM_DISPLAY_RATE = 1;
export const MAX_STREAM_DISPLAY_RATE = 120;

export type StreamDisplayMode = ThemeStreamDisplayMode;

export const DEFAULT_STREAM_DISPLAY_SETTINGS: ThemeStreamDisplaySettings = {
  enabled: DEFAULT_STREAM_DISPLAY_ENABLED,
  rate: DEFAULT_STREAM_DISPLAY_RATE,
  mode: DEFAULT_STREAM_DISPLAY_MODE,
};

export const normalizeStreamDisplayEnabled = (value: unknown): boolean =>
  typeof value === "boolean" ? value : DEFAULT_STREAM_DISPLAY_ENABLED;

export const normalizeStreamDisplayRate = (value: unknown): number => {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) return DEFAULT_STREAM_DISPLAY_RATE;
  return Math.min(MAX_STREAM_DISPLAY_RATE, Math.max(MIN_STREAM_DISPLAY_RATE, Math.round(numeric)));
};

export const normalizeStreamDisplayMode = (value: unknown): StreamDisplayMode =>
  value === "adaptive" ? "adaptive" : DEFAULT_STREAM_DISPLAY_MODE;

export const normalizeStreamDisplaySettings = (value: unknown): ThemeStreamDisplaySettings => {
  const source = value && typeof value === "object"
    ? value as Record<string, unknown>
    : {};
  return {
    enabled: normalizeStreamDisplayEnabled(source.enabled),
    rate: normalizeStreamDisplayRate(source.rate),
    mode: normalizeStreamDisplayMode(source.mode),
  };
};

export const estimateAdaptiveStreamRate = (
  previousRate: number | null,
  deltaCharacters: number,
  elapsedMs: number,
  fallbackRate = DEFAULT_STREAM_DISPLAY_RATE,
): number => {
  const fallback = normalizeStreamDisplayRate(fallbackRate);
  if (deltaCharacters <= 0 || elapsedMs <= 0) return previousRate ?? fallback;
  const instantRate = (deltaCharacters * 1000) / elapsedMs;
  const smoothedRate = previousRate == null
    ? instantRate
    : previousRate + (instantRate - previousRate) * 0.25;
  return Math.min(
    MAX_STREAM_DISPLAY_RATE,
    Math.max(MIN_STREAM_DISPLAY_RATE, smoothedRate),
  );
};
