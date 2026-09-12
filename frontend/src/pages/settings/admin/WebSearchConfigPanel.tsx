import React, { useCallback, useEffect, useState } from "react";
import { Globe2, KeyRound, RefreshCw, Save } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  getServerWebSearchConfig,
  updateServerWebSearchConfig,
  type WebSearchConfigSummary,
} from "../../../api/config";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useI18n } from "../../../i18n";

interface WebSearchForm {
  protocol: string;
  provider: string;
  base_url: string;
  model: string;
  max_results: number;
  max_uses: number;
  max_tokens: number;
  timeout_secs: number;
  apiKey: string;
  clearApiKey: boolean;
}

function formFromConfig(config: WebSearchConfigSummary): WebSearchForm {
  return {
    protocol: config.protocol,
    provider: config.provider,
    base_url: config.base_url,
    model: config.model,
    max_results: config.max_results,
    max_uses: config.max_uses,
    max_tokens: config.max_tokens,
    timeout_secs: config.timeout_secs,
    apiKey: "",
    clearApiKey: false,
  };
}

export const WebSearchConfigPanel: React.FC = () => {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const [config, setConfig] = useState<WebSearchConfigSummary | null>(null);
  const [form, setForm] = useState<WebSearchForm | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const nextConfig = await getServerWebSearchConfig();
      setConfig(nextConfig);
      setForm(formFromConfig(nextConfig));
    } catch (error) {
      addToast(extractErrorMessage(error, t("settingsAdmin.webSearchLoadFailed")), "error");
    } finally {
      setLoading(false);
    }
  }, [addToast, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const updateField = <K extends keyof WebSearchForm>(field: K, value: WebSearchForm[K]) => {
    setForm((current) => (current ? { ...current, [field]: value } : current));
  };

  const save = async () => {
    if (!form || saving) return;
    setSaving(true);
    try {
      const saved = await updateServerWebSearchConfig({
        protocol: form.protocol,
        provider: form.provider,
        base_url: form.base_url,
        model: form.model,
        max_results: form.max_results,
        max_uses: form.max_uses,
        max_tokens: form.max_tokens,
        timeout_secs: form.timeout_secs,
        api_key_encrypted: form.clearApiKey
          ? null
          : form.apiKey.trim() || undefined,
      });
      setConfig(saved);
      setForm(formFromConfig(saved));
      addToast(t("settingsAdmin.webSearchSaved"), "success");
    } catch (error) {
      addToast(extractErrorMessage(error, t("settingsAdmin.webSearchSaveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <section aria-label={t("settingsAdmin.webSearch")} className="w-full max-w-4xl">
      <div className="flex min-w-0 items-start gap-2">
        <Globe2 size={18} className="mt-0.5 shrink-0 text-book-primary" />
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-semibold text-book-text-main">
            {t("settingsAdmin.webSearch")}
          </h3>
          <p className="mt-1 text-small leading-relaxed text-book-text-muted">
            {t("settingsAdmin.webSearchHint")}
          </p>
        </div>
        <BookIconButton
          label={t("settingsAdmin.refresh")}
          variant="secondary"
          size="sm"
          onClick={() => void load()}
          disabled={loading || saving}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

      {loading && form == null ? (
        <div className="inline-flex items-center gap-2 py-10 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("settingsAdmin.loadingConfig")}
        </div>
      ) : form ? (
        <div className="a4n-flat-surface mt-3 min-w-0 overflow-hidden">
          <div className="flex min-w-0 items-center gap-2 border-b border-book-border/30 px-3 py-2.5 text-xs">
            <KeyRound size={14} className="shrink-0 text-book-primary" />
            <span className="min-w-0 flex-1 truncate text-book-text-sub">
              {config?.has_api_key
                ? t("settingsAdmin.configured")
                : t("settingsAdmin.webSearchNotConfigured")}
            </span>
            <span className="shrink-0 text-book-text-muted">{form.provider || "—"}</span>
          </div>

          <form
            className="grid min-w-0 grid-cols-1 gap-3 p-3 sm:grid-cols-2"
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <label className="block min-w-0 text-xs text-book-text-sub">
              <span className="mb-1 block">{t("settingsAdmin.webSearchProtocol")}</span>
              <ThemedSelect
                value={form.protocol}
                onChange={(event) => updateField("protocol", event.currentTarget.value)}
                disabled={saving}
                className="pyqt-input h-9 min-h-9 w-full text-xs"
              >
                <option value="anthropic">Anthropic / DeepSeek Web Search</option>
                <option value="openai_responses">OpenAI Responses</option>
                <option value="tavily">Tavily</option>
                <option value="brave">Brave Search</option>
                <option value="serper">Serper</option>
                <option value="bing">Bing Web Search</option>
              </ThemedSelect>
            </label>
            <BookInput
              label={t("settingsForm.provider")}
              value={form.provider}
              onChange={(event) => updateField("provider", event.currentTarget.value)}
              disabled={saving}
              autoComplete="off"
            />
            <BookInput
              label={t("settingsForm.model")}
              value={form.model}
              onChange={(event) => updateField("model", event.currentTarget.value)}
              disabled={saving}
              autoComplete="off"
            />
            <BookInput
              label={t("settingsForm.endpoint")}
              type="url"
              value={form.base_url}
              onChange={(event) => updateField("base_url", event.currentTarget.value)}
              disabled={saving}
              className="sm:col-span-2"
              autoComplete="url"
            />
            <BookInput
              label={t("settingsAdmin.webSearchMaxResults")}
              type="number"
              min={1}
              max={20}
              step={1}
              value={form.max_results}
              onChange={(event) => updateField("max_results", Number(event.currentTarget.value))}
              disabled={saving}
            />
            <BookInput
              label={t("settingsAdmin.webSearchMaxUses")}
              type="number"
              min={1}
              max={20}
              step={1}
              value={form.max_uses}
              onChange={(event) => updateField("max_uses", Number(event.currentTarget.value))}
              disabled={saving}
            />
            <BookInput
              label={t("settingsAdmin.webSearchMaxTokens")}
              type="number"
              min={1}
              max={32768}
              step={1}
              value={form.max_tokens}
              onChange={(event) => updateField("max_tokens", Number(event.currentTarget.value))}
              disabled={saving}
            />
            <BookInput
              label={t("settingsAdmin.webSearchTimeout")}
              type="number"
              min={1}
              max={300}
              step={1}
              value={form.timeout_secs}
              onChange={(event) => updateField("timeout_secs", Number(event.currentTarget.value))}
              disabled={saving}
            />
            <div className="min-w-0 sm:col-span-2">
              <BookInput
                label={t("settingsForm.apiKey")}
                type="password"
                value={form.apiKey}
                onChange={(event) => updateField("apiKey", event.currentTarget.value)}
                disabled={form.clearApiKey || saving}
                placeholder={config?.has_api_key ? t("settingsAdmin.webSearchKeySaved") : undefined}
                autoComplete="new-password"
              />
              <label className="mt-2 inline-flex min-w-0 items-start gap-2 text-xs text-book-text-sub">
                <input
                  type="checkbox"
                  checked={form.clearApiKey}
                  onChange={(event) => {
                    const checked = event.currentTarget.checked;
                    updateField("clearApiKey", checked);
                    if (checked) updateField("apiKey", "");
                  }}
                  disabled={saving}
                  className="mt-0.5 shrink-0 accent-[rgb(var(--color-primary))]"
                />
                <span className="min-w-0 break-words">{t("settingsAdmin.webSearchClearKey")}</span>
              </label>
            </div>
            <div className="flex min-w-0 flex-wrap items-center justify-end gap-2 sm:col-span-2">
              <span className="mr-auto min-w-0 text-small text-book-text-muted">
                {t("settingsAdmin.webSearchKeyHint")}
              </span>
              <BookButton type="submit" variant="primary" size="sm" disabled={saving}>
                {saving ? <LoadingSpinner size="sm" /> : <Save size={14} />}
                {saving ? t("common.loading") : t("common.save")}
              </BookButton>
            </div>
          </form>
        </div>
      ) : null}
    </section>
  );
};
