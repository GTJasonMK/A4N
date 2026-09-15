import { ChevronRight, CircleAlert, Wrench } from "lucide-react";
import type { AgentMessageView } from "../../api/agent";
import { usePacedStreamText } from "../../hooks/usePacedStreamText";
import type { Locale } from "../../i18n";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { MarkdownText } from "../ui/MarkdownText";

export interface AgentMessageRowProps {
  message: AgentMessageView;
  locale: Locale;
  toolResults?: AgentMessageView[];
  wasStreamed?: boolean;
  compactAssistantMessages?: boolean;
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

function isPendingMessage(metadata: Record<string, unknown> | null): boolean {
  return metadata?.pending === true;
}

export function AgentMessageRow({
  message,
  locale,
  toolResults = [],
  wasStreamed = false,
  compactAssistantMessages = false,
}: AgentMessageRowProps) {
  const metadata = objectValue(message.metadata);
  const toolCalls = toolCallsFrom(metadata);
  const hasContent = Boolean(message.content.trim());
  const isUser = message.role === "user";
  const isTool = message.role === "tool";
  const hasToolCalls = toolCalls.length > 0;
  const pending = isPendingMessage(metadata);
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
  const processState = relatedToolResults.some((result) => toolError(objectValue(result.metadata), locale))
    ? "error"
    : message.status !== "complete" || callResults.some(({ results }) => results.length === 0)
      ? "running"
      : "complete";
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
      data-agent-message-pending={pending || undefined}
      data-agent-message-kind={
        hasToolCalls
          ? relatedToolResults.length > 0
            ? "tool-operation"
            : "tool-call"
          : isTool
            ? "tool-result"
            : message.role
      }
      className={`agent-message-row agent-message-row--${message.role}`}
    >
      <div className="agent-message-stack">
        {isUser ? (
          <div className="agent-message-bubble">
            <p className="agent-message-user-text">{message.content || emptyLabel}</p>
          </div>
        ) : null}
        {hasToolCalls ? (
          <div className="agent-message-process-group" data-agent-process-state={processState}>
            <div className="agent-message-process-heading">
              <Wrench size={13} aria-hidden="true" />
              <span>{locale === "zh-CN" ? "执行轨迹" : "Execution trace"}</span>
              <span className="agent-message-process-count">{toolCalls.length}</span>
              {message.status !== "complete" ? (
                <span className="agent-message-process-state">{statusLabel(message.status, locale)}</span>
              ) : null}
            </div>
            {callResults.map(({ call, results }, index) => {
              const args = toolArguments(call);
              const id = toolCallId(call);
              return (
                <details key={`${message.id}-tool-${index}`} className="agent-message-process">
                  <summary className="agent-message-process-summary">
                    <Wrench size={11} className="shrink-0 text-book-primary" aria-hidden="true" />
                    <span className="agent-message-process-name">{toolName(call, locale)}</span>
                    <span className="agent-message-process-result">{toolCallResultLabel(results, locale)}</span>
                    <ChevronRight size={13} className="agent-message-process-chevron" aria-hidden="true" />
                  </summary>
                  <div className="agent-message-process-body">
                    {id ? (
                      <p className="agent-message-trace">
                        {locale === "zh-CN" ? "调用" : "Call"} <code>{id}</code>
                      </p>
                    ) : null}
                    {args !== undefined ? (
                      <details className="agent-message-arguments">
                        <summary>
                          {locale === "zh-CN" ? "查看参数" : "View arguments"}
                        </summary>
                        <pre className="agent-message-json custom-scrollbar">
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
              <div className="agent-message-unpaired-results">
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
          <ToolResultBlock message={message} locale={locale} nested />
        ) : message.role === "assistant" &&
          (hasContent || message.status === "streaming") ? (
          <AssistantMessageContent
            message={message}
            locale={locale}
            wasStreamed={wasStreamed}
            compact={compactAssistantMessages}
          />
        ) : !isUser && hasContent ? (
          <MarkdownText
            className="agent-message-markdown"
            text={message.content}
          />
        ) : !isUser && !hasToolCalls ? (
          <p className="agent-message-empty">{emptyLabel}</p>
        ) : null}
        {pending ? (
          <span className="agent-message-pending-label" role="status">
            {locale === "zh-CN" ? "已提交，等待 Agent 接管" : "Submitted, waiting for Agent"}
          </span>
        ) : null}
        {message.status === "interrupted" ? (
          <span className="agent-message-interrupted">
            {locale === "zh-CN" ? "已中断" : "Interrupted"}
          </span>
        ) : null}
      </div>
    </article>
  );
}

function AssistantMessageContent({
  message,
  locale,
  wasStreamed,
  compact,
}: {
  message: AgentMessageView;
  locale: Locale;
  wasStreamed: boolean;
  compact: boolean;
}) {
  const canCollapse = compact && message.status !== "streaming" && isLongAssistantMessage(message.content);
  if (canCollapse) {
    return (
      <details className="agent-message-collapsible">
        <summary className="agent-message-collapsible__summary">
          <span className="agent-message-collapsible__label">
            {assistantMessageSummary(message.content, locale)}
          </span>
          <span className="agent-message-collapsible__action">
            {locale === "zh-CN" ? "展开" : "Expand"}
          </span>
          <ChevronRight className="agent-message-collapsible__chevron" size={13} aria-hidden="true" />
        </summary>
        <div className="agent-message-collapsible__body">
          <AssistantMessageBody message={message} locale={locale} wasStreamed={wasStreamed} />
        </div>
      </details>
    );
  }
  return <AssistantMessageBody message={message} locale={locale} wasStreamed={wasStreamed} />;
}

function AssistantMessageBody({
  message,
  locale,
  wasStreamed,
}: {
  message: AgentMessageView;
  locale: Locale;
  wasStreamed: boolean;
}) {
  const pacedStream = usePacedStreamText(
    message.content,
    message.status === "streaming",
    true,
    wasStreamed,
  );
  const streamDisplaying = pacedStream.isStreamDisplayEnabled
    && (message.status === "streaming" || pacedStream.hasPendingText);
  const showStreamStatus = streamDisplaying;

  if (!showStreamStatus) {
    return (
      <MarkdownText
        className="agent-message-markdown"
        text={pacedStream.displayedText}
      />
    );
  }

  return (
    <div className="min-w-0 break-words text-xs leading-relaxed text-book-text-main" role="status">
      {pacedStream.displayedText ? (
        <div className="whitespace-pre-wrap">{pacedStream.displayedText}</div>
      ) : null}
      {pacedStream.displayedText ? null : (
        <LoadingSpinner
          size="sm"
          text={locale === "zh-CN" ? "正在思考" : "Thinking"}
          className="agent-message-thinking"
        />
      )}
      {pacedStream.displayedText ? (
        <span
          aria-hidden="true"
          className="agent-message-caret"
        />
      ) : null}
    </div>
  );
}

function isLongAssistantMessage(content: string): boolean {
  const nonEmptyLines = content.split(/\r?\n/).filter((line) => line.trim()).length;
  return content.length > 420 || nonEmptyLines > 6;
}

function assistantMessageSummary(content: string, locale: Locale): string {
  const lines = content.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const heading = lines.find((line) => /^#{1,6}\s+/.test(line))?.replace(/^#{1,6}\s+/, "");
  const numberedItems = lines.filter((line) => /^\d+[.)]\s+/.test(line)).length;
  const subject = heading || lines[0] || (locale === "zh-CN" ? "Agent 回复" : "Agent reply");
  return numberedItems > 0
    ? `${subject} · ${numberedItems} ${locale === "zh-CN" ? "项" : numberedItems === 1 ? "item" : "items"}`
    : subject;
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
    <div className={nested ? "agent-message-tool-result agent-message-tool-result--nested" : "agent-message-tool-result"}>
      {nested ? (
        <div className="agent-message-tool-result-heading">
          {error ? <CircleAlert size={11} className="shrink-0 text-error" aria-hidden="true" /> : null}
          <span>
            {locale === "zh-CN" ? "执行结果" : "Execution result"}
          </span>
        </div>
      ) : null}
      <div className="agent-message-tool-result-line">
        <p className={error ? "agent-message-tool-error" : "agent-message-tool-summary"}>
          {error ?? resultSummary(metadata, message.content, locale)}
        </p>
        {traceText ? (
          <span
            className="agent-message-trace agent-message-trace--inline"
            title={traceText}
          >
            {traceText}
          </span>
        ) : null}
      </div>
      {raw ? (
        <details className="agent-message-arguments">
          <summary>
            {locale === "zh-CN" ? "查看原始结果" : "View raw result"}
          </summary>
          <pre className="agent-message-json agent-message-json--result custom-scrollbar">
            {raw}
          </pre>
        </details>
      ) : null}
    </div>
  );
}
