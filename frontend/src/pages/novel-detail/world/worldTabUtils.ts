export type WorldObject = Record<string, unknown>;

import type { MessageKey, MessageTranslator } from "../../../i18n";

export type WorldEditMode = "structured" | "json" | "text";

interface WorldEditDraft {
  mode: WorldEditMode;
  json: string;
  text: string;
}

export type WorldListItem = {
  title?: string;
  description: string;
  raw?: unknown;
};

const SYSTEM_KEYS = new Set([
  "_discarded_chapter_outlines",
  "_blueprint_generation_notes",
  "_debug",
  "_meta",
  "metadata",
]);

const SYSTEM_KEY_SEGMENT_RE = /(?:^|_)(?:hash|checksum|schema|version|id|ids|metadata|debug|trace)(?:_|$)/i;

export const isSystemKey = (key: string): boolean =>
  key.startsWith("_") || SYSTEM_KEYS.has(key) || SYSTEM_KEY_SEGMENT_RE.test(key);

export const CORE_KEYS = ["core_rules"];
export const PREMISE_KEYS = ["premise", "overview"];
export const SYSTEM_KEYS_ORDER = ["technology_or_magic", "power_system"];
export const HISTORY_KEYS = ["history_timeline", "history"];
export const CULTURE_KEYS = ["culture_and_society", "culture"];
export const CONFLICT_KEYS = ["conflict_sources", "conflicts"];
export const MYSTERY_KEYS = ["mysteries_and_foreshadowing", "mysteries"];
export const PROMISE_KEYS = ["genre_promises"];
export const LOCATION_KEYS = ["key_locations", "locations"];
export const FACTION_KEYS = ["factions"];

const TITLE_KEYS = new Set(["name", "title", "mystery", "period"]);

const FIELD_LABEL_KEYS: Record<string, MessageKey> = Object.fromEntries(
  [
    "premise",
    "core_rules",
    "technology_or_magic",
    "power_system",
    "name",
    "source",
    "rules",
    "limits",
    "cost",
    "costs",
    "key_locations",
    "locations",
    "factions",
    "history_timeline",
    "history",
    "event",
    "impact",
    "conflict_sources",
    "conflicts",
    "stakes",
    "culture_and_society",
    "culture",
    "customs",
    "taboo",
    "values",
    "class_structure",
    "daily_life",
    "mysteries_and_foreshadowing",
    "mysteries",
    "early_clues",
    "payoff_direction",
    "genre_promises",
    "story_function",
    "agenda",
    "representative_characters",
    "description",
    "weather_cycle",
  ].map((key) => [key, `novel.world.fieldLabels.${key}` as MessageKey]),
);

export const isRecord = (value: unknown): value is WorldObject =>
  !!value && typeof value === "object" && !Array.isArray(value);

export const stringifyPretty = (value: unknown): string => {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  const json = JSON.stringify(value, null, 2);
  if (json === undefined) {
    throw new Error("世界观字段无法序列化为 JSON");
  }
  return json;
};

export const inputTextValue = (value: unknown): string => {
  if (value === null || value === undefined) return "";
  return typeof value === "string" ? value : stringifyPretty(value);
};

export const parseMaybeJsonString = (value: unknown): unknown => {
  if (typeof value !== "string") return value;
  const trimmed = value.trim();
  if (!trimmed) return "";
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) return value;
  return JSON.parse(trimmed);
};

export const pickFirst = (
  obj: WorldObject,
  keys: string[],
): [string, unknown] | null => {
  for (const key of keys) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) return [key, obj[key]];
  }
  return null;
};

