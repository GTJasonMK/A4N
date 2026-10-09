import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { CheckCircle2, Clipboard, Copy, RotateCcw, XCircle } from "lucide-react";
import type {
  ModelRequestPreview,
  ModelRequestPreviewMessage,
  ModelRequestPreviewStep,
} from "../../api/requestPreview";
import { extractErrorMessage } from "../../api";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../ui/BookButton";
import { Modal } from "../ui/Modal";
import { copyTextToClipboard } from "../../utils/clipboard";
import { useToast } from "./Toast";
import { formatNumber, useI18n, type Locale, type MessageTranslator } from "../../i18n";

interface RequestPreviewModalProps {
  open: boolean;
  loading: boolean;
  data: ModelRequestPreview | null;
  recorded?: boolean;
  fallbackTitle?: string;
  confirmBusy?: boolean;
  confirmDisabled?: boolean;
  confirmLabel?: string;
  onConfirm?: () => void | Promise<void>;
  onClose: () => void;
}

const parseJsonLikeString = (value: string): unknown => {
  const trimmed = value.trim();
  if (!trimmed) return value;
  const first = trimmed[0];
  const last = trimmed[trimmed.length - 1];
  if (!((first === "{" && last === "}") || (first === "[" && last === "]"))) {
    return value;
  }
  try {
    return JSON.parse(trimmed) as unknown;
  } catch {
    return value;
  }
};

const normalizedPreviewValue = (value: unknown): unknown =>
  typeof value === "string" ? parseJsonLikeString(value) : value;

const textValue = (value: unknown): string => {
  const normalized = normalizedPreviewValue(value);
  if (typeof normalized === "string") return normalized;
  if (normalized == null) return "";
  return JSON.stringify(normalized, null, 2) ?? "";
};

const jsonTextValue = (value: unknown): string => {
  const normalized = normalizedPreviewValue(value);
  if (normalized == null) return "";
  return JSON.stringify(normalized, null, 2) ?? "";
};

const stateLabel = (state: string, t: MessageTranslator): string => {
  if (state === "ready") return t("feedback.previewReady");
  if (state === "reused") return t("feedback.reused");
  return t("feedback.unavailable");
};

const messageRoleLabel = (role: unknown, t: MessageTranslator): string => {
  if (role === "system") return t("feedback.systemMessage");
  if (role === "user") return t("feedback.userMessage");
  if (role === "assistant") return t("feedback.assistantMessage");
  if (role === "tool") return t("feedback.toolMessage");
  return t("feedback.message");
};

const stateClass = (state: string): string => {
  if (state === "ready") return "border-success/35 bg-success-bg text-success";
  if (state === "reused") return "border-info/35 bg-info-bg text-info";
  return "border-warning/35 bg-warning-bg text-warning";
};

const stateTextClass = (state: string): string => {
  if (state === "ready") return "text-success";
  if (state === "reused") return "text-info";
  return "text-warning";
};

const stateIcon = (state: string) => {
  if (state === "ready") return <CheckCircle2 size={12} />;
  if (state === "reused") return <RotateCcw size={12} />;
  return <XCircle size={12} />;
};

const messagesArray = (value: unknown): ModelRequestPreviewMessage[] =>
  Array.isArray(value) ? (value as ModelRequestPreviewMessage[]) : [];

const requestObject = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;

const stringRecordField = (value: unknown, field: string): string | null => {
  const raw = requestObject(value)?.[field];
  return typeof raw === "string" && raw.trim() ? raw : null;
};

const numericField = (value: unknown, field: string): number | null => {
  const raw = requestObject(value)?.[field];
  return typeof raw === "number" && Number.isFinite(raw) ? raw : null;
};

const lengthLabel = (
  chars: number | null,
  bytes: number | null,
  locale: Locale,
  t: MessageTranslator,
): string | null => {
  if (chars == null && bytes == null) return null;
  if (chars != null && bytes != null) {
    return t("feedback.charactersAndBytes", {
      characters: formatNumber(chars, locale),
      bytes: formatNumber(bytes, locale),
    });
  }
  if (chars != null) return t("feedback.characters", { count: formatNumber(chars, locale) });
  return t("feedback.bytes", { count: formatNumber(bytes ?? 0, locale) });
};

