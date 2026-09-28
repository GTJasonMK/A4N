import React, { useCallback, useEffect, useState } from "react";
import { Globe2, KeyRound, RefreshCw, Save } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  getLocalWebFetchConfig,
  getLocalWebSearchConfigSummary,
  updateLocalWebToolsConfig,
  type LocalWebFetchConfig,
  type LocalWebSearchConfigSummary,
} from "../../api/localRuntimeSettings";
import { useToast } from "../../components/feedback/Toast";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { BookInput } from "../../components/ui/BookInput";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { useI18n } from "../../i18n";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";

interface WebSearchForm {
  protocol: string;
  provider: string;
  base_url: string;
  model: string;
  max_results: string;
  max_uses: string;
  max_tokens: string;
  timeout_secs: string;
  apiKey: string;
  clearApiKey: boolean;
}

type WebFetchForm = { [K in keyof LocalWebFetchConfig]: LocalWebFetchConfig[K] extends number ? string : LocalWebFetchConfig[K] };

function formFromFetchConfig(config: LocalWebFetchConfig): WebFetchForm {
  return {
    enabled: config.enabled,
    timeout_secs: String(config.timeout_secs),
    max_redirects: String(config.max_redirects),
    max_response_bytes: String(config.max_response_bytes),
    max_body_chars: String(config.max_body_chars),
    max_output_chars: String(config.max_output_chars),
  };
}

function formFromConfig(config: LocalWebSearchConfigSummary): WebSearchForm {
  return {
    protocol: config.protocol,
    provider: config.provider,
    base_url: config.base_url,
    model: config.model,
    max_results: String(config.max_results),
    max_uses: String(config.max_uses),
    max_tokens: String(config.max_tokens),
    timeout_secs: String(config.timeout_secs),
    apiKey: "",
    clearApiKey: false,
  };
}

