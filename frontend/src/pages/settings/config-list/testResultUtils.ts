import { formatDateTime, type Locale } from "../../../i18n";

export const formatConfigTestTime = (
  value: string,
  options: { withSeconds?: boolean } = {},
  locale: Locale,
): string => {
  return formatDateTime(value, locale, value, {
    includeYear: false,
    includeSeconds: options.withSeconds,
  });
};