const StepState: React.FC<{
  state: string;
  recorded: boolean;
  t: MessageTranslator;
}> = ({ state, recorded, t }) => (
  <span
    className={`inline-flex shrink-0 items-center gap-1.5 rounded-md border px-2 py-1 text-caption font-medium ${
      recorded ? "border-success/35 bg-success-bg text-success" : stateClass(state)
    }`}
  >
    {recorded ? <CheckCircle2 size={12} /> : stateIcon(state)}
    {recorded ? t("feedback.requestRecordedState") : stateLabel(state, t)}
  </span>
);

const PromptBlock: React.FC<{
  label: string;
  content: string;
  copyContent?: string;
  copyKey: string;
  copiedKey: string | null;
  code?: boolean;
  onCopy: (text: string, key: string) => void;
  t: MessageTranslator;
}> = ({ label, content, copyContent, copyKey, copiedKey, code = false, onCopy, t }) => {
  if (!content.trim()) return null;
  const raw = copyContent ?? content;
  const copyLabel = t("feedback.copyLabel", { label });
  return (
    <section className="min-w-0 overflow-hidden rounded-md border border-book-border/45 bg-book-bg-secondary/35">
      <div className="flex min-w-0 items-center justify-between gap-3 border-b border-book-border/40 px-3 py-2">
        <span className="min-w-0 truncate text-xs font-semibold text-book-text-main">{label}</span>
        <BookIconButton
          size="sm"
          label={copyLabel}
          variant="ghost"
          onClick={() => void onCopy(raw, copyKey)}
        >
          {copiedKey === copyKey ? <CheckCircle2 size={13} /> : <Copy size={13} />}
        </BookIconButton>
      </div>
      <pre className={`max-h-[min(42vh,32rem)] overflow-auto whitespace-pre-wrap break-words px-3 py-3 text-body-sm leading-relaxed text-book-text-main [overflow-wrap:anywhere] ${code ? "font-mono" : "font-sans"}`}>
        {content}
      </pre>
    </section>
  );
};

