import type { Locale } from "./locale";

export type UiDateInput = string | number | Date | null | undefined;

const parseDate = (value: UiDateInput): Date | null => {
  if (value == null || value === "") return null;
  const date = value instanceof Date ? value : new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
};

const shouldIncludeYear = (date: Date): boolean =>
  date.getFullYear() !== new Date().getFullYear();

export const formatDateTime = (
  value: UiDateInput,
  locale: Locale,
  fallback = "—",
): string => {
  const date = parseDate(value);
  if (!date) return fallback;
  return date.toLocaleString(locale, {
    year: shouldIncludeYear(date) ? "numeric" : undefined,
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: locale === "zh-CN" ? false : undefined,
  });
};

export const formatDate = (
  value: UiDateInput,
  locale: Locale,
  fallback = "—",
): string => {
  const date = parseDate(value);
  if (!date) return fallback;
  return date.toLocaleDateString(locale, {
    year: shouldIncludeYear(date) ? "numeric" : undefined,
    month: "2-digit",
    day: "2-digit",
  });
};

export const formatTime = (
  value: UiDateInput,
  locale: Locale,
  fallback = "—",
): string => {
  const date = parseDate(value);
  if (!date) return fallback;
  return date.toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: locale === "zh-CN" ? false : undefined,
  });
};

export const formatNumber = (
  value: number | bigint,
  locale: Locale,
  options?: Intl.NumberFormatOptions,
): string => new Intl.NumberFormat(locale, options).format(value);

export const formatRelativeTime = (
  value: UiDateInput,
  locale: Locale,
  fallback = "—",
): string => {
  const date = parseDate(value);
  if (!date) return fallback;
  const seconds = (date.getTime() - Date.now()) / 1000;
  const absolute = Math.abs(seconds);
  const unit = absolute < 60 ? "second" : absolute < 3600 ? "minute" : absolute < 86400 ? "hour" : "day";
  const divisor = unit === "second" ? 1 : unit === "minute" ? 60 : unit === "hour" ? 3600 : 86400;
  return new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(
    Math.round(seconds / divisor),
    unit,
  );
};
