export const CATEGORY_LABELS: Record<string, string> = {
  explicit: "显性",
  implicit: "隐性",
  social: "社会",
};

export const OPERATION_LABELS: Record<string, string> = {
  add: "新增",
  modify: "修改",
  delete: "删除",
};

export const ATTRIBUTE_CATEGORY_OPTIONS = [
  { value: "explicit", label: "显性" },
  { value: "implicit", label: "隐性" },
  { value: "social", label: "社会" },
] as const;

export const ATTRIBUTE_OPERATION_OPTIONS = [
  { value: "add", label: "新增" },
  { value: "modify", label: "修改" },
  { value: "delete", label: "删除" },
] as const;

export const jsonPreview = (value: unknown): string => {
  if (value == null) return "";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
};

export const listPreview = (value: unknown): string => {
  if (Array.isArray(value)) return value.map(String).filter(Boolean).join("、");
  return jsonPreview(value);
};

export const compactJson = (value: unknown): string => {
  const text = jsonPreview(value);
  return text.length > 96 ? `${text.slice(0, 96)}...` : text;
};

export const isPlainRecord = (value: unknown): value is Record<string, unknown> =>
  value != null && typeof value === "object" && !Array.isArray(value);

export const recordNumber = (
  record: Record<string, unknown> | null,
  key: string,
): number | null => {
  const value = record?.[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
};

export const recordString = (
  record: Record<string, unknown> | null,
  key: string,
): string | null => {
  const value = record?.[key];
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
};

export const recordArray = (
  record: Record<string, unknown> | null,
  key: string,
): unknown[] => {
  const value = record?.[key];
  return Array.isArray(value) ? value : [];
};

export const nestedRecord = (
  record: Record<string, unknown> | null,
  key: string,
): Record<string, unknown> | null => {
  const value = record?.[key];
  return isPlainRecord(value) ? value : null;
};