const StepView: React.FC<{
  step: ModelRequestPreviewStep;
  stepIndex: number;
  recorded: boolean;
  copiedKey: string | null;
  onCopy: (text: string, key: string) => void;
}> = ({ step, stepIndex, recorded, copiedKey, onCopy }) => {
  const { locale, t } = useI18n();
  const request = normalizedPreviewValue(step.request);
  const contextManifest = normalizedPreviewValue(step.context_manifest);
  const contextPlan = normalizedPreviewValue(step.context_plan);
  const contextPackRefs = normalizedPreviewValue(step.context_pack_refs);
  const messages = messagesArray(step.messages);
  const systemPrompt = textValue(step.system_prompt);
  const userPrompt = textValue(step.user_prompt);
  const requestJson = jsonTextValue(request);
  const contextManifestJson = jsonTextValue(contextManifest);
  const contextPlanJson = jsonTextValue(contextPlan);
  const contextPackRefsJson = jsonTextValue(contextPackRefs);
  const maxTokens = numericField(request, "max_tokens");
  const modelName = stringRecordField(request, "model");
  const providerName = stringRecordField(request, "provider");
  const contextAvailableInputTokens = numericField(contextPlan, "available_input_tokens");
  const contextEstimatedInputTokens = numericField(contextPlan, "estimated_input_tokens");
  const contextStablePrefixTokens = numericField(contextPlan, "stable_prefix_tokens");
  const contextDynamicDeltaTokens = numericField(contextPlan, "dynamic_delta_tokens");
  const promptLength = lengthLabel(
    numericField(step.char_counts, "prompt_chars"),
    numericField(step.char_counts, "prompt_bytes"),
    locale,
    t,
  );
  const assemblySource = step.prompt_assembly_source
    ? t(
        step.prompt_assembly_source === "project"
          ? "feedback.promptAssemblyProject"
          : step.prompt_assembly_source === "global"
            ? "feedback.promptAssemblyGlobal"
            : "feedback.promptAssemblyDefault",
      )
    : null;
  const assemblyBlocks = step.prompt_assembly_blocks ?? [];
  const requestLength = lengthLabel(
    numericField(step.char_counts, "request_chars"),
    numericField(step.char_counts, "request_bytes"),
    locale,
    t,
  );
  const metadata = [
    modelName ? { label: t("feedback.modelLabel"), value: modelName } : null,
    providerName ? { label: t("feedback.providerLabel"), value: providerName } : null,
    step.prompt_name ? { label: t("feedback.templateLabel"), value: step.prompt_name } : null,
    assemblySource ? { label: t("feedback.promptAssemblySourceLabel"), value: assemblySource } : null,
    assemblyBlocks.length > 0
      ? {
          label: t("feedback.promptAssemblyBlocksLabel"),
          value: assemblyBlocks.join(t("common.listSeparator")),
        }
      : null,
    promptLength ? { label: t("feedback.promptLengthLabel"), value: promptLength } : null,
    requestLength ? { label: t("feedback.requestLengthLabel"), value: requestLength } : null,
    maxTokens != null ? { label: t("feedback.outputTokenLimitLabel"), value: formatNumber(maxTokens, locale) } : null,
  ].filter((item): item is { label: string; value: string } => item != null);
  const hasContextDetails = Boolean(
    contextPlanJson.trim() || contextPackRefsJson.trim() || contextManifestJson.trim(),
  );
  const hasPromptText = messages.length > 0 || systemPrompt.trim() || userPrompt.trim();

  return (
    <section id={`request-preview-step-${stepIndex}`} className="min-w-0 space-y-4">
      <header className="flex min-w-0 flex-wrap items-start justify-between gap-3 border-b border-book-border/45 pb-3">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold leading-relaxed text-book-text-main">{step.title}</h2>
          {step.prompt_name ? <p className="mt-1 truncate text-small text-book-text-muted">{t("feedback.template", { name: step.prompt_name })}</p> : null}
        </div>
        <StepState state={step.state} recorded={recorded} t={t} />
      </header>

      {step.note ? (
        <div className={`rounded-md border px-3 py-2.5 text-small leading-relaxed ${recorded ? "border-book-border/45 bg-book-bg-secondary/35 text-book-text-muted" : "border-warning/35 bg-warning-bg text-warning"}`}>
          {step.note}
        </div>
      ) : null}

      {metadata.length > 0 ? (
        <dl className="grid min-w-0 grid-cols-2 gap-3 border-y border-book-border/40 py-3 sm:grid-cols-3 xl:grid-cols-4">
          {metadata.map((item) => (
            <Metric key={`${item.label}:${item.value}`} label={item.label} value={item.value || "—"} />
          ))}
        </dl>
      ) : null}

      {contextAvailableInputTokens != null && contextEstimatedInputTokens != null ? (
        <ContextBudget
          estimated={contextEstimatedInputTokens}
          available={contextAvailableInputTokens}
          stablePrefix={contextStablePrefixTokens}
          dynamicDelta={contextDynamicDeltaTokens}
          locale={locale}
          t={t}
        />
      ) : null}

      <section className="min-w-0 space-y-3">
        <div className="flex min-w-0 items-center justify-between gap-3">
          <h3 className="text-xs font-semibold text-book-text-main">{t("feedback.promptMessages")}</h3>
          {messages.length > 0 ? (
            <span className="shrink-0 text-caption tabular-nums text-book-text-muted">
              {t("feedback.messageCount", { count: formatNumber(messages.length, locale) })}
            </span>
          ) : null}
        </div>
        {messages.length > 0 ? (
          <div className="grid min-w-0 gap-3">
            {messages.map((message, index) => {
              const label = `${messageRoleLabel(message.role, t)} ${formatNumber(index + 1, locale)}`;
              return (
                <PromptBlock
                  key={`${step.key}-${index}`}
                  label={label}
                  content={textValue(message.content ?? message)}
                  copyKey={`${step.key}:message:${index}`}
                  copiedKey={copiedKey}
                  onCopy={onCopy}
                  t={t}
                />
              );
            })}
          </div>
        ) : systemPrompt.trim() || userPrompt.trim() ? (
          <div className="grid min-w-0 gap-3 xl:grid-cols-2">
            <PromptBlock
              label={t("feedback.systemPrompt")}
              content={systemPrompt}
              copyKey={`${step.key}:system`}
              copiedKey={copiedKey}
              onCopy={onCopy}
              t={t}
            />
            <PromptBlock
              label={t("feedback.userPrompt")}
              content={userPrompt}
              copyKey={`${step.key}:user`}
              copiedKey={copiedKey}
              onCopy={onCopy}
              t={t}
            />
          </div>
        ) : (
          <p className="border-l-2 border-book-border/60 pl-3 text-small text-book-text-muted">
            {t("feedback.noPromptText")}
          </p>
        )}
      </section>

      {requestJson.trim() ? (
        <details open={recorded} className="min-w-0 border-t border-book-border/40 pt-3 text-small">
          <summary className="min-h-11 w-fit cursor-pointer py-3 font-semibold text-book-text-sub hover:text-book-text-main">
            {t("feedback.fullRequest")}
          </summary>
          <div className="mt-3">
            <PromptBlock
              label={t("feedback.fullRequest")}
              content={requestJson}
              copyContent={requestJson}
              copyKey={`${step.key}:request`}
              copiedKey={copiedKey}
              code
              onCopy={onCopy}
              t={t}
            />
          </div>
        </details>
      ) : null}

      {hasContextDetails ? (
        <details className="min-w-0 border-t border-book-border/40 pt-3 text-small">
          <summary className="min-h-11 w-fit cursor-pointer py-3 font-semibold text-book-text-sub hover:text-book-text-main">
            {t("feedback.contextDetails")}
          </summary>
          <div className="mt-3 grid min-w-0 gap-3 xl:grid-cols-3">
            {contextPlanJson.trim() ? (
              <PromptBlock
                label={t("feedback.budget")}
                content={contextPlanJson}
                copyContent={contextPlanJson}
                copyKey={`${step.key}:context-plan`}
                copiedKey={copiedKey}
                code
                onCopy={onCopy}
                t={t}
              />
            ) : null}
            {contextPackRefsJson.trim() ? (
              <PromptBlock
                label={t("feedback.references")}
                content={contextPackRefsJson}
                copyContent={contextPackRefsJson}
                copyKey={`${step.key}:context-refs`}
                copiedKey={copiedKey}
                code
                onCopy={onCopy}
                t={t}
              />
            ) : null}
            {contextManifestJson.trim() ? (
              <PromptBlock
                label={t("feedback.manifest")}
                content={contextManifestJson}
                copyContent={contextManifestJson}
                copyKey={`${step.key}:context-manifest`}
                copiedKey={copiedKey}
                code
                onCopy={onCopy}
                t={t}
              />
            ) : null}
          </div>
        </details>
      ) : null}

      {!hasPromptText && !requestJson.trim() && !hasContextDetails ? (
        <div className="border-t border-book-border/40 pt-3 text-small text-book-text-muted">
          {t("feedback.noPreviewData")}
        </div>
      ) : null}
    </section>
  );
};