export const WebSearchConfigPanel: React.FC = () => {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const [config, setConfig] = useState<LocalWebSearchConfigSummary | null>(null);
  const [form, setForm] = useState<WebSearchForm | null>(null);
  const [fetchForm, setFetchForm] = useState<WebFetchForm | null>(null);
  const [savedFetch, setSavedFetch] = useState<WebFetchForm | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const nextConfig = getLocalWebSearchConfigSummary();
      setConfig(nextConfig);
      setForm(formFromConfig(nextConfig));
      const nextFetch = formFromFetchConfig(getLocalWebFetchConfig());
      setFetchForm(nextFetch);
      setSavedFetch(nextFetch);
    } catch (error) {
      setLoadError({ reason: error });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const dirty = form != null && config != null && (
    JSON.stringify(form) !== JSON.stringify(formFromConfig(config))
    || JSON.stringify(fetchForm) !== JSON.stringify(savedFetch)
  );

  const refresh = async () => {
    if (dirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("settings.webSearch"), action: t("common.refresh") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    await load();
  };

  const updateField = <K extends keyof WebSearchForm>(field: K, value: WebSearchForm[K]) => {
    setForm((current) => (current ? { ...current, [field]: value } : current));
  };

  const updateFetchField = <K extends keyof WebFetchForm>(
    field: K,
    value: WebFetchForm[K],
  ) => {
    setFetchForm((current) => (current ? { ...current, [field]: value } : current));
  };

  const save = async () => {
    if (!form || !fetchForm || saving) return;
    setSaving(true);
    try {
      const integer = (value: string, label: string): number => {
        const parsed = Number(value);
        if (!value.trim() || !Number.isSafeInteger(parsed)) {
          throw new Error(t("settingsForm.integerRequired", { label }));
        }
        return parsed;
      };
      const input = {
        protocol: form.protocol,
        provider: form.provider,
        base_url: form.base_url,
        model: form.model,
        max_results: integer(form.max_results, t("settings.webSearchMaxResults")),
        max_uses: integer(form.max_uses, t("settings.webSearchMaxUses")),
        max_tokens: integer(form.max_tokens, t("settings.webSearchMaxTokens")),
        timeout_secs: integer(form.timeout_secs, t("settings.webSearchTimeout")),
        api_key_encrypted: form.clearApiKey
          ? null
          : form.apiKey.trim() || undefined,
      };
      const saved = await updateLocalWebToolsConfig(input, {
        enabled: fetchForm.enabled,
        timeout_secs: integer(fetchForm.timeout_secs, t("settings.webFetchTimeout")),
        max_redirects: integer(fetchForm.max_redirects, t("settings.webFetchMaxRedirects")),
        max_response_bytes: integer(fetchForm.max_response_bytes, t("settings.webFetchMaxResponseBytes")),
        max_body_chars: integer(fetchForm.max_body_chars, t("settings.webFetchMaxBodyChars")),
        max_output_chars: integer(fetchForm.max_output_chars, t("settings.webFetchMaxOutputChars")),
      });
      setLoadError(null);
      setConfig(saved.search);
      setForm(formFromConfig(saved.search));
      setFetchForm(formFromFetchConfig(saved.fetch));
      setSavedFetch(formFromFetchConfig(saved.fetch));
      addToast(t("settings.webSearchSaved"), "success");
    } catch (error) {
      addToast(extractErrorMessage(error, t("settings.webSearchSaveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <section
      data-settings-dirty={dirty ? "true" : undefined}
      aria-label={t("settings.webSearch")}
      className="flex h-auto min-h-0 w-full max-w-4xl flex-col md:h-full"
    >
      <div className="flex min-w-0 shrink-0 items-start gap-2 pb-3">
        <Globe2 size={18} className="mt-0.5 shrink-0 text-book-primary" />
        <div className="min-w-0 flex-1">
          <h2 className="a4n-section-heading">
            {t("settings.webSearch")}
          </h2>
          <p className="mt-1 text-small leading-relaxed text-book-text-muted">
            {t("settings.webSearchHint")}
          </p>
        </div>
        <BookIconButton
          label={t("common.refresh")}
          variant="secondary"
          size="sm"
          onClick={() => void refresh()}
          disabled={loading || saving}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

      {loadError ? (
        <div role="alert" className="a4n-context-bar mb-3 text-sm text-book-text-main">
          <span className="min-w-0 flex-1">
            {extractErrorMessage(loadError.reason, t("settings.webSearchLoadFailed"))}
          </span>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void refresh()}
            disabled={loading || saving}
          >
            {t("common.retry")}
          </BookButton>
        </div>
      ) : null}

      <form
        id="web-tools-settings-form"
        className="flex min-h-0 flex-1 flex-col"
        onSubmit={(event) => { event.preventDefault(); void save(); }}
      >
      <div className="custom-scrollbar min-h-0 flex-1 overflow-visible pb-4 md:overflow-y-auto md:pr-2">
      {loading && (form == null || fetchForm == null) ? (
        <div className="inline-flex items-center gap-2 py-10 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("common.loading")}
        </div>
      ) : form && fetchForm ? (
        <div className="a4n-flat-surface mt-3 min-w-0 overflow-hidden">
          <div className="flex min-w-0 items-center gap-2 border-b border-book-border/30 px-3 py-2.5 text-xs">
            <KeyRound size={14} className="shrink-0 text-book-primary" />
            <span className="min-w-0 flex-1 truncate text-book-text-sub">
              {config?.has_api_key
                ? t("settings.webSearchConfigured")
                : t("settings.webSearchNotConfigured")}
            </span>
            <span className="shrink-0 text-book-text-muted">{form.provider || "—"}</span>
          </div>

          <div className="grid min-w-0 grid-cols-1 gap-3 p-3 sm:grid-cols-2">
            <label className="block min-w-0 text-xs text-book-text-sub">
              <span className="mb-1 block">{t("settings.webSearchProtocol")}</span>
              <ThemedSelect
                value={form.protocol}
                onChange={(selectedValue) => updateField("protocol", selectedValue)}
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
            <div className="sm:col-span-2">
              <BookInput
                label={t("settingsForm.model")}
                value={form.model}
                onChange={(event) => updateField("model", event.currentTarget.value)}
                disabled={saving}
                autoComplete="off"
              />
            </div>
            <div className="sm:col-span-2">
              <BookInput
                label={t("settingsForm.endpoint")}
                type="url"
                value={form.base_url}
                onChange={(event) => updateField("base_url", event.currentTarget.value)}
                disabled={saving}
                autoComplete="url"
              />
            </div>
            <div className="min-w-0 sm:col-span-2">
              <BookInput
                label={t("settingsForm.apiKey")}
                type="password"
                value={form.apiKey}
                onChange={(event) => updateField("apiKey", event.currentTarget.value)}
                disabled={form.clearApiKey || saving}
                placeholder={config?.has_api_key ? t("settings.webSearchKeySaved") : undefined}
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
                <span className="min-w-0 break-words">{t("settings.webSearchClearKey")}</span>
              </label>
              <p className="mt-2 text-small text-book-text-muted">{t("settings.webSearchKeyHint")}</p>
            </div>
            <BookInput
              label={t("settings.webSearchMaxResults")}
              type="number"
              min={1}
              max={20}
              step={1}
              value={form.max_results}
              onChange={(event) => updateField("max_results", event.currentTarget.value)}
              disabled={saving}
            />
            <BookInput
              label={t("settings.webSearchMaxUses")}
              type="number"
              min={1}
              max={20}
              step={1}
              value={form.max_uses}
              onChange={(event) => updateField("max_uses", event.currentTarget.value)}
              disabled={saving}
            />
            <BookInput
              label={t("settings.webSearchMaxTokens")}
              type="number"
              min={1}
              max={32768}
              step={1}
              value={form.max_tokens}
              onChange={(event) => updateField("max_tokens", event.currentTarget.value)}
              disabled={saving}
            />
            <BookInput
              label={t("settings.webSearchTimeout")}
              type="number"
              min={1}
              max={300}
              step={1}
              value={form.timeout_secs}
              onChange={(event) => updateField("timeout_secs", event.currentTarget.value)}
              disabled={saving}
            />


          </div>
        </div>
      ) : null}

      {fetchForm ? (
        <section className="a4n-flat-surface mt-3 min-w-0 overflow-hidden" aria-label={t("settings.webFetch")}>
          <div className="flex min-w-0 items-start gap-2 border-b border-book-border/30 px-3 py-2.5">
            <Globe2 size={16} className="mt-0.5 shrink-0 text-book-primary" />
            <div className="min-w-0 flex-1">
              <h4 className="text-xs font-semibold text-book-text-main">{t("settings.webFetch")}</h4>
              <p className="mt-1 text-small leading-relaxed text-book-text-muted">
                {t("settings.webFetchHint")}
              </p>
            </div>
          </div>
          <div className="grid min-w-0 grid-cols-1 gap-3 p-3 sm:grid-cols-2">
            <label className="inline-flex min-w-0 items-start gap-2 text-xs text-book-text-sub sm:col-span-2">
              <input
                type="checkbox"
                checked={fetchForm.enabled}
                form="web-tools-settings-form"
                onChange={(event) => updateFetchField("enabled", event.currentTarget.checked)}
                disabled={saving}
                className="mt-0.5 shrink-0 accent-[rgb(var(--color-primary))]"
              />
              <span className="min-w-0 break-words">{t("settings.webFetchEnabled")}</span>
            </label>
            <BookInput
              label={t("settings.webFetchTimeout")}
              type="number"
              form="web-tools-settings-form"
              required
              min={1}
              max={300}
              step={1}
              value={fetchForm.timeout_secs}
              onChange={(event) => updateFetchField("timeout_secs", event.currentTarget.value)}
              disabled={saving}
            />
            <BookInput
              label={t("settings.webFetchMaxRedirects")}
              type="number"
              form="web-tools-settings-form"
              required
              min={0}
              max={10}
              step={1}
              value={fetchForm.max_redirects}
              onChange={(event) => updateFetchField("max_redirects", event.currentTarget.value)}
              disabled={saving}
            />
            <BookInput
              label={t("settings.webFetchMaxResponseBytes")}
              type="number"
              form="web-tools-settings-form"
              required
              min={1}
              max={10_000_000}
              step={1}
              value={fetchForm.max_response_bytes}
              onChange={(event) => updateFetchField("max_response_bytes", event.currentTarget.value)}
              disabled={saving}
            />
            <BookInput
              label={t("settings.webFetchMaxBodyChars")}
              type="number"
              form="web-tools-settings-form"
              required
              min={1}
              max={200_000}
              step={1}
              value={fetchForm.max_body_chars}
              onChange={(event) => updateFetchField("max_body_chars", event.currentTarget.value)}
              disabled={saving}
            />
            <BookInput
              label={t("settings.webFetchMaxOutputChars")}
              type="number"
              form="web-tools-settings-form"
              required
              min={1}
              max={200_000}
              step={1}
              value={fetchForm.max_output_chars}
              onChange={(event) => updateFetchField("max_output_chars", event.currentTarget.value)}
              disabled={saving}
            />
          </div>
        </section>
      ) : null}
      </div>
      {form && fetchForm ? (
        <footer className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-t border-book-border/50 py-3">
          <p className="text-small text-book-text-muted">{t("settings.webToolsSaveHint")}</p>
          <BookButton type="submit" variant="primary" disabled={saving}>
            {saving ? <LoadingSpinner size="sm" /> : <Save size={14} />}
            {saving ? t("common.loading") : t("common.save")}
          </BookButton>
        </footer>
      ) : null}
      </form>
    </section>
  );
};
