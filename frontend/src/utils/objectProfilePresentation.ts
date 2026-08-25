import type { MessageKey, MessageTranslator } from "../i18n";

export const OBJECT_PROFILE_TYPE_KEYS: Record<string, MessageKey> = {
  prop: "novel.objectType.prop",
  tool: "novel.objectType.tool",
  weapon: "novel.objectType.weapon",
  token: "novel.objectType.token",
  vehicle: "novel.objectType.vehicle",
  clothing: "novel.objectType.clothing",
  location_object: "novel.objectType.locationObject",
  environment_object: "novel.objectType.environmentObject",
  other: "novel.objectType.other",
};

export const OBJECT_PROFILE_IMPORTANCE_KEYS: Record<string, MessageKey> = {
  primary: "novel.objectImportance.primary",
  secondary: "novel.objectImportance.secondary",
  minor: "novel.objectImportance.minor",
};

export const getObjectProfileTypeLabel = (
  value: string,
  t: MessageTranslator,
): string => value ? t(OBJECT_PROFILE_TYPE_KEYS[value] ?? "novel.objectType.other") : t("novel.unconfirmed");

export const getObjectProfileImportanceLabel = (
  value: string,
  t: MessageTranslator,
): string => t(OBJECT_PROFILE_IMPORTANCE_KEYS[value] ?? "novel.unlabeled");
