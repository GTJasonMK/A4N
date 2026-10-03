import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { CheckCircle2, Clipboard, RotateCcw, XCircle } from "lucide-react";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import type {
  ModelRequestPreview,
  ModelRequestPreviewMessage,
  ModelRequestPreviewStep,
} from "../../api/requestPreview";
import { extractErrorMessage } from "../../api";
import { BookButton } from "../ui/BookButton";
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

const indent = (level: number): string => "  ".repeat(level);

const multilineString = (value: string, level: number): string => {
  const padding = indent(level);
  const bodyPadding = indent(level + 1);
  const body = value
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => `${bodyPadding}${line}`)
    .join("\n");
  return `"""\n${body}\n${padding}"""`;
};

const shouldUseMultilineString = (value: string): boolean =>
  value.includes("\n") || value.length > 180 || value.includes('\\"') || value.includes("\\n");

const readableValue = (value: unknown, level = 0): string => {
  const normalized = normalizedPreviewValue(value);
  if (normalized == null) return "null";
  if (typeof normalized === "string") {
    return shouldUseMultilineString(normalized)
      ? multilineString(normalized, level)
      : JSON.stringify(normalized);
  }
  if (
    typeof normalized === "number" ||
    typeof normalized === "boolean"
  ) {
    return JSON.stringify(normalized);
  }
  if (Array.isArray(normalized)) {
    if (normalized.length === 0) return "[]";
    const items = normalized.map(
      (item) => `${indent(level + 1)}${readableValue(item, level + 1)}`,
    );
    return `[\n${items.join(",\n")}\n${indent(level)}]`;
  }
  if (typeof normalized === "object") {
    const entries = Object.entries(normalized as Record<string, unknown>);
    if (entries.length === 0) return "{}";
    const fields = entries.map(([key, item]) => {
      const formatted = readableValue(item, level + 1);
      return `${indent(level + 1)}${JSON.stringify(key)}: ${formatted}`;
    });
    return `{\n${fields.join(",\n")}\n${indent(level)}}`;
  }
  return String(normalized);
};

const charLabel = (value: number, locale: Locale, t: MessageTranslator): string =>
  t("feedback.inputCharacters", { value: formatNumber(value, locale) });

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
  if (state === "ready") {
    return "text-success";
  }
  if (state === "reused") {
    return "text-info";
  }
  return "text-warning";
};

const stateIcon = (state: string) => {
  if (state === "ready") return <CheckCircle2 size={11} />;
  if (state === "reused") return <RotateCcw size={11} />;
  return <XCircle size={11} />;
};

const messagesArray = (value: unknown): ModelRequestPreviewMessage[] => {
  return Array.isArray(value) ? (value as ModelRequestPreviewMessage[]) : [];
};

const requestObject = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;

const numericField = (value: unknown, field: string): number | null => {
  const raw = requestObject(value)?.[field];
  if (typeof raw === "number" && Number.isFinite(raw)) return raw;
  return null;
};

const numericRecordField = (value: unknown, field: string): number | null => {
  const raw = requestObject(value)?.[field];
  if (typeof raw === "number" && Number.isFinite(raw)) return raw;
  return null;
};

const arrayLength = (value: unknown): number | null => {
  const normalized = normalizedPreviewValue(value);
  return Array.isArray(normalized) ? normalized.length : null;
};

const maxTokensLabel = (
  value: number | null,
  locale: Locale,
  t: MessageTranslator,
): string | null =>
  value == null ? null : t("feedback.outputTokenLimit", { value: formatNumber(value, locale) });

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

const PromptBlock: React.FC<{
  label: string;
  content: string;
  copyContent?: string;
  onCopy: (text: string) => void;
  t: MessageTranslator;
}> = ({ label, content, copyContent, onCopy, t }) => {
  if (!content.trim()) return null;
  const copyText = copyContent ?? content;
  return (
    <section className="min-w-0 overflow-hidden rounded-[var(--pyqt-radius-lg)] border border-book-border/50 bg-book-bg-secondary/45">
      <div className="flex min-w-0 items-center justify-between gap-3 border-b border-book-border/45 px-3 py-2.5">
        <span className="min-w-0 truncate text-xs font-semibold text-book-text-main">{label}</span>
        <button
          type="button"
          onClick={() => onCopy(copyText)}
          className="a4n-icon-button inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] text-book-text-muted transition-colors hover:bg-book-bg-tertiary hover:text-book-primary"
          title={t("feedback.copyLabel", { label })}
          aria-label={t("feedback.copyLabel", { label })}
        >
          <Clipboard size={12} />
        </button>
      </div>
      <pre className="min-h-[7rem] whitespace-pre-wrap break-words px-3 py-3 font-mono text-small leading-relaxed text-book-text-main [overflow-wrap:anywhere]">
        {content}
      </pre>
    </section>
  );
};

