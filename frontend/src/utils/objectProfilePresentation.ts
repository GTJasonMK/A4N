export const OBJECT_PROFILE_TYPE_LABELS: Record<string, string> = {
  prop: "道具",
  tool: "工具",
  weapon: "武器",
  token: "信物",
  vehicle: "载具",
  clothing: "服装",
  location_object: "设施",
  environment_object: "环境物件",
  other: "其他",
};

export const OBJECT_PROFILE_IMPORTANCE_LABELS: Record<string, string> = {
  primary: "关键",
  secondary: "重要",
  minor: "次要",
};

export const getObjectProfileTypeLabel = (value: string): string =>
  value ? (OBJECT_PROFILE_TYPE_LABELS[value] ?? "其他") : "未确定";

export const getObjectProfileImportanceLabel = (value: string): string =>
  OBJECT_PROFILE_IMPORTANCE_LABELS[value] ?? "未标注";
