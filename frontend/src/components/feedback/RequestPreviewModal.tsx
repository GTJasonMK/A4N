import React, { useEffect, useRef, useState } from "react";
import { CheckCircle2, Clipboard, Loader2, RotateCcw, XCircle } from "lucide-react";
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
  return state;
};

const stateClass = (state: string): string => {
  if (state === "ready") {
    return "border-success/40 bg-success/10 text-success";
  }
  if (state === "reused") {
    return "border-info/40 bg-info/10 text-info";
  }
  if (state === "preview_placeholder") {
    return "border-warning/40 bg-warning/10 text-warning";
  }
  return "border-warning/40 bg-warning/10 text-warning";
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
    <section className="min-w-0 rounded border border-book-border/50 bg-book-bg-paper/50">
      <div className="flex items-center justify-between gap-2 border-b border-book-border/40 px-3 py-2 text-xs">
        <span className="min-w-0 truncate font-semibold text-book-text-main">{label}</span>
        <button
          type="button"
          onClick={() => onCopy(copyText)}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          title={`复制 ${label}`}
        >
          <Clipboard size={12} />
        </button>
      </div>
      <pre className="max-h-[42vh] min-h-[7rem] overflow-auto whitespace-pre-wrap break-words px-3 py-2 font-mono text-[11px] leading-relaxed text-book-text-main custom-scrollbar">
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
    <section className="min-w-0 rounded border border-book-border/60 bg-book-bg-tertiary/20 p-3">
      <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 text-xs">
        <span className="min-w-0 flex-1 truncate font-semibold text-book-text-main">
          {step.title}
        </span>
        {step.prompt_name && <span className="pyqt-pill font-mono !text-[10px]">{step.prompt_name}</span>}
        <span className={`rounded border px-2 py-0.5 text-[10px] ${stateClass(step.state)}`}>
          <span className="inline-flex items-center gap-1">
            {stateIcon(step.state)}
            {stateLabel(step.state)}
          </span>
        </span>
        {maxTokens != null && (
          <span
            className="pyqt-pill font-mono !text-[10px]"
            title="本次请求发送给模型的最大输出 token 数"
          >
            {maxTokensLabel(maxTokens)}
          </span>
        )}
        <span className="font-mono text-book-text-muted" title="System + User 提示词字符数">
          输入 {charLabel(step.total_chars)}
        </span>
        {promptLength && (
          <span className="font-mono text-book-text-muted" title="provider request.prompt 的长度">
            Prompt {promptLength}
          </span>
        )}
        {requestLength && (
          <span className="font-mono text-book-text-muted" title="完整 provider request JSON 的长度">
            Request {requestLength}
          </span>
        )}
        {contextAvailableInputTokens != null && (
          <span
            className="pyqt-pill font-mono !text-[10px]"
            title="预览按默认模型上下文能力估算；真实调用以当前 LLM 配置为准"
          >
            上下文 {contextEstimatedInputTokens?.toLocaleString() ?? "-"} /{" "}
            {contextAvailableInputTokens.toLocaleString()} tokens
          </span>
        )}
        {contextStablePrefixTokens != null && (
          <span className="pyqt-pill font-mono !text-[10px]" title="稳定前缀有利于 DeepSeek prompt cache 命中">
            稳定前缀 {contextStablePrefixTokens.toLocaleString()}
          </span>
        )}
        {contextDynamicDeltaTokens != null && (
          <span className="pyqt-pill font-mono !text-[10px]" title="每次任务变化的动态尾部 token 估算">
            动态尾部 {contextDynamicDeltaTokens.toLocaleString()}
          </span>
        )}
        {contextPackCount != null && (
          <span className="pyqt-pill font-mono !text-[10px]" title="本次预览选中的上下文包数量">
            上下文包 {contextPackCount}
          </span>
        )}
        {contextDroppedOptionalPacks != null && contextDroppedOptionalPacks > 0 && (
          <span className="rounded border border-warning/40 bg-warning/10 px-2 py-0.5 font-mono text-[10px] text-warning">
            丢弃可选包 {contextDroppedOptionalPacks}
          </span>
        )}
      </div>
      {step.note && (
        <div className="mb-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs leading-relaxed text-warning">
          {step.note}
        </div>
      )}
      {messages.length > 0 ? (
        <div className="grid min-w-0 gap-3 xl:grid-cols-2">
          {messages.map((message, index) => (
            <PromptBlock
              key={`${step.key}-${index}`}
              label={`${message.role ?? "message"} #${index + 1}`}
              content={textValue(message.content ?? message)}
              onCopy={onCopy}
            />
          ))}
        </div>
      ) : (
        <div className="grid min-w-0 gap-3 xl:grid-cols-2">
          <PromptBlock label="System" content={systemPrompt} onCopy={onCopy} />
          <PromptBlock label="User / Prompt" content={userPrompt} onCopy={onCopy} />
        </div>
      )}
      {requestJson.trim() && (
        <div className="mt-3">
          <PromptBlock
            label="完整 Request（可读）"
            content={requestReadable}
            copyContent={requestJson}
            onCopy={onCopy}
          />
        </div>
      )}
      {(contextPlanJson.trim() || contextPackRefsJson.trim() || contextManifestJson.trim()) && (
        <div className="mt-3 grid min-w-0 gap-3 xl:grid-cols-3">
          <PromptBlock
            label="上下文预算 Plan"
            content={contextPlanReadable}
            copyContent={contextPlanJson}
            onCopy={onCopy}
          />
          <PromptBlock
            label="上下文包 Refs"
            content={contextPackRefsReadable}
            copyContent={contextPackRefsJson}
            onCopy={onCopy}
          />
          <PromptBlock
            label="上下文 Manifest"
            content={contextManifestReadable}
            copyContent={contextManifestJson}
            onCopy={onCopy}
          />
        </div>
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
  const outputMaxTokens = data
    ? Array.from(
        new Set(
          data.steps
            .map((step) => numericField(step.request, "max_tokens"))
            .filter((value): value is number => value != null),
        ),
      )
    : [];
  const outputMaxTokensSummary =
    outputMaxTokens.length === 1
      ? maxTokensLabel(outputMaxTokens[0])
      : outputMaxTokens.length > 1
        ? `输出上限 ${outputMaxTokens.map((value) => value.toLocaleString()).join(" / ")} tokens`
        : null;

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title={data?.title ?? fallbackTitle}
      subtitle={
        data
          ? `${data.ready_step_count}/${data.steps.length} 步可直接显示 · 输入 ${charLabel(data.total_chars)}${
              outputMaxTokensSummary ? ` · ${outputMaxTokensSummary}` : ""
            }`
          : "直接构造将发送给模型的请求，不发送"
      }
      maxWidthClassName="max-w-[min(96vw,88rem)]"
      bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      footer={
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs text-book-text-muted">{copied ? "已复制" : ""}</span>
          <div className="flex items-center gap-2">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void copyText(allText)}
              disabled={!allText}
            >
              复制全部
            </BookButton>
            <BookButton variant="secondary" size="sm" onClick={onClose}>
              关闭
            </BookButton>
          </div>
        </div>
      }
    >
      {loading ? (
        <div className="flex min-h-[18rem] flex-col items-center justify-center gap-3 text-book-text-muted">
          <Loader2 size={28} className="animate-spin text-book-primary" />
          <span className="text-sm">正在构造模型请求…</span>
        </div>
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
