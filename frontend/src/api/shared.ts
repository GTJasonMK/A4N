export type ApiId = number | string;

const describeIdValue = (id: ApiId): string => {
  if (id === "") return "空字符串";
  return typeof id === "string" ? `"${id}"` : String(id);
};

const normalizeIdLabel = (label: string | number = "API ID"): string =>
  typeof label === "string" ? label : "API ID";

export const integerNum = (id: ApiId, label: string | number = "API ID"): number => {
  const field = normalizeIdLabel(label);
  const value = typeof id === "number" ? id : Number(id);
  if (!Number.isInteger(value)) {
    throw new Error(`${field} 必须是整数：收到 ${describeIdValue(id)}`);
  }
  return value;
};

export const idNum = (id: ApiId, label: string | number = "API ID"): number => {
  const field = normalizeIdLabel(label);
  const value = integerNum(id, label);
  if (value <= 0) {
    throw new Error(`${field} 必须是正整数：收到 ${describeIdValue(id)}`);
  }
  return value;
};
