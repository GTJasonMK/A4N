import zhCN, { type Messages } from "./messages/zh-CN";
import type { Locale } from "./locale";
import enUS from "./messages/en-US";
import { formatNumber } from "./format";

export type { Messages };

type MessageKeyOf<T> = {
  [K in keyof T & string]: T[K] extends string
    ? K
    : T[K] extends Record<string, unknown>
      ? `${K}.${MessageKeyOf<T[K]>}`
      : never;
}[keyof T & string];

export type MessageKey = MessageKeyOf<Messages>;
export type MessageParams = Record<string, string | number>;
export type MessageTranslator = (
  key: MessageKey,
  params?: MessageParams,
) => string;

const dictionaries: Record<Locale, Messages> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

const lookup = (messages: Messages, key: MessageKey): string => {
  let value: unknown = messages;
  for (const segment of key.split(".")) {
    if (!value || typeof value !== "object") break;
    value = (value as Record<string, unknown>)[segment];
  }
  if (typeof value !== "string") {
    throw new Error(`Missing i18n message: ${key}`);
  }
  return value;
};

export const translateMessage = (
  locale: Locale,
  key: MessageKey,
  params: MessageParams = {},
): string => {
  const template = lookup(dictionaries[locale], key);
  const names = new Set(
    Array.from(template.matchAll(/\{([a-zA-Z0-9_]+)\}/g), (match) => match[1]),
  );
  const supplied = Object.keys(params);
  const missing = Array.from(names).filter((name) => !(name in params));
  const extra = supplied.filter((name) => !names.has(name));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `Invalid i18n params for ${key}: missing=${missing.join(",")} extra=${extra.join(",")}`,
    );
  }
  return template.replace(/\{([a-zA-Z0-9_]+)\}/g, (_, name: string) =>
    formatMessageParam(name, params[name], locale),
  );
};

const isIdentifierParam = (name: string): boolean =>
  name === "id"
  || name.endsWith("Id")
  || name.endsWith("Ids")
  || name.endsWith("_id")
  || name.endsWith("_ids");

const formatMessageParam = (
  name: string,
  value: string | number,
  locale: Locale,
): string =>
  typeof value === "number" && !isIdentifierParam(name)
    ? formatNumber(value, locale)
    : String(value);