const StepView: React.FC<{
  step: ModelRequestPreviewStep;
  recorded: boolean;
  onCopy: (text: string) => void;
}> = ({ step, recorded, onCopy }) => {
  const { locale, t } = useI18n();
  const request = normalizedPreviewValue(step.request);
  const contextManifest = normalizedPreviewValue(step.context_manifest);
  const contextPlan = normalizedPreviewValue(step.context_plan);
  const contextPackRefs = normalizedPreviewValue(step.context_pack_refs);
  const messages = messagesArray(step.messages);
  const systemPrompt = textValue(step.system_prompt);
  const userPrompt = textValue(step.user_prompt);
  const requestJson = jsonTextValue(request);
  const requestReadable = readableValue(request);
  const contextManifestJson = jsonTextValue(contextManifest);
  const contextPlanJson = jsonTextValue(contextPlan);
  const contextPackRefsJson = jsonTextValue(contextPackRefs);
  const contextPlanReadable = readableValue(contextPlan);
  const contextManifestReadable = readableValue(contextManifest);
  const contextPackRefsReadable = readableValue(contextPackRefs);
  const maxTokens = numericField(request, "max_tokens");
  const contextAvailableInputTokens = numericField(contextPlan, "available_input_tokens");
  const contextEstimatedInputTokens = numericField(contextPlan, "estimated_input_tokens");
  const contextStablePrefixTokens = numericField(contextPlan, "stable_prefix_tokens");
  const contextDynamicDeltaTokens = numericField(contextPlan, "dynamic_delta_tokens");
  const contextDroppedOptionalPacks = arrayLength(requestObject(contextPlan)?.dropped_optional_pack_refs);
  const contextPackCount = arrayLength(contextPackRefs);
  const promptLength = lengthLabel(
    numericRecordField(step.char_counts, "prompt_chars"),
    numericRecordField(step.char_counts, "prompt_bytes"),
    locale,
    t,
  );
  const requestLength = lengthLabel(
    numericRecordField(step.char_counts, "request_chars"),
    numericRecordField(step.char_counts, "request_bytes"),
    locale,
    t,
  );
  return (
    <section className="min-w-0">
      <div className="mb-3 flex min-w-0 items-center gap-2 border-b border-book-border/45 pb-2 text-xs">
        <span className="min-w-0 flex-1 truncate font-semibold text-book-text-main">
          {step.title}
        </span>
        {recorded ? (
          <span className="inline-flex shrink-0 items-center gap-1 text-small text-success">
            <CheckCircle2 size={11} />
            {t("feedback.requestRecordedState")}
          </span>
        ) : (
          <span className={`inline-flex shrink-0 items-center gap-1 text-small ${stateClass(step.state)}`}>
            {stateIcon(step.state)}
            {stateLabel(step.state, t)}
          </span>
        )}
      </div>
      {step.note && (
        <div className={`mb-3 rounded-[var(--pyqt-radius-lg)] border px-3 py-2.5 text-xs leading-relaxed ${recorded ? "border-book-border/50 bg-book-bg-secondary/45 text-book-text-muted" : "border-warning/30 bg-warning/10 text-warning"}`}>
          {step.note}
        </div>
      )}
      <details className="mb-3 text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("feedback.requestInfo")}</summary>
        <div className="mt-2 flex min-w-0 flex-wrap gap-x-3 gap-y-1 border-l-2 border-book-border/60 pl-2 leading-relaxed">
          {step.prompt_name && <span>{t("feedback.template", { name: step.prompt_name })}</span>}
          <span>{charLabel(step.total_chars, locale, t)}</span>
          {maxTokens != null && <span>{maxTokensLabel(maxTokens, locale, t)}</span>}
          {promptLength && <span>{t("feedback.promptLength", { value: promptLength })}</span>}
          {requestLength && <span>{t("feedback.requestLength", { value: requestLength })}</span>}
          {contextAvailableInputTokens != null && (
            <span>
              {t("feedback.contextTokens", {
                estimated:
                  contextEstimatedInputTokens == null
                    ? "-"
                    : formatNumber(contextEstimatedInputTokens, locale),
                available: formatNumber(contextAvailableInputTokens, locale),
              })}
            </span>
          )}
          {contextStablePrefixTokens != null && (
            <span>{t("feedback.stablePrefix", { value: formatNumber(contextStablePrefixTokens, locale) })}</span>
          )}
          {contextDynamicDeltaTokens != null && (
            <span>{t("feedback.dynamicTail", { value: formatNumber(contextDynamicDeltaTokens, locale) })}</span>
          )}
          {contextPackCount != null && <span>{t("feedback.contextPacks", { count: formatNumber(contextPackCount, locale) })}</span>}
          {contextDroppedOptionalPacks != null && contextDroppedOptionalPacks > 0 && (
            <span className="text-warning">{t("feedback.droppedOptionalPacks", { count: formatNumber(contextDroppedOptionalPacks, locale) })}</span>
          )}
        </div>
      </details>
      {messages.length > 0 ? (
        <div className="grid min-w-0 gap-3 xl:grid-cols-2">
          {messages.map((message, index) => (
            <PromptBlock
              key={`${step.key}-${index}`}
              label={`${messageRoleLabel(message.role, t)} ${index + 1}`}
              content={textValue(message.content ?? message)}
              onCopy={onCopy}
              t={t}
            />
          ))}
        </div>
      ) : (
        <div className="grid min-w-0 gap-3 xl:grid-cols-2">
          <PromptBlock label={t("feedback.systemPrompt")} content={systemPrompt} onCopy={onCopy} t={t} />
          <PromptBlock label={t("feedback.userPrompt")} content={userPrompt} onCopy={onCopy} t={t} />
        </div>
      )}
      {requestJson.trim() && (
        <details open={recorded} className="mt-3 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("feedback.fullRequest")}</summary>
          <div className="mt-2">
            <PromptBlock
              label={t("feedback.fullRequest")}
              content={requestReadable}
              copyContent={requestJson}
              onCopy={onCopy}
              t={t}
            />
          </div>
        </details>
      )}
      {(contextPlanJson.trim() || contextPackRefsJson.trim() || contextManifestJson.trim()) && (
        <details className="mt-3 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("feedback.contextDetails")}</summary>
          <div className="mt-2 grid min-w-0 gap-3 xl:grid-cols-3">
            <PromptBlock
              label={t("feedback.budget")}
              content={contextPlanReadable}
              copyContent={contextPlanJson}
              onCopy={onCopy}
              t={t}
            />
            <PromptBlock
              label={t("feedback.references")}
              content={contextPackRefsReadable}
              copyContent={contextPackRefsJson}
              onCopy={onCopy}
              t={t}
            />
            <PromptBlock
              label={t("feedback.manifest")}
              content={contextManifestReadable}
              copyContent={contextManifestJson}
              onCopy={onCopy}
              t={t}
            />
          </div>
        </details>
      )}
    </section>
  );
};

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
  const [copied, setCopied] = useState(false);
  const copiedTimerRef = useRef<number | null>(null);
  const copySequenceRef = useRef(0);
  const { addToast } = useToast();
  const { locale, t } = useI18n();

  useLayoutEffect(() => {
    copySequenceRef.current += 1;
    setCopied(false);
    if (copiedTimerRef.current != null) {
      window.clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = null;
    }
  }, [open, data]);

  useEffect(
    () => () => {
      copySequenceRef.current += 1;
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
    },
    [],
  );

  const copyText = async (text: string) => {
    const copyRequest = ++copySequenceRef.current;
    try {
      await copyTextToClipboard(text);
      if (copySequenceRef.current !== copyRequest) return;
      setCopied(true);
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
      copiedTimerRef.current = window.setTimeout(() => {
        copiedTimerRef.current = null;
        setCopied(false);
      }, 1200);
    } catch (err) {
      if (copySequenceRef.current !== copyRequest) return;
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
        copiedTimerRef.current = null;
      }
      setCopied(false);
      addToast(extractErrorMessage(err, t("feedback.copyPreviewFailed")), "error");
    }
  };
  const allText = data
    ? data.steps
        .map((step) =>
          [
            `## ${step.title}${step.prompt_name ? ` (${step.prompt_name})` : ""}`,
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
  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title={data?.title ?? fallbackTitle ?? t("feedback.requestPreviewTitle")}
      subtitle={
        data
          ? t(recorded ? "feedback.requestRecorded" : "feedback.requestPreviewReady", {
              ready: formatNumber(data.ready_step_count, locale),
              total: formatNumber(data.steps.length, locale),
            })
          : t(recorded ? "feedback.requestRecordedLoading" : "feedback.requestPreviewSubtitle")
      }
      maxWidthClassName="max-w-[min(94vw,72rem)]"
      zIndexClassName="z-[80]"
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void copyText(allText)}
            disabled={!allText}
            className="!min-h-8 !text-xs"
          >
            <Clipboard size={12} />
            {copied ? t("feedback.copiedAll") : t("feedback.copyAll")}
          </BookButton>
          {confirmLabel && onConfirm ? (
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onConfirm()}
              disabled={confirmDisabled || confirmBusy}
              className="!min-h-8 !text-xs"
            >
              {confirmBusy ? <LoadingSpinner size="sm" /> : <CheckCircle2 size={12} />}
              {confirmLabel}
            </BookButton>
          ) : null}
        </div>
      }
    >
      {loading ? (
        <LoadingSpinner size="lg" text={t("feedback.buildingRequest")} inline={false} className="min-h-[18rem]" />
      ) : data ? (
        <div className="flex min-w-0 flex-col gap-3">
          {data.steps.map((step) => (
            <StepView key={step.key} step={step} recorded={recorded} onCopy={copyText} />
          ))}
        </div>
      ) : (
        <div className="py-10 text-center text-sm text-book-text-muted">{t("feedback.noPreviewData")}</div>
      )}
    </Modal>
  );
};
