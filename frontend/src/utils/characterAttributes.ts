import type { CharacterProfile } from "../api/character";
import { asJsonRecord } from "./jsonRecord";

export const CHARACTER_ATTRIBUTE_LABELS = {
  explicit: "显性属性",
  implicit: "隐性属性",
  social: "社会关系",
} as const;

const CHARACTER_ATTRIBUTE_KEYS = {
  explicit: "explicit_attributes",
  implicit: "implicit_attributes",
  social: "social_attributes",
} as const;

export type CharacterAttributeCategory = keyof typeof CHARACTER_ATTRIBUTE_LABELS;

const characterAttributeRecord = (
  profile: CharacterProfile,
  category: CharacterAttributeCategory,
) => {
  const state = asJsonRecord(profile.current_state);
  return state ? asJsonRecord(state[CHARACTER_ATTRIBUTE_KEYS[category]]) : null;
};

export const characterAttributeEntries = (
  profile: CharacterProfile,
  category: CharacterAttributeCategory,
): Array<[string, unknown]> => {
  const attributes = characterAttributeRecord(profile, category);
  return attributes ? Object.entries(attributes) : [];
};

export const characterAttributeText = (
  profile: CharacterProfile,
  category: CharacterAttributeCategory,
  key: string,
): string | null => {
  const value = characterAttributeRecord(profile, category)?.[key];
  if (typeof value !== "string") return null;
  return value.trim() || null;
};

export const characterAttributeValueText = (value: unknown): string => {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value.map(characterAttributeValueText).filter(Boolean).join("、");
  }
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
};
