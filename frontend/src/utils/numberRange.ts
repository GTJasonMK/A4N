export const isSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value);

export const isPositiveSafeInteger = (value: unknown): value is number =>
  isSafeInteger(value) && value > 0;

export const clampFiniteRounded = (
  value: number,
  min: number,
  max: number,
  fallback: number,
): number => {
  if (!Number.isFinite(value)) return fallback;
  return Math.min(max, Math.max(min, Math.round(value)));
};
