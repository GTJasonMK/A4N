import React, { useEffect, useRef, useState } from "react";
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

interface RequestPreviewModalProps {
  open: boolean;
  loading: boolean;
  data: ModelRequestPreview | null;
  fallbackTitle?: string;
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

const charLabel = (value: number): string => `${value.toLocaleString()} 字符`;

const stateLabel = (state: string): string => {
  if (state === "ready") return "可预览";
  if (state === "preview_placeholder") return "结构预览";
  if (state === "reused") return "复用";
  if (state === "blocked") return "等待前序输出";
  return "暂不可用";
};

const messageRoleLabel = (role: unknown): string => {
  if (role === "system") return "系统消息";
  if (role === "user") return "用户消息";
  if (role === "assistant") return "模型消息";
  if (role === "tool") return "工具消息";
  return "消息";
};

const stateClass = (state: string): string => {
  if (state === "ready") {
    return "text-success";
  }
  if (state === "reused") {
    return "text-info";
  }
  if (state === "preview_placeholder") {
    return "text-warning";
  }
  return "text-warning";
};

const stateIcon = (state: string) => {
  if (state === "ready") return <CheckCircle2 size={11} />;
  if (state === "reused") return <RotateCcw size={11} />;
  if (state === "preview_placeholder") return <CheckCircle2 size={11} />;
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

const maxTokensLabel = (value: number | null): string | null =>
  value == null ? null : `输出上限 ${value.toLocaleString()} tokens`;

const lengthLabel = (chars: number | null, bytes: number | null): string | null => {
  if (chars == null && bytes == null) return null;
  if (chars != null && bytes != null) {
    return `${chars.toLocaleString()} 字符 / ${bytes.toLocaleString()} 字节`;
  }
  if (chars != null) return `${chars.toLocaleString()} 字符`;
  return `${bytes?.toLocaleString()} 字节`;
};

const PromptBlock: React.FC<{
  label: string;
  content: string;
  copyContent?: string;
  onCopy: (text: string) => void;
}> = ({ label, content, copyContent, onCopy }) => {
  if (!content.trim()) return null;
  const copyText = copyContent ?? content;
  return (
    <section className="min-w-0 overflow-hidden rounded-[var(--pyqt-radius-lg)] border border-book-border/50 bg-book-bg-secondary/45">
      <div className="flex min-w-0 items-center justify-between gap-3 border-b border-book-border/45 px-3 py-2.5">
        <span className="min-w-0 truncate text-xs font-semibold text-book-text-main">{label}</span>
        <button
          type="button"
          onClick={() => onCopy(copyText)}
          className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] text-book-text-muted transition-colors hover:bg-book-bg-tertiary hover:text-book-primary"
          title={`复制 ${label}`}
          aria-label={`复制 ${label}`}
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
  onCopy: (text: string) => void;
}> = ({ step, onCopy }) => {
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
  );
  const requestLength = lengthLabel(
    numericRecordField(step.char_counts, "request_chars"),
    numericRecordField(step.char_counts, "request_bytes"),
  );
  return (
    <section className="min-w-0">
      <div className="mb-3 flex min-w-0 items-center gap-2 border-b border-book-border/45 pb-2 text-xs">
        <span className="min-w-0 flex-1 truncate font-semibold text-book-text-main">
          {step.title}
        </span>
        <span className={`inline-flex shrink-0 items-center gap-1 text-small ${stateClass(step.state)}`}>
          {stateIcon(step.state)}
          {stateLabel(step.state)}
        </span>
      </div>
      {step.note && (
        <div className="mb-3 rounded-[var(--pyqt-radius-lg)] border border-warning/30 bg-warning/10 px-3 py-2.5 text-xs leading-relaxed text-warning">
          {step.note}
        </div>
      )}
      <details className="mb-3 text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer hover:text-book-text-main">请求信息</summary>
        <div className="mt-2 flex min-w-0 flex-wrap gap-x-3 gap-y-1 border-l-2 border-book-border/60 pl-2 leading-relaxed">
          {step.prompt_name && <span>模板 {step.prompt_name}</span>}
          <span>输入 {charLabel(step.total_chars)}</span>
          {maxTokens != null && <span>{maxTokensLabel(maxTokens)}</span>}
          {promptLength && <span>Prompt {promptLength}</span>}
          {requestLength && <span>Request {requestLength}</span>}
          {contextAvailableInputTokens != null && (
            <span>
              上下文 {contextEstimatedInputTokens?.toLocaleString() ?? "-"} /{" "}
              {contextAvailableInputTokens.toLocaleString()} tokens
            </span>
          )}
          {contextStablePrefixTokens != null && (
            <span>稳定前缀 {contextStablePrefixTokens.toLocaleString()}</span>
          )}
          {contextDynamicDeltaTokens != null && (
            <span>动态尾部 {contextDynamicDeltaTokens.toLocaleString()}</span>
          )}
          {contextPackCount != null && <span>上下文包 {contextPackCount}</span>}
          {contextDroppedOptionalPacks != null && contextDroppedOptionalPacks > 0 && (
            <span className="text-warning">丢弃可选包 {contextDroppedOptionalPacks}</span>
          )}
        </div>
      </details>
      {messages.length > 0 ? (
        <div className="grid min-w-0 gap-3 xl:grid-cols-2">
          {messages.map((message, index) => (
            <PromptBlock
              key={`${step.key}-${index}`}
              label={`${messageRoleLabel(message.role)} ${index + 1}`}
              content={textValue(message.content ?? message)}
              onCopy={onCopy}
            />
          ))}
        </div>
      ) : (
        <div className="grid min-w-0 gap-3 xl:grid-cols-2">
          <PromptBlock label="系统提示词" content={systemPrompt} onCopy={onCopy} />
          <PromptBlock label="用户提示词" content={userPrompt} onCopy={onCopy} />
        </div>
      )}
      {requestJson.trim() && (
        <details className="mt-3 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer hover:text-book-text-main">完整请求</summary>
          <div className="mt-2">
            <PromptBlock
              label="Request"
              content={requestReadable}
              copyContent={requestJson}
              onCopy={onCopy}
            />
          </div>
        </details>
      )}
      {(contextPlanJson.trim() || contextPackRefsJson.trim() || contextManifestJson.trim()) && (
        <details className="mt-3 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer hover:text-book-text-main">上下文详情</summary>
          <div className="mt-2 grid min-w-0 gap-3 xl:grid-cols-3">
            <PromptBlock
              label="预算"
              content={contextPlanReadable}
              copyContent={contextPlanJson}
              onCopy={onCopy}
            />
            <PromptBlock
              label="引用"
              content={contextPackRefsReadable}
              copyContent={contextPackRefsJson}
              onCopy={onCopy}
            />
            <PromptBlock
              label="清单"
              content={contextManifestReadable}
              copyContent={contextManifestJson}
              onCopy={onCopy}
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
  fallbackTitle = "模型请求预览",
  onClose,
}) => {
  const [copied, setCopied] = useState(false);
  const copiedTimerRef = useRef<number | null>(null);
  const { addToast } = useToast();

  useEffect(() => {
    setCopied(false);
    if (copiedTimerRef.current != null) {
      window.clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = null;
    }
  }, [open, data]);

  useEffect(
    () => () => {
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
    },
    [],
  );

  const copyText = async (text: string) => {
    try {
      await copyTextToClipboard(text);
      setCopied(true);
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
      copiedTimerRef.current = window.setTimeout(() => {
        copiedTimerRef.current = null;
        setCopied(false);
      }, 1200);
    } catch (err) {
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
        copiedTimerRef.current = null;
      }
      setCopied(false);
      addToast(extractErrorMessage(err, "复制请求预览失败"), "error");
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
      title={data?.title ?? fallbackTitle}
      subtitle={
        data
          ? `${data.ready_step_count}/${data.steps.length} 步可预览 · 不会发送请求`
          : "直接构造将发送给模型的请求，不发送"
      }
      maxWidthClassName="max-w-[min(94vw,72rem)]"
      zIndexClassName="z-[80]"
      footer={
        <BookButton
          variant="secondary"
          size="sm"
          onClick={() => void copyText(allText)}
          disabled={!allText}
          className="!min-h-8 !text-xs"
        >
          <Clipboard size={12} />
          {copied ? "已复制" : "复制全部"}
        </BookButton>
      }
    >
      {loading ? (
        <LoadingSpinner size="lg" text="正在构造模型请求…" inline={false} className="min-h-[18rem]" />
      ) : data ? (
        <div className="flex min-w-0 flex-col gap-3">
          {data.steps.map((step) => (
            <StepView key={step.key} step={step} onCopy={copyText} />
          ))}
        </div>
      ) : (
        <div className="py-10 text-center text-sm text-book-text-muted">暂无预览数据</div>
      )}
    </Modal>
  );
};
