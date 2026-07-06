export type JsonRecord = Record<string, unknown>;

export const isJsonRecord = (value: unknown): value is JsonRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

export const asJsonRecord = (value: unknown): JsonRecord | null =>
  isJsonRecord(value) ? value : null;

export const asArray = (value: unknown): unknown[] =>
  Array.isArray(value) ? value : [];
