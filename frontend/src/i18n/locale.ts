import {
  readPortableItem,
  removePortableItem,
  writePortableItem,
} from "../utils/portableStorage";

export type Locale = "zh-CN" | "en-US";

export const LOCALE_STORAGE_KEY = "a4n:settings:locale";

let activeLocale: Locale | null = null;

export const setActiveLocale = (locale: Locale): void => {
  activeLocale = locale;
};

export const getActiveLocale = (): Locale => activeLocale ?? resolveInitialLocale();

export const normalizeLocale = (value: string | null | undefined): Locale | null => {
  const normalized = value?.trim().toLowerCase();
  if (!normalized) return null;
  if (normalized === "en" || normalized.startsWith("en-")) return "en-US";
  if (normalized === "zh" || normalized.startsWith("zh-")) return "zh-CN";
  return null;
};

export const localeFromNavigator = (
  languages: readonly string[] | undefined,
  language: string | undefined,
): Locale => {
  for (const candidate of languages ?? []) {
    const locale = normalizeLocale(candidate);
    if (locale) return locale;
  }
  return normalizeLocale(language) ?? "zh-CN";
};

export const readLocalePreference = (): Locale | null => {
  try {
    return normalizeLocale(readPortableItem(LOCALE_STORAGE_KEY));
  } catch {
    return null;
  }
};

export const resolveInitialLocale = (): Locale => {
  if (typeof navigator === "undefined") return readLocalePreference() ?? "zh-CN";
  return (
    readLocalePreference() ??
    localeFromNavigator(navigator.languages, navigator.language)
  );
};

export const writeLocalePreference = async (locale: Locale): Promise<void> => {
  await writePortableItem(LOCALE_STORAGE_KEY, locale);
};

export const clearLocalePreference = (): Promise<void> => {
  return removePortableItem(LOCALE_STORAGE_KEY);
};