const StepNavigation: React.FC<{
  steps: ModelRequestPreviewStep[];
  activeIndex: number;
  recorded: boolean;
  onSelect: (index: number) => void;
  t: MessageTranslator;
}> = ({ steps, activeIndex, recorded, onSelect, t }) => {
  const { locale } = useI18n();
  const buttons = steps.map((step, index) => (
    <StepNavigationButton
      key={`${step.key}:${index}`}
      step={step}
      index={index}
      active={index === activeIndex}
      recorded={recorded}
      onClick={() => onSelect(index)}
      t={t}
      locale={locale}
    />
  ));
  return (
    <>
      <nav aria-label={t("feedback.stepNavigation")} className="flex min-w-0 gap-2 overflow-x-auto pb-1 custom-scrollbar xl:hidden">
        {buttons}
      </nav>
      <aside className="hidden min-w-0 xl:block">
        <div className="sticky top-0 max-h-[78vh] overflow-y-auto custom-scrollbar">
          <h2 className="mb-2 text-caption font-semibold uppercase text-book-text-muted">
            {t("feedback.stepNavigation")}
          </h2>
          <nav aria-label={t("feedback.stepNavigation")} className="grid min-w-0 gap-1.5">
            {buttons}
          </nav>
        </div>
      </aside>
    </>
  );
};

