export const STREAM_DISPLAY_RATE_KEY = "a4n:stream-display-rate:v1";
export const STREAM_DISPLAY_MODE_KEY = "a4n:stream-display-mode:v1";
export const DEFAULT_STREAM_DISPLAY_RATE = 30;
export const DEFAULT_STREAM_DISPLAY_MODE = "fixed" as const;
export const MIN_STREAM_DISPLAY_RATE = 1;
export const MAX_STREAM_DISPLAY_RATE = 120;

export type StreamDisplayMode = "fixed" | "adaptive";

export const normalizeStreamDisplayRate = (value: unknown): number => {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) return DEFAULT_STREAM_DISPLAY_RATE;
  return Math.min(MAX_STREAM_DISPLAY_RATE, Math.max(MIN_STREAM_DISPLAY_RATE, Math.round(numeric)));
};

export const normalizeStreamDisplayMode = (value: unknown): StreamDisplayMode =>
  value === "adaptive" ? "adaptive" : DEFAULT_STREAM_DISPLAY_MODE;

export const parseStreamDisplayRate = (raw: string): number => {
  try {
    return normalizeStreamDisplayRate(JSON.parse(raw));
  } catch {
    return DEFAULT_STREAM_DISPLAY_RATE;
  }
};

export const serializeStreamDisplayRate = (value: number): string =>
  JSON.stringify(normalizeStreamDisplayRate(value));

export const parseStreamDisplayMode = (raw: string): StreamDisplayMode => {
  try {
    return normalizeStreamDisplayMode(JSON.parse(raw));
  } catch {
    return DEFAULT_STREAM_DISPLAY_MODE;
  }
};

export const serializeStreamDisplayMode = (value: StreamDisplayMode): string =>
  JSON.stringify(normalizeStreamDisplayMode(value));

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
