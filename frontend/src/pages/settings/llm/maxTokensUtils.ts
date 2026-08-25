import type { LlmMaxTokensConfig } from "../../../api/config";
import { LLM_MAX_TOKEN_LIMIT } from "../../../api/generated/config";
import type { MessageKey, MessageParams } from "../../../i18n";

type Translate = (key: MessageKey, params?: MessageParams) => string;

type TokenFieldKey = keyof LlmMaxTokensConfig;

export const TOKEN_GROUPS: Array<{
  title: MessageKey;
  fields: Array<{ key: TokenFieldKey; label: MessageKey }>;
}> = [
  {
    title: "settingsForm.tokenGroupBasic",
    fields: [
      { key: "connection_test", label: "settingsForm.tokenConnectionTest" },
      { key: "short", label: "settingsForm.tokenShort" },
      { key: "standard", label: "settingsForm.tokenStandard" },
      { key: "json_standard", label: "settingsForm.tokenJsonStandard" },
      { key: "long", label: "settingsForm.tokenLong" },
      { key: "extra_long", label: "settingsForm.tokenExtraLong" },
    ],
  },
  {
    title: "settingsForm.tokenGroupGeneration",
    fields: [
      { key: "chapter_generation", label: "settingsForm.tokenChapterGeneration" },
      { key: "chapter_analysis", label: "settingsForm.tokenChapterAnalysis" },
      { key: "outline", label: "settingsForm.tokenOutline" },
      { key: "blueprint", label: "settingsForm.tokenBlueprint" },
      { key: "character_discovery", label: "settingsForm.tokenCharacterDiscovery" },
      { key: "character_sync", label: "settingsForm.tokenCharacterSync" },
      { key: "media_prompt", label: "settingsForm.tokenMediaPrompt" },
    ],
  },
];

const TOKEN_FIELD_KEYS = TOKEN_GROUPS.flatMap((group) =>
  group.fields.map((field) => field.key),
);

export function validateMaxTokensConfig(
  value: LlmMaxTokensConfig,
  t: Translate,
  modelMaxOutputTokens?: number | null,
): string | null {
  for (const key of TOKEN_FIELD_KEYS) {
    const numberValue = value[key];
    if (!Number.isInteger(numberValue) || numberValue <= 0) {
      return t("settingsForm.tokenPositive", { label: tokenLabel(key, t) });
    }
    if (numberValue > LLM_MAX_TOKEN_LIMIT) {
      return t("settingsForm.tokenTooLarge", {
        label: tokenLabel(key, t),
        limit: LLM_MAX_TOKEN_LIMIT,
      });
    }
    if (
      modelMaxOutputTokens != null &&
      Number.isInteger(modelMaxOutputTokens) &&
      modelMaxOutputTokens > 0 &&
      numberValue > modelMaxOutputTokens
    ) {
      return t("settingsForm.tokenModelLimit", {
        label: tokenLabel(key, t),
        limit: modelMaxOutputTokens,
      });
    }
  }
  return null;
}

function tokenLabel(key: TokenFieldKey, t: Translate): string {
  for (const group of TOKEN_GROUPS) {
    const field = group.fields.find((item) => item.key === key);
    if (field) return t(field.label);
  }
  return key;
}

export function maxTokensSummary(
  form: LlmMaxTokensConfig | null,
  loading: boolean,
  t: Translate,
  modelMaxOutputTokens?: number | null,
): string {
  if (loading) return t("settingsForm.loading");
  if (!form) return t("settingsForm.notLoaded");
  const modelCap = modelMaxOutputTokens
    ? ` · ${t("settingsForm.modelMaxOutput", { count: modelMaxOutputTokens })}`
    : "";
  return `${t("settingsForm.chapterBudget", { count: form.chapter_generation })} · ${t("settingsForm.outlineBudget", { count: form.outline })} · ${t("settingsForm.characterDiscoveryBudget", { count: form.character_discovery })} · ${t("settingsForm.characterSyncBudget", { count: form.character_sync })}${modelCap}`;
}
