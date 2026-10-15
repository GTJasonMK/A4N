export function containsUnrepresentableJsonNumber(value: unknown): boolean {
  if (typeof value === "number") {
    return !Number.isFinite(value) || (Number.isInteger(value) && !Number.isSafeInteger(value));
  }
  if (Array.isArray(value)) return value.some(containsUnrepresentableJsonNumber);
  if (value !== null && typeof value === "object") {
    return Object.values(value).some(containsUnrepresentableJsonNumber);
  }
  return false;
}
