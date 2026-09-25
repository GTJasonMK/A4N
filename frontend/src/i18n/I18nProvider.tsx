import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  resolveInitialLocale,
  setActiveLocale,
  writeLocalePreference,
  type Locale,
} from "./locale";
import {
  translateMessage,
  type MessageKey,
  type MessageParams,
} from "./translate";
import { extractErrorMessage } from "../utils/errorMessage";
import { useToast } from "../components/feedback/toastStore";

export interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: MessageKey, params?: MessageParams) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export const I18nProvider = ({
  children,
  initialLocale,
}: {
  children: ReactNode;
  initialLocale?: Locale;
}) => {
  const [locale, setLocaleState] = useState<Locale>(
    () => initialLocale ?? resolveInitialLocale(),
  );

  const setLocale = useCallback(async (nextLocale: Locale) => {
    try {
      await writeLocalePreference(nextLocale);
      setLocaleState(nextLocale);
      setActiveLocale(nextLocale);
    } catch (error) {
      useToast.getState().addToast(
        extractErrorMessage(error, translateMessage(nextLocale, "errors.saveFailed")),
        "error",
      );
    }
  }, []);

  useEffect(() => {
    setActiveLocale(locale);
    document.documentElement.lang = locale;
  }, [locale]);

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      setLocale,
      t: (key, params) => translateMessage(locale, key, params),
    }),
    [locale, setLocale],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
};

export const useI18n = (): I18nContextValue => {
  const value = useContext(I18nContext);
  if (!value) throw new Error("useI18n must be used inside I18nProvider");
  return value;
};
