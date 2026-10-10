import type { Locale } from "./locale";
import { parseApiTimestampMs } from "../utils/timestamp";

export type UiDateInput = string | number | Date | null | undefined;

const numberFormatters = {
  "zh-CN": new Intl.NumberFormat("zh-CN"),
  "en-US": new Intl.NumberFormat("en-US"),
};
const BYTE_SIZE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

type ByteSizeUnit = (typeof BYTE_SIZE_UNITS)[number];

interface ByteSizeFormatOptions {
  maxUnit?: ByteSizeUnit;
  minimumFractionDigits?: 0 | 1;
}

const relativeTimeFormatters = {
  "zh-CN": new Intl.RelativeTimeFormat("zh-CN", { numeric: "auto" }),
  "en-US": new Intl.RelativeTimeFormat("en-US", { numeric: "auto" }),
};

const parseDate = (value: UiDateInput): Date | null => {
  if (value == null || value === "") return null;
  let date: Date;
  if (value instanceof Date) {
    date = value;
  } else if (typeof value === "string") {
    const timestamp = parseApiTimestampMs(value);
    if (timestamp === null) return null;
    date = new Date(timestamp);
  } else {
    date = new Date(value);
  }
  return Number.isNaN(date.getTime()) ? null : date;
};

const shouldIncludeYear = (date: Date): boolean =>
  date.getFullYear() !== new Date().getFullYear();

export const formatDateTime = (
  value: UiDateInput,
  locale: Locale,
  fallback = "—",
  options: { includeYear?: boolean; includeSeconds?: boolean } = {},
): string => {
  const date = parseDate(value);
  if (!date) return fallback;
  const includeYear = options.includeYear ?? shouldIncludeYear(date);
  return date.toLocaleString(locale, {
    year: includeYear ? "numeric" : undefined,
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    ...(options.includeSeconds ? { second: "2-digit" as const } : {}),
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
): string => (options === undefined
  ? numberFormatters[locale]
  : new Intl.NumberFormat(locale, options)).format(value);

export const formatBytes = (
  value: number,
  locale: Locale,
  options: ByteSizeFormatOptions = {},
): string => {
  if (!Number.isFinite(value) || value <= 0) return `${formatNumber(0, locale)} B`;
  const maxUnitIndex = options.maxUnit == null
    ? BYTE_SIZE_UNITS.length - 1
    : BYTE_SIZE_UNITS.indexOf(options.maxUnit);
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < maxUnitIndex) {
    size /= 1024;
    unit += 1;
  }
  const maximumFractionDigits = unit === 0 ? 0 : 1;
  const minimumFractionDigits = unit === 0 ? 0 : options.minimumFractionDigits ?? 0;
  const formattedSize = formatNumber(size, locale, {
    minimumFractionDigits,
    maximumFractionDigits,
  });
  return `${formattedSize} ${BYTE_SIZE_UNITS[unit]}`;
};

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
  return relativeTimeFormatters[locale].format(
    Math.round(seconds / divisor),
    unit,
  );
};
