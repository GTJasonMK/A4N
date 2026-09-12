import { Bot, ChevronRight, CircleAlert, Wrench } from "lucide-react";
import type { AgentMessageView } from "../../api/agent";
import { MarkdownText } from "../ui/MarkdownText";
import { formatDateTime } from "../../i18n/format";
import type { Locale } from "../../i18n";

export interface AgentMessageRowProps {
  message: AgentMessageView;
  locale: Locale;
  agentLabel?: string;
  toolResults?: AgentMessageView[];
}

export function compactAgentValue(value: unknown, maxLength = 220): string {
  if (typeof value === "string") return value.slice(0, maxLength);
  if (value == null) return "";
  try {
    const text = JSON.stringify(value);
    return text ? text.slice(0, maxLength) : "";
  } catch {
    return "";
  }
}

function objectValue(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function stringValue(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function formatJson(value: unknown): string {
  if (typeof value === "string") {
    try {
      const parsed = JSON.parse(value) as unknown;
      return JSON.stringify(parsed, null, 2) ?? value;
    } catch {
      return value;
    }
  }
  try {
    return JSON.stringify(value, null, 2) ?? "";
  } catch {
    return compactAgentValue(value);
  }
}

function toolCallsFrom(metadata: Record<string, unknown> | null): unknown[] {
  return Array.isArray(metadata?.tool_calls) ? metadata.tool_calls : [];
}

export function agentToolCallIds(message: AgentMessageView): string[] {
  const metadata = objectValue(message.metadata);
  return toolCallsFrom(metadata)
    .map(toolCallId)
    .filter((id): id is string => id != null);
}

export function agentToolResultCallId(message: AgentMessageView): string | null {
  return stringValue(objectValue(message.metadata)?.tool_call_id);
}

function toolName(call: unknown, locale: Locale): string {
  const record = objectValue(call);
  const functionValue = objectValue(record?.function);
  return (
    stringValue(record?.name) ??
    stringValue(functionValue?.name) ??
    (locale === "zh-CN" ? "未命名工具" : "Unnamed tool")
  );
}

function toolCallId(call: unknown): string | null {
  const record = objectValue(call);
  return stringValue(record?.id);
}

function toolArguments(call: unknown): unknown {
  const record = objectValue(call);
  const functionValue = objectValue(record?.function);
  return record && "arguments" in record
    ? record.arguments
    : functionValue && "arguments" in functionValue
      ? functionValue.arguments
      : undefined;
}

function messageLabel(
  message: AgentMessageView,
  locale: Locale,
  agentLabel: string,
): string {
  if (message.role === "user") return locale === "zh-CN" ? "你" : "You";
  if (message.role === "tool") return locale === "zh-CN" ? "工具结果" : "Tool result";
  if (message.role === "system") return locale === "zh-CN" ? "系统" : "System";
  return agentLabel;
}

function statusLabel(status: AgentMessageView["status"], locale: Locale): string {
  if (status === "streaming") return locale === "zh-CN" ? "流式中" : "Streaming";
  if (status === "interrupted") return locale === "zh-CN" ? "已中断" : "Interrupted";
  return locale === "zh-CN" ? "已完成" : "Complete";
}

function resultSummary(
  metadata: Record<string, unknown> | null,
  content: string,
  locale: Locale,
): string {
  const result = metadata?.result;
  const resultRecord = objectValue(result);
  const directSummary = ["summary", "message", "text"]
    .map((key) => stringValue(resultRecord?.[key]))
    .find((value): value is string => value != null);
  if (directSummary) return compactAgentValue(directSummary);
  if (Array.isArray(result)) {
    return locale === "zh-CN" ? `返回 ${result.length} 项` : `${result.length} item(s) returned`;
  }
  const collection = resultRecord && Object.entries(resultRecord).find(([, value]) => Array.isArray(value));
  if (collection) {
    const [key, rawValue] = collection;
    const value = rawValue as unknown[];
    return locale === "zh-CN"
      ? `返回 ${key} ${value.length} 项`
      : `${value.length} ${key} item(s) returned`;
  }
  if (resultRecord) {
    const keys = Object.keys(resultRecord);
    if (keys.length === 0) return locale === "zh-CN" ? "返回空对象" : "Empty object returned";
    const preview = keys.slice(0, 4).join(locale === "zh-CN" ? "、" : ", ");
    const suffix = keys.length > 4 ? "…" : "";
    return locale === "zh-CN"
      ? `返回 ${keys.length} 个字段：${preview}${suffix}`
      : `${keys.length} field(s) returned: ${preview}${suffix}`;
  }
  if (result != null) {
    const compact = compactAgentValue(result);
    if (compact) return compact;
  }
  const compactContent = compactAgentValue(content.trim());
  return compactContent || (locale === "zh-CN" ? "工具结果没有可读摘要" : "Tool result has no readable summary");
}

function toolCallResultLabel(
  results: AgentMessageView[],
  locale: Locale,
): string {
  if (results.length === 0) {
    return locale === "zh-CN" ? "等待结果" : "Waiting for result";
  }
  const error = results
    .map((result) => toolError(objectValue(result.metadata), locale))
    .find((value): value is string => value != null);
  if (error) return error;
  if (results.length === 1) {
    const result = results[0];
    return resultSummary(objectValue(result.metadata), result.content, locale);
  }
  return locale === "zh-CN"
    ? `返回 ${results.length} 个结果`
    : `${results.length} results returned`;
}

function toolError(
  metadata: Record<string, unknown> | null,
  locale: Locale,
): string | null {
  const result = objectValue(metadata?.result);
  return (
    stringValue(metadata?.error) ??
    stringValue(result?.error) ??
    (metadata?.status === "failed"
      ? locale === "zh-CN"
        ? "工具执行失败"
        : "Tool execution failed"
      : null)
  );
}

function rawResult(content: string, metadata: Record<string, unknown> | null): string {
  if (content.trim()) return formatJson(content);
  if (metadata && "result" in metadata) return formatJson(metadata.result);
  return "";
}

function resultTrace(
  metadata: Record<string, unknown> | null,
  locale: Locale,
): string | null {
  const toolCallId = stringValue(metadata?.tool_call_id);
  const executionId = stringValue(metadata?.execution_id);
  const parts = [
    toolCallId ? `${locale === "zh-CN" ? "调用" : "call"} ${toolCallId}` : null,
    executionId ? `${locale === "zh-CN" ? "执行" : "execution"} #${executionId}` : null,
  ].filter((part): part is string => part != null);
  return parts.length > 0 ? parts.join(" · ") : null;
}

export function AgentMessageRow({
  message,
  locale,
  agentLabel = "Agent",
  toolResults = [],
}: AgentMessageRowProps) {
  const metadata = objectValue(message.metadata);
  const toolCalls = toolCallsFrom(metadata);
  const hasContent = Boolean(message.content.trim());
  const isUser = message.role === "user";
  const isTool = message.role === "tool";
  const hasToolCalls = toolCalls.length > 0;
  const relatedToolResults = message.role === "assistant" ? toolResults : [];
  const callResults = toolCalls.map((call) => ({
    call,
    results: toolCallId(call)
      ? relatedToolResults.filter(
          (result) => agentToolResultCallId(result) === toolCallId(call),
        )
      : [],
  }));
  const pairedResultIds = new Set(
    callResults.flatMap(({ results }) => results.map((result) => result.id)),
  );
  const unpairedToolResults = relatedToolResults.filter(
    (result) => !pairedResultIds.has(result.id),
  );
  const error = isTool ? toolError(metadata, locale) : null;
  const label = hasToolCalls
    ? locale === "zh-CN"
      ? "工具调用"
      : "Tool calls"
    : messageLabel(message, locale, agentLabel);
  const emptyLabel =
    message.role === "system"
      ? locale === "zh-CN"
        ? "系统未返回文本"
        : "System returned no text"
      : locale === "zh-CN"
        ? "Agent 未返回文本"
        : "Agent returned no text";

  return (
    <article
      data-agent-message-role={message.role}
      data-agent-message-kind={
        hasToolCalls
          ? relatedToolResults.length > 0
            ? "tool-operation"
            : "tool-call"
          : isTool
            ? "tool-result"
            : message.role
      }
      className={`flex min-w-0 ${isUser ? "justify-end" : "justify-start"}`}
    >
      <div
        className={`min-w-0 max-w-[min(94%,48rem)] rounded-2xl border px-2.5 py-2 ${
          isUser
            ? "border-book-primary/25 bg-book-primary/8"
            : error
              ? "border-error/35 bg-error-bg/45"
              : hasToolCalls
                ? "border-book-primary/25 bg-book-primary/5"
                : isTool
                  ? "border-book-border/45 bg-book-bg-tertiary/40"
                  : "border-book-border/45 bg-book-bg-paper"
        }`}
      >
        {!hasToolCalls ? (
          <div className="mb-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-[10px] text-book-text-muted">
            {isTool && error ? <CircleAlert size={12} className="shrink-0 text-error" aria-hidden="true" /> : null}
            {!isUser && !isTool ? <Bot size={12} className="shrink-0 text-book-primary/70" aria-hidden="true" /> : null}
            <span className="font-medium text-book-text-sub">{label}</span>
            <time dateTime={message.created_at}>{formatDateTime(message.created_at, locale)}</time>
            {message.status !== "complete" ? (
              <span className="rounded-full bg-warning-bg px-1.5 py-0.5 text-warning">
                {statusLabel(message.status, locale)}
              </span>
            ) : null}
          </div>
        ) : null}

        {hasToolCalls ? (
          <div className="space-y-1.5">
            {callResults.map(({ call, results }, index) => {
              const args = toolArguments(call);
              const id = toolCallId(call);
              return (
                <details key={`${message.id}-tool-${index}`} className="group min-w-0 rounded-lg bg-book-bg/55">
                  <summary className="flex min-w-0 cursor-pointer list-none items-center gap-1.5 px-2 py-1.5 text-[10px] [&::-webkit-details-marker]:hidden">
                    <Wrench size={11} className="shrink-0 text-book-primary" aria-hidden="true" />
                    <span className="shrink-0 font-medium text-book-text-sub">{label}</span>
                    <span aria-hidden="true" className="shrink-0 text-book-text-muted">·</span>
                    <span className="min-w-0 flex-1 truncate font-medium text-book-text-sub">{toolName(call, locale)}</span>
                    <span className="min-w-0 max-w-[48%] shrink-0 truncate text-right text-[10px] text-book-text-muted">
                      · {toolCallResultLabel(results, locale)}
                    </span>
                    {index === 0 ? <time className="shrink-0 text-[9px] text-book-text-muted" dateTime={message.created_at}>{formatDateTime(message.created_at, locale)}</time> : null}
                    {message.status !== "complete" && index === 0 ? (
                      <span className="shrink-0 rounded-full bg-warning-bg px-1.5 py-0.5 text-warning">
                        {statusLabel(message.status, locale)}
                      </span>
                    ) : null}
                    <ChevronRight size={12} className="shrink-0 text-book-text-muted transition-transform group-open:rotate-90" aria-hidden="true" />
                  </summary>
                  <div className="min-w-0 px-2 pb-1.5">
                    {id ? (
                      <p className="break-all text-[10px] text-book-text-muted">
                        {locale === "zh-CN" ? "调用" : "Call"} <code>{id}</code>
                      </p>
                    ) : null}
                    {args !== undefined ? (
                      <details className="mt-1 text-[10px] text-book-text-muted">
                        <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main [&::-webkit-details-marker]:hidden">
                          {locale === "zh-CN" ? "查看参数" : "View arguments"}
                        </summary>
                        <pre className="mt-1.5 max-h-32 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-book-bg-tertiary/60 p-2 font-mono leading-relaxed text-book-text-main custom-scrollbar">
                          {formatJson(args)}
                        </pre>
                      </details>
                    ) : null}
                    {results.map((result) => (
                      <ToolResultBlock
                        key={`${message.id}-${result.id}`}
                        message={result}
                        locale={locale}
                        nested
                      />
                    ))}
                  </div>
                </details>
              );
            })}
            {unpairedToolResults.length > 0 ? (
              <div className="min-w-0 rounded-lg border border-book-border/35 bg-book-bg-paper/45 px-2 py-1.5">
                {unpairedToolResults.map((result) => (
                  <ToolResultBlock
                    key={`${message.id}-${result.id}`}
                    message={result}
                    locale={locale}
                    nested
                  />
                ))}
              </div>
            ) : null}
          </div>
        ) : null}

        {isTool ? (
          <ToolResultBlock message={message} locale={locale} />
        ) : hasContent ? (
          <MarkdownText
            className="break-words text-xs leading-relaxed text-book-text-main"
            text={message.content}
          />
        ) : hasToolCalls ? null : (
          <p className="text-xs italic leading-relaxed text-book-text-muted">{emptyLabel}</p>
        )}
      </div>
    </article>
  );
}

function ToolResultBlock({
  message,
  locale,
  nested = false,
}: {
  message: AgentMessageView;
  locale: Locale;
  nested?: boolean;
}) {
  const metadata = objectValue(message.metadata);
  const error = toolError(metadata, locale);
  const raw = rawResult(message.content, metadata);
  const resultToolName = stringValue(metadata?.tool);
  const trace = resultTrace(metadata, locale);
  const traceText = [resultToolName, trace]
    .filter((part): part is string => part != null)
    .join(" · ");

  return (
    <div className={nested ? "mt-1.5 min-w-0 border-t border-book-border/35 pt-1.5" : "mt-1.5 min-w-0"}>
      {nested ? (
        <div className="mb-0.5 flex min-w-0 items-center gap-1.5 text-[10px] text-book-text-muted">
          {error ? <CircleAlert size={11} className="shrink-0 text-error" aria-hidden="true" /> : null}
          <span className="font-medium text-book-text-sub">
            {locale === "zh-CN" ? "工具结果" : "Tool result"}
          </span>
        </div>
      ) : null}
      <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5">
        <p className={`min-w-0 flex-1 break-words text-xs leading-relaxed ${error ? "text-error" : "text-book-text-main"}`}>
          {error ?? resultSummary(metadata, message.content, locale)}
        </p>
        {traceText ? (
          <span
            className="min-w-0 max-w-[48%] shrink-0 truncate text-right text-[10px] text-book-text-muted"
            title={traceText}
          >
            {traceText}
          </span>
        ) : null}
      </div>
      {raw ? (
        <details className="mt-1.5 text-[10px] text-book-text-muted">
          <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main [&::-webkit-details-marker]:hidden">
            {locale === "zh-CN" ? "查看原始结果" : "View raw result"}
          </summary>
          <pre className="mt-1.5 max-h-40 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-book-bg/65 p-2 font-mono leading-relaxed text-book-text-main custom-scrollbar">
            {raw}
          </pre>
        </details>
      ) : null}
    </div>
  );
}
