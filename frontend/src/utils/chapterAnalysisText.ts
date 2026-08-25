import { asArray, type JsonRecord } from "./jsonRecord";

export const analysisTextValue = (value: unknown): string => {
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return "";
};

export const firstAnalysisText = (
  record: JsonRecord | null,
  keys: string[],
): string => {
  if (!record) return "";
  for (const key of keys) {
    const value = analysisTextValue(record[key]);
    if (value) return value;
  }
  return "";
};

export const analysisTextList = (
  value: unknown,
  options: { limit?: number; unique?: boolean } = {},
): string[] => {
  let values = asArray(value).map(analysisTextValue).filter(Boolean);
  if (options.unique) {
    const seen = new Set<string>();
    values = values.filter((item) => {
      if (seen.has(item)) return false;
      seen.add(item);
      return true;
    });
  }
  return values.slice(0, options.limit ?? 12);
};
