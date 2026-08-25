import type { MessageTranslator } from "../../../i18n";

export function explainMangaPipelineAiConfigError(
  message: string,
  t: MessageTranslator,
): string {
  const text = message.trim();
  if (!text) return text;

  if (text.includes("使用系统默认 LLM 需要先")) {
    return t("comic.llmApiKeyRequired");
  }

  if (
    text.includes("默认 LLM 配置鉴权失败") ||
    text.includes("invalid_api_key") ||
    text.includes("Invalid API key")
  ) {
    return t("comic.llmAuthFailed", {
      hint: t("comic.llmAuthHint"),
      raw: text,
    });
  }

  if (text.includes("默认 LLM 配置对应账号余额不足或额度耗尽")) {
    return t("comic.llmQuotaExceeded", { raw: text });
  }

  return text;
}
