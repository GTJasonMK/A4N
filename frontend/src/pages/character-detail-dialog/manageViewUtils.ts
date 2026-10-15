import type { MessageKey, MessageTranslator, Messages } from "../../i18n";
import { isPositiveSafeInteger } from "../../utils/numberRange";

export const CHARACTER_DETAIL_DRAFT_SECTIONS = ["event", "deletion", "implicit", "snapshot"] as const;
export type CharacterDetailDraftSection = typeof CHARACTER_DETAIL_DRAFT_SECTIONS[number];

export const characterDetailDraftKey = (
  profileId: number,
  section: CharacterDetailDraftSection,
): string => `character-detail:${profileId}:${section}`;

export const parsePositiveChapterNumber = (raw: string): number | null => {
  const text = raw.trim();
  if (!text) return null;
  const value = Number(text);
  return isPositiveSafeInteger(value) ? value : null;
};

type CharacterLabelKey = keyof Messages["characterDetail"]["labels"];

const label = (t: MessageTranslator, key: CharacterLabelKey): string =>
  t(`characterDetail.labels.${key}` as MessageKey);

const CATEGORY_LABELS: Record<string, CharacterLabelKey> = {
  explicit: "explicit",
  implicit: "implicit",
  social: "social",
};

const OPERATION_LABELS: Record<string, CharacterLabelKey> = {
  add: "add",
  added: "add",
  modify: "modify",
  modified: "modify",
  delete: "delete",
  deleted: "delete",
};

const ATTRIBUTE_KEY_LABELS: Record<string, CharacterLabelKey> = {
  gender: "gender",
  age: "age",
  core_desire: "coreDesire",
  personality: "personality",
  personality_trait: "personalityTrait",
  emotional_state: "emotionalState",
  current_location: "currentLocation",
  current_goal: "currentGoal",
  motivation: "motivation",
  fear: "fear",
  belief: "belief",
  trust: "trust",
  relationship: "relationship",
  relationship_status: "relationshipStatus",
  attitude: "attitude",
  occupation: "occupation",
  appearance: "appearance",
  background: "background",
  health: "health",
  skill: "skill",
  secret: "secret",
};

const ATTRIBUTE_TOKEN_LABELS: Record<string, CharacterLabelKey> = {
  personality: "personality",
  trait: "trait",
  emotional: "emotional",
  emotion: "emotion",
  state: "state",
  current: "current",
  goal: "goal",
  location: "location",
  motivation: "motivation",
  fear: "fear",
  belief: "belief",
  trust: "trust",
  relationship: "relationship",
  status: "state",
  attitude: "attitude",
  occupation: "occupation",
  appearance: "appearance",
  background: "background",
  health: "health",
  skill: "skill",
  secret: "secret",
  toward: "toward",
  towards: "toward",
};

const BEHAVIOR_TYPE_LABELS: Record<string, CharacterLabelKey> = {
  action: "action",
  dialogue: "dialogue",
  decision: "decision",
  reaction: "reaction",
  interaction: "interaction",
  emotion: "emotion",
  thought: "thought",
  movement: "movement",
  conflict: "conflict",
  discovery: "discovery",
  relationship_change: "relationshipChange",
  attitude_change: "attitudeChange",
  goal_change: "goalChange",
};

const DECISION_LABELS: Record<string, CharacterLabelKey> = {
  keep: "keep",
  modify: "modify",
  delete: "delete",
};

const SOURCE_KIND_LABELS: Record<string, CharacterLabelKey> = {
  user: "user",
  ai_sync: "aiSync",
  text_fact: "textFact",
  legacy_unknown: "legacyUnknown",
};

export const categoryLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string => {
  const key = CATEGORY_LABELS[String(value ?? "").trim().toLowerCase()];
  return key ? label(t, key) : label(t, "otherCategory");
};

export const operationLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string => {
  const key = OPERATION_LABELS[String(value ?? "").trim().toLowerCase()];
  return key ? label(t, key) : label(t, "unknownOperation");
};

export const attributeKeyLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string => {
  const text = String(value ?? "").trim();
  if (!text) return label(t, "unnamedAttribute");
  const normalized = text.toLowerCase();
  const exactKey = ATTRIBUTE_KEY_LABELS[normalized];
  if (exactKey) return label(t, exactKey);
  if (/[^\x00-\x7f]/.test(text) || /\s/.test(text)) return text;
  const tokens = normalized.split(/[_-]+/).filter(Boolean);
  if (tokens.length === 0) return text;
  return tokens
    .map((token) => {
      const key = ATTRIBUTE_TOKEN_LABELS[token];
      if (key) return label(t, key);
      return token.replace(/^\w/, (char) => char.toUpperCase());
    })
    .join(" ");
};

export const behaviorTypeLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string => {
  const key = BEHAVIOR_TYPE_LABELS[String(value ?? "").trim().toLowerCase()];
  return key ? label(t, key) : label(t, "otherBehavior");
};

export const decisionLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string => {
  const key = DECISION_LABELS[String(value ?? "").trim().toLowerCase()];
  return key ? label(t, key) : label(t, "unknownDecision");
};

export const sourceKindLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string => {
  const key = SOURCE_KIND_LABELS[String(value ?? "").trim().toLowerCase()];
  return key ? label(t, key) : label(t, "unknownSource");
};

export const ATTRIBUTE_CATEGORY_OPTIONS = [
  { value: "explicit", labelKey: "explicit" },
  { value: "implicit", labelKey: "implicit" },
  { value: "social", labelKey: "social" },
] as const;

export const ATTRIBUTE_OPERATION_OPTIONS = [
  { value: "add", labelKey: "add" },
  { value: "modify", labelKey: "modify" },
  { value: "delete", labelKey: "delete" },
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

export const listPreview = (value: unknown, t: MessageTranslator): string => {
  if (Array.isArray(value)) {
    return value
      .map(String)
      .filter(Boolean)
      .join(t("common.listSeparator"));
  }
  return jsonPreview(value);
};

export const compactJson = (value: unknown): string => {
  const text = jsonPreview(value);
  return text.length > 96 ? `${text.slice(0, 96)}...` : text;
};

export const attributeValueText = (
  value: unknown,
  t: MessageTranslator,
  depth = 0,
): string => {
  if (value == null) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value
      .map((item) => attributeValueText(item, t, depth + 1))
      .filter(Boolean)
      .join(t("common.listSeparator"));
  }
  if (isPlainRecord(value)) {
    return Object.entries(value)
      .slice(0, depth > 0 ? 6 : 10)
      .map(([key, item]) => {
        const text = attributeValueText(item, t, depth + 1);
        return text ? `${attributeKeyLabel(key, t)}: ${text}` : "";
      })
      .filter(Boolean)
      .join(t("common.listSeparator"));
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