const StepNavigationButton: React.FC<{
  step: ModelRequestPreviewStep;
  index: number;
  active: boolean;
  recorded: boolean;
  onClick: () => void;
  t: MessageTranslator;
  locale: Locale;
}> = ({ step, index, active, recorded, onClick, t, locale }) => (
  <button
    type="button"
    aria-current={active ? "step" : undefined}
    onClick={onClick}
    className={`flex basis-36 shrink-0 max-w-[18rem] items-start gap-2 rounded-md border p-2.5 text-left transition-colors xl:min-w-0 xl:max-w-none ${
      active
        ? "border-book-primary/45 bg-book-bg-secondary text-book-text-main"
        : "border-transparent text-book-text-sub hover:border-book-border/45 hover:bg-book-bg-secondary/55"
    }`}
  >
    <span className="flex size-6 shrink-0 items-center justify-center rounded-full bg-book-bg-tertiary text-caption font-semibold tabular-nums text-book-text-muted">
      {formatNumber(index + 1, locale)}
    </span>
    <span className="min-w-0 flex-1">
      <span className="block line-clamp-2 text-xs font-semibold leading-relaxed">
        {step.title}
      </span>
      <span className="mt-1 flex min-w-0 items-center gap-1.5 text-caption">
        <span className={recorded ? "text-success" : stateTextClass(step.state)}>
          {recorded ? <CheckCircle2 size={11} /> : stateIcon(step.state)}
        </span>
        <span className="truncate">{recorded ? t("feedback.requestRecordedState") : stateLabel(step.state, t)}</span>
        {step.prompt_name ? <span className="truncate text-book-text-muted">· {step.prompt_name}</span> : null}
      </span>
    </span>
  </button>
);

const Metric: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="min-w-0 border-l border-book-border/50 pl-3 first:border-l-0 first:pl-0">
    <dt className="text-caption text-book-text-muted">{label}</dt>
    <dd className="mt-1 truncate text-small font-semibold tabular-nums text-book-text-main" title={value}>
      {value}
    </dd>
  </div>
);