export function formatFieldLabel(key: string, t: MessageTranslator): string {
  const messageKey = FIELD_LABEL_KEYS[key];
  if (messageKey) return t(messageKey);
  return key
    .replace(/^_+/, "")
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

export const valueToText = (
  value: unknown,
  t: MessageTranslator,
): string => {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean")
    return String(value);
  if (Array.isArray(value)) {
    return value
      .map((item) => {
        if (typeof item === "string") return item.trim();
        if (isRecord(item)) {
          const title = inputTextValue(
            item.name ?? item.title ?? item.mystery ?? item.period,
          ).trim();
          const details = Object.entries(item)
            .filter(([key]) => !TITLE_KEYS.has(key))
            .map(([key, child]) => {
              const text = valueToText(child, t).trim();
              return text ? `${formatFieldLabel(key, t)}: ${text}` : "";
            })
            .filter(Boolean)
            .join("\n");
          if (title && details) return `${title}：${details}`;
          if (title) return title;
          if (details) return details;
          return stringifyPretty(item);
        }
        return inputTextValue(item).trim();
      })
      .filter(Boolean)
      .join("\n");
  }
  if (isRecord(value)) {
    return Object.entries(value)
      .map(([key, child]) => {
        const text = valueToText(child, t).trim();
        return text ? `${formatFieldLabel(key, t)}: ${text}` : "";
      })
      .filter(Boolean)
      .join("\n");
  }
  return stringifyPretty(value);
};

export const normalizeList = (
  value: unknown,
  t: MessageTranslator,
): WorldListItem[] => {
  if (value === null || value === undefined) return [];
  const source = Array.isArray(value) ? value : [value];

  return source
    .map((item): WorldListItem | null => {
      if (typeof item === "string") {
        const text = item.trim();
        return text ? { title: text, description: "" } : null;
      }
      if (typeof item === "number" || typeof item === "boolean") {
        return { title: String(item), description: "" };
      }
      if (isRecord(item)) {
        const title = inputTextValue(
          item.name ?? item.title ?? item.mystery ?? item.period,
        ).trim();
        const description = Object.entries(item)
          .filter(([key]) => !TITLE_KEYS.has(key))
          .map(([key, child]) => {
            const text = valueToText(child, t).trim();
            return text ? `${formatFieldLabel(key, t)}: ${text}` : "";
          })
          .filter(Boolean)
          .join("\n");
        if (title || description) {
          return { title: title ? title : undefined, description, raw: item };
        }
        const rawText = stringifyPretty(item).trim();
        return rawText ? { description: rawText, raw: item } : null;
      }
      return null;
    })
    .filter((item): item is WorldListItem => item !== null);
};

export const namedListToEditText = (value: unknown): string => {
  const source = Array.isArray(value)
    ? value
    : value === null || value === undefined
      ? []
      : [value];
  return source
    .map((item) => {
      if (typeof item === "string") return item.trim();
      if (!isRecord(item)) return inputTextValue(item).trim();
      const name = inputTextValue(item.name).trim();
      const description = inputTextValue(item.description).trim();
      if (!name && !description) return "";
      return description ? `${name ? `${name}｜` : "｜"}${description}` : name;
    })
    .filter(Boolean)
    .join("\n");
};

export const textToNamedWorldList = (
  text: string,
  previous: unknown,
): Array<
  string | { name: string; description: string; [key: string]: unknown }
> => {
  const previousItems = Array.isArray(previous) ? previous : [];
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line, index) => {
      const sepIdx = line.includes("｜")
        ? line.indexOf("｜")
        : line.includes("|")
          ? line.indexOf("|")
          : -1;
      const name = sepIdx < 0 ? line : line.slice(0, sepIdx).trim();
      const description = sepIdx < 0 ? "" : line.slice(sepIdx + 1).trim();
      const previousItem = previousItems[index];
      if (isRecord(previousItem)) {
        return {
          ...previousItem,
          name,
          description,
        };
      }
      return description ? { name, description } : name;
    });
};

export const isEmptyValue = (value: unknown): boolean => {
  if (value === null || value === undefined) return true;
  if (typeof value === "string") return value.trim() === "";
  if (Array.isArray(value)) return value.length === 0;
  if (isRecord(value)) return Object.keys(value).length === 0;
  return false;
};

export const summarizeExtraValue = (
  value: unknown,
  t: MessageTranslator,
): string => {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean")
    return String(value);
  if (Array.isArray(value)) {
    const normalized = normalizeList(value, t);
    if (normalized.length > 0) {
      return normalized
        .map((item) =>
          item.title && item.description
            ? `${item.title}：${item.description}`
            : item.title || item.description,
        )
        .join("\n");
    }
  }
  return stringifyPretty(value);
};

export const tryParseObjectDraft = (draft: string): WorldObject | null => {
  try {
    const parsed = draft.trim() ? JSON.parse(draft) : {};
    if (!isRecord(parsed)) return null;
    return parsed;
  } catch {
    return null;
  }
};

export const createWorldEditDraft = (value: unknown): WorldEditDraft => {
  if (isRecord(value)) {
    return {
      mode: "structured",
      json: JSON.stringify(value, null, 2),
      text: "",
    };
  }

  if (value === null || value === undefined || value === "") {
    return {
      mode: "structured",
      json: JSON.stringify(
        {
          core_rules: "",
          key_locations: [],
          factions: [],
        },
        null,
        2,
      ),
      text: "",
    };
  }

  return {
    mode: "text",
    json: JSON.stringify(value, null, 2),
    text: String(value),
  };
};

export const worldSettingSaveNotice = (
  warnings: string[],
  t: MessageTranslator,
) => {
  const warning = warnings
    .map((item) => item.trim())
    .filter(Boolean)
    .join(t("common.listSeparator"));
  return warning
    ? {
        message: t("novel.world.savedWithWarnings", { warnings: warning }),
        type: "info" as const,
      }
    : {
        message: t("novel.world.saved"),
        type: "success" as const,
      };
};
