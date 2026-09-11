export const STREAM_DISPLAY_RATE_KEY = "a4n:stream-display-rate:v1";
export const DEFAULT_STREAM_DISPLAY_RATE = 30;
export const MIN_STREAM_DISPLAY_RATE = 1;
export const MAX_STREAM_DISPLAY_RATE = 120;

export const normalizeStreamDisplayRate = (value: unknown): number => {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) return DEFAULT_STREAM_DISPLAY_RATE;
  return Math.min(MAX_STREAM_DISPLAY_RATE, Math.max(MIN_STREAM_DISPLAY_RATE, Math.round(numeric)));
};

export const parseStreamDisplayRate = (raw: string): number => {
  try {
    return normalizeStreamDisplayRate(JSON.parse(raw));
  } catch {
    return DEFAULT_STREAM_DISPLAY_RATE;
  }
};

export const serializeStreamDisplayRate = (value: number): string =>
  JSON.stringify(normalizeStreamDisplayRate(value));
