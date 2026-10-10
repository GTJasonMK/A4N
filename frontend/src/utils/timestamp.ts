const ISO_DATETIME_PREFIX = /^\d{4}-\d{2}-\d{2}[T ]/;
const TIMEZONE_SUFFIX = /(?:Z|[+-]\d{2}:?\d{2})$/i;

// DB datetime strings are UTC even when their serialized form has no offset.
export function parseApiTimestampMs(value: string | null | undefined): number | null {
  if (value == null) return null;
  const trimmed = value.trim();
  if (!trimmed) return null;

  const isDateTime = ISO_DATETIME_PREFIX.test(trimmed);
  const normalized = isDateTime ? trimmed.replace(" ", "T") : trimmed;
  const hasTimezone = TIMEZONE_SUFFIX.test(normalized);
  const parsed = Date.parse(isDateTime && !hasTimezone ? `${normalized}Z` : normalized);
  return Number.isFinite(parsed) ? parsed : null;
}