const ContextBudget: React.FC<{
  estimated: number;
  available: number;
  stablePrefix: number | null;
  dynamicDelta: number | null;
  locale: Locale;
  t: MessageTranslator;
}> = ({ estimated, available, stablePrefix, dynamicDelta, locale, t }) => {
  if (available <= 0) return null;
  const used = Math.max(0, estimated);
  const ratio = Math.min(1, used / available);
  const hasBreakdown = stablePrefix != null || dynamicDelta != null;
  const stableTokens = hasBreakdown
    ? Math.min(used, Math.max(0, stablePrefix ?? 0))
    : 0;
  const dynamicTokens = Math.min(
    Math.max(0, used - stableTokens),
    Math.max(0, dynamicDelta ?? (stablePrefix != null ? used - stableTokens : 0)),
  );
  const otherTokens = Math.max(0, used - stableTokens - dynamicTokens);
  const over = used > available;
  const percent = Math.round(ratio * 100);
  const stableWidth = ratio > 0 ? Math.round((stableTokens / available) * 100 / ratio) : 0;
  const dynamicWidth = ratio > 0 ? Math.round((dynamicTokens / available) * 100 / ratio) : 0;
  const otherWidth = Math.max(0, 100 - stableWidth - dynamicWidth);

  return (
    <section className="min-w-0 rounded-md border border-book-border/45 bg-book-bg-secondary/35 px-3 py-3">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="text-xs font-semibold text-book-text-main">{t("feedback.contextBudget")}</h3>
        <span className={`text-small font-semibold tabular-nums ${over ? "text-warning" : "text-book-text-sub"}`}>
          {t("feedback.contextTokens", {
            estimated: formatNumber(used, locale),
            available: formatNumber(available, locale),
          })}
        </span>
      </div>
      <div
        className="mt-2 h-2.5 overflow-hidden rounded-full bg-book-bg-tertiary"
        role="progressbar"
        aria-label={t("feedback.contextBudget")}
        aria-valuemin={0}
        aria-valuemax={available}
        aria-valuenow={Math.min(used, available)}
        aria-valuetext={t("feedback.contextTokens", {
          estimated: formatNumber(used, locale),
          available: formatNumber(available, locale),
        })}
      >
        <div className={`flex h-full ${over ? "bg-warning" : ""}`} style={{ width: `${percent}%` }}>
          {stableTokens > 0 ? <span className="h-full bg-book-primary" style={{ width: `${stableWidth}%` }} /> : null}
          {dynamicTokens > 0 ? <span className="h-full bg-book-accent" style={{ width: `${dynamicWidth}%` }} /> : null}
          {otherTokens > 0 ? <span className="h-full bg-info" style={{ width: `${otherWidth}%` }} /> : null}
        </div>
      </div>
      <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-caption text-book-text-muted">
        {stablePrefix != null ? <span className="inline-flex items-center gap-1.5"><i className="size-2 rounded-sm bg-book-primary" />{t("feedback.stablePrefix", { value: formatNumber(stablePrefix, locale) })}</span> : null}
        {dynamicDelta != null ? <span className="inline-flex items-center gap-1.5"><i className="size-2 rounded-sm bg-book-accent" />{t("feedback.dynamicTail", { value: formatNumber(dynamicDelta, locale) })}</span> : null}
        {over ? <span className="text-warning">{t("feedback.contextBudgetExceeded", { count: formatNumber(used - available, locale) })}</span> : null}
      </div>
    </section>
  );
};

const buildAllText = (data: ModelRequestPreview | null): string =>
  data
    ? data.steps
        .map((step) =>
          [
            `## ${step.title}${step.prompt_name ? ` (${step.prompt_name})` : ""}`,
            step.prompt_assembly_source
              ? `Assembly source: ${step.prompt_assembly_source} [${(step.prompt_assembly_blocks ?? []).join(", ")}]`
              : null,
            step.note ? `Note: ${step.note}` : null,
            step.system_prompt ? `### System\n${textValue(step.system_prompt)}` : null,
            step.user_prompt ? `### User / Prompt\n${textValue(step.user_prompt)}` : null,
            step.messages ? `### Messages\n${textValue(step.messages)}` : null,
            step.request ? `### Request\n${jsonTextValue(step.request)}` : null,
            step.context_plan ? `### Context Plan\n${jsonTextValue(step.context_plan)}` : null,
            step.context_pack_refs ? `### Context Pack Refs\n${jsonTextValue(step.context_pack_refs)}` : null,
            step.context_manifest ? `### Context Manifest\n${jsonTextValue(step.context_manifest)}` : null,
          ]
            .filter(Boolean)
            .join("\n\n"),
        )
        .join("\n\n")
    : "";

