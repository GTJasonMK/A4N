import { describe, expect, it } from "vitest";
import {
  clearLocalePreference,
  localeFromNavigator,
  normalizeLocale,
  resolveInitialLocale,
  writeLocalePreference,
} from "./locale";
import { formatBytes, formatDate, formatDateTime, formatNumber } from "./format";
import zhCN from "./messages/zh-CN";
import enUS from "./messages/en-US";
import { translateMessage } from "./translate";

const flattenKeys = (value: Record<string, unknown>, prefix = ""): string[] =>
  Object.entries(value).flatMap(([key, child]) => {
    const full = prefix ? `${prefix}.${key}` : key;
    return typeof child === "string"
      ? [full]
      : flattenKeys(child as Record<string, unknown>, full);
  });

describe("locale resolution", () => {
  it("normalizes supported browser language ranges", () => {
    expect(normalizeLocale("en")).toBe("en-US");
    expect(normalizeLocale("en-GB")).toBe("en-US");
    expect(normalizeLocale("zh")).toBe("zh-CN");
    expect(normalizeLocale("zh-TW")).toBe("zh-CN");
    expect(normalizeLocale("ja-JP")).toBeNull();
    expect(localeFromNavigator(["ja-JP", "en-GB"], "ja-JP")).toBe("en-US");
  });

  it("prefers stored locale over navigator locale", async () => {
    const storage = new Map<string, string>();
    const localStorage = {
      get length() {
        return storage.size;
      },
      key: (index: number) => Array.from(storage.keys())[index] ?? null,
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => {
        storage.set(key, value);
      },
      removeItem: (key: string) => storage.delete(key),
      clear: () => storage.clear(),
    } as unknown as Storage;
    const previousNavigator = globalThis.navigator;
    const previousStorage = globalThis.localStorage;
    const previousWindow = globalThis.window;
    Object.defineProperty(globalThis, "navigator", {
      configurable: true,
      value: { languages: ["en-US"], language: "en-US" },
    });
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: localStorage,
    });
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: { localStorage, sessionStorage: localStorage },
    });
    await clearLocalePreference();
    expect(resolveInitialLocale()).toBe("en-US");
    await writeLocalePreference("zh-CN");
    expect(resolveInitialLocale()).toBe("zh-CN");
    await clearLocalePreference();
    Object.defineProperty(globalThis, "navigator", {
      configurable: true,
      value: previousNavigator,
    });
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: previousStorage,
    });
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: previousWindow,
    });
  });
});

describe("translation contracts", () => {
  it("keeps locale key sets identical", () => {
    const zhKeys = flattenKeys(zhCN).sort();
    const enKeys = flattenKeys(enUS).sort();
    expect(enKeys).toEqual(zhKeys);
  });

  it("interpolates and rejects invalid parameters", () => {
    expect(translateMessage("en-US", "writing.chapterLabel", { number: 3 })).toBe(
      "Chapter 3",
    );
    expect(
      translateMessage("en-US", "feedback.progressCount", {
        current: 1234567,
        total: 2000000,
      }),
    ).toBe("1,234,567/2,000,000");
    expect(() =>
      translateMessage("en-US", "writing.chapterLabel", {}),
    ).toThrow(/missing=number/);
    expect(() =>
      translateMessage("en-US", "writing.chapterLabel", { number: 3, extra: 1 }),
    ).toThrow(/extra=extra/);
  });
});

describe("locale formatting", () => {
  it("formats numbers and dates with the active locale", () => {
    expect(formatNumber(1234567, "en-US")).toBe("1,234,567");
    expect(formatNumber(1234567, "zh-CN")).toBe("1,234,567");
    expect(formatDate("2020-01-02T00:00:00Z", "en-US")).toMatch(/2020/);
    expect(formatDate("2020-01-02T00:00:00Z", "zh-CN")).toMatch(/2020/);
    expect(formatDateTime("2020-01-02 00:00:00", "en-US")).toBe(
      formatDateTime("2020-01-02T00:00:00Z", "en-US"),
    );
  });

  it("applies caller precision through the shared number formatter", () => {
    expect(
      formatNumber(1.5, "en-US", { minimumFractionDigits: 2, maximumFractionDigits: 6 }),
    ).toBe("1.50");
  });

  it("formats byte sizes consistently through terabytes", () => {
    expect(formatBytes(0, "en-US")).toBe("0 B");
    expect(formatBytes(Number.POSITIVE_INFINITY, "en-US")).toBe("0 B");
    expect(formatBytes(1536, "en-US")).toBe("1.5 KB");
    expect(formatBytes(11 * 1024, "en-US")).toBe("11 KB");
    expect(formatBytes(1024 ** 4, "en-US")).toBe("1 TB");
    expect(
      formatBytes(1024 ** 3, "en-US", { maxUnit: "MB", minimumFractionDigits: 1 }),
    ).toBe("1,024.0 MB");
  });
});
