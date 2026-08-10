const CATEGORY_LABELS: Record<string, string> = {
  explicit: "显性",
  implicit: "隐性",
  social: "社会",
};

const OPERATION_LABELS: Record<string, string> = {
  add: "新增",
  added: "新增",
  modify: "修改",
  modified: "修改",
  delete: "删除",
  deleted: "删除",
};

const ATTRIBUTE_KEY_LABELS: Record<string, string> = {
  personality: "性格",
  personality_trait: "性格特征",
  emotional_state: "情绪状态",
  current_location: "当前位置",
  current_goal: "当前目标",
  motivation: "动机",
  fear: "恐惧",
  belief: "信念",
  trust: "信任",
  relationship: "关系",
  relationship_status: "关系状态",
  attitude: "态度",
  occupation: "职业",
  appearance: "外貌",
  background: "背景",
  health: "健康状态",
  skill: "技能",
  secret: "秘密",
};

const ATTRIBUTE_TOKEN_LABELS: Record<string, string> = {
  personality: "性格",
  trait: "特征",
  emotional: "情绪",
  emotion: "情绪",
  state: "状态",
  current: "当前",
  goal: "目标",
  location: "位置",
  motivation: "动机",
  fear: "恐惧",
  belief: "信念",
  trust: "信任",
  relationship: "关系",
  status: "状态",
  attitude: "态度",
  occupation: "职业",
  appearance: "外貌",
  background: "背景",
  health: "健康",
  skill: "技能",
  secret: "秘密",
  toward: "对",
  towards: "对",
};

const BEHAVIOR_TYPE_LABELS: Record<string, string> = {
  action: "行动",
  dialogue: "对话",
  decision: "决定",
  reaction: "反应",
  interaction: "互动",
  emotion: "情绪表现",
  thought: "内心活动",
  movement: "移动",
  conflict: "冲突",
  discovery: "发现",
  relationship_change: "关系变化",
  attitude_change: "态度变化",
  goal_change: "目标变化",
};

const DECISION_LABELS: Record<string, string> = {
  keep: "保留",
  modify: "修改",
  delete: "删除",
};

const SOURCE_KIND_LABELS: Record<string, string> = {
  user: "用户记录",
  ai_sync: "AI 同步",
  legacy_unknown: "历史记录",
};

export const categoryLabel = (value: string | null | undefined): string =>
  CATEGORY_LABELS[String(value ?? "").trim().toLowerCase()] ?? "其他属性";

export const operationLabel = (value: string | null | undefined): string =>
  OPERATION_LABELS[String(value ?? "").trim().toLowerCase()] ?? "未知操作";

export const attributeKeyLabel = (value: string | null | undefined): string => {
  const text = String(value ?? "").trim();
  if (!text) return "未命名属性";
  const normalized = text.toLowerCase();
  if (ATTRIBUTE_KEY_LABELS[normalized]) return ATTRIBUTE_KEY_LABELS[normalized];
  if (/[^\x00-\x7f]/.test(text) || /\s/.test(text)) return text;
  const tokens = normalized.split(/[_-]+/).filter(Boolean);
  if (tokens.length === 0 || tokens.some((token) => !ATTRIBUTE_TOKEN_LABELS[token])) {
    return "其他属性";
  }
  return tokens.map((token) => ATTRIBUTE_TOKEN_LABELS[token]).join("");
};

export const behaviorTypeLabel = (value: string | null | undefined): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return BEHAVIOR_TYPE_LABELS[normalized] ?? "其他行为";
};

export const decisionLabel = (value: string | null | undefined): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return DECISION_LABELS[normalized] ?? "未知建议";
};

export const sourceKindLabel = (value: string | null | undefined): string =>
  SOURCE_KIND_LABELS[String(value ?? "").trim().toLowerCase()] ?? "未知来源";

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

export const attributeValueText = (value: unknown, depth = 0): string => {
  if (value == null) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value
      .map((item) => attributeValueText(item, depth + 1))
      .filter(Boolean)
      .join("、");
  }
  if (isPlainRecord(value)) {
    return Object.entries(value)
      .slice(0, depth > 0 ? 6 : 10)
      .map(([key, item]) => {
        const text = attributeValueText(item, depth + 1);
        return text ? `${attributeKeyLabel(key)}：${text}` : "";
      })
      .filter(Boolean)
      .join("；");
  }
  return "";
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
