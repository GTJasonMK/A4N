export type UiDateInput = string | number | Date | null | undefined;

const parseDate = (value: UiDateInput): Date | null => {
  if (value == null || value === "") return null;
  const date = value instanceof Date ? value : new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
};

const shouldIncludeYear = (date: Date): boolean =>
  date.getFullYear() !== new Date().getFullYear();

/**
 * 面向普通界面的紧凑时间：当年省略年份，固定中文 24 小时制。
 */
export const formatUiDateTime = (
  value: UiDateInput,
  fallback = "—",
): string => {
  const date = parseDate(value);
  if (!date) return fallback;
  return date.toLocaleString("zh-CN", {
    year: shouldIncludeYear(date) ? "numeric" : undefined,
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
};

/** 面向列表元信息的紧凑日期；跨年时自动补全年份。 */
export const formatUiDate = (value: UiDateInput, fallback = "—"): string => {
  const date = parseDate(value);
  if (!date) return fallback;
  return date.toLocaleDateString("zh-CN", {
    year: shouldIncludeYear(date) ? "numeric" : undefined,
    month: "2-digit",
    day: "2-digit",
  });
};