export const RequestPreviewModal: React.FC<RequestPreviewModalProps> = ({
  open,
  loading,
  data,
  recorded = false,
  fallbackTitle,
  confirmBusy = false,
  confirmDisabled = false,
  confirmLabel,
  onConfirm,
  onClose,
}) => {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [activeStepIndex, setActiveStepIndex] = useState(0);
  const copiedTimerRef = useRef<number | null>(null);
  const copySequenceRef = useRef(0);
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const steps = data?.steps ?? [];
  const activeStep = steps[Math.min(activeStepIndex, Math.max(0, steps.length - 1))] ?? null;
  const allText = buildAllText(data);

  useLayoutEffect(() => {
    copySequenceRef.current += 1;
    setCopiedKey(null);
    if (copiedTimerRef.current != null) {
      window.clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = null;
    }
    const readyIndex = steps.reduce(
      (lastIndex, step, index) =>
        step.state === "ready" || step.state === "reused" ? index : lastIndex,
      -1,
    );
    setActiveStepIndex(readyIndex >= 0 ? readyIndex : Math.max(0, steps.length - 1));
  }, [open, data]);

  useEffect(
    () => () => {
      copySequenceRef.current += 1;
      if (copiedTimerRef.current != null) window.clearTimeout(copiedTimerRef.current);
    },
    [],
  );

  const copyText = async (text: string, key: string) => {
    const copyRequest = ++copySequenceRef.current;
    try {
      await copyTextToClipboard(text);
      if (copySequenceRef.current !== copyRequest) return;
      setCopiedKey(key);
      if (copiedTimerRef.current != null) window.clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = window.setTimeout(() => {
        copiedTimerRef.current = null;
        setCopiedKey(null);
      }, 1200);
    } catch (err) {
      if (copySequenceRef.current !== copyRequest) return;
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
        copiedTimerRef.current = null;
      }
      setCopiedKey(null);
      addToast(extractErrorMessage(err, t("feedback.copyPreviewFailed")), "error");
    }
  };

  const contextSummary = data
    ? t(recorded ? "feedback.requestRecorded" : "feedback.requestPreviewReady", {
        ready: formatNumber(data.ready_step_count, locale),
        total: formatNumber(data.steps.length, locale),
      })
    : t(recorded ? "feedback.requestRecordedLoading" : "feedback.requestPreviewSubtitle");

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title={data?.title ?? fallbackTitle ?? t("feedback.requestPreviewTitle")}
      subtitle={contextSummary}
      maxWidthClassName="max-w-[min(96vw,86rem)]"
      className="sm:max-h-[88vh]"
      bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-3 py-3 sm:px-5 sm:py-4"
      zIndexClassName="z-[80]"
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void copyText(allText, "all")}
            disabled={!allText}
            className="!min-h-8 !text-xs"
          >
            {copiedKey === "all" ? <CheckCircle2 size={13} /> : <Clipboard size={13} />}
            {copiedKey === "all" ? t("feedback.copiedAll") : t("feedback.copyAll")}
          </BookButton>
          {confirmLabel && onConfirm ? (
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onConfirm()}
              disabled={confirmDisabled || confirmBusy}
              className="!min-h-8 !text-xs"
            >
              {confirmBusy ? <LoadingSpinner size="sm" /> : <CheckCircle2 size={13} />}
              {confirmLabel}
            </BookButton>
          ) : null}
        </div>
      }
    >
      {loading ? (
        <div className="flex min-h-[18rem] items-center justify-center">
          <div className="flex items-center gap-2 text-small text-book-text-muted">
            <LoadingSpinner size="sm" />
            {t("feedback.buildingRequest")}
          </div>
        </div>
      ) : data && activeStep ? (
        <div className={`grid min-w-0 gap-4 ${steps.length > 1 ? "xl:grid-cols-[14rem_minmax(0,1fr)]" : "grid-cols-1"}`}>
          {steps.length > 1 ? (
            <StepNavigation
              steps={steps}
              activeIndex={activeStepIndex}
              recorded={recorded}
              onSelect={setActiveStepIndex}
              t={t}
            />
          ) : null}
          <StepView
            key={`${activeStep.key}:${activeStepIndex}`}
            step={activeStep}
            stepIndex={activeStepIndex}
            recorded={recorded}
            copiedKey={copiedKey}
            onCopy={copyText}
          />
        </div>
      ) : (
        <div className="flex min-h-[14rem] items-center justify-center text-small text-book-text-muted">
          {t("feedback.noPreviewData")}
        </div>
      )}
    </Modal>
  );
};
