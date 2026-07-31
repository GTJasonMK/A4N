import { AlertCircle, Bot, CheckCircle2, User, Wrench } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ReactNode } from "react";

import {
  formatItemMeta,
  itemClassName,
  kindLabel,
  processStepBehavior,
  type StreamItem,
  type StreamItemKind,
} from "./optimizeTabUtils";
import type { ProgressDialogStatus } from "../../../components/feedback/ProgressDialog";

export function StreamMessage({ item }: { item: StreamItem }) {
  const meta = formatItemMeta(item);
  const currentStep = currentProcessStep(item.steps);
  const currentBehavior = currentStep ? processStepBehavior(currentStep) : null;
  const isProcessItem = isProcessBubble(item) && currentBehavior != null;
  const title = isProcessItem
    ? currentBehavior.title
    : item.title ?? kindLabel(item.kind);
  const content = isProcessItem ? currentBehavior.detail : item.content;
  const contentClassName = "whitespace-pre-wrap break-words";
  const processSteps = visibleProcessSteps(item.steps, isProcessItem, currentStep);
  const showTimeline = item.kind !== "user" && (processSteps.length > 0 || isLiveModelItem(item));
  const showHeader = !showTimeline;
  return (
    <article
      className={`grid min-w-0 items-start gap-2 ${
        item.kind === "user"
          ? "grid-cols-[minmax(0,1fr)_auto]"
          : "grid-cols-[auto_minmax(0,1fr)]"
      }`}
    >
      {item.kind !== "user" ? <StreamIcon kind={item.kind} status={item.status} /> : null}
      <div
        className={`min-w-0 max-w-[96%] overflow-hidden rounded-2xl border px-3 py-2.5 text-xs leading-relaxed ${
          item.kind === "user" ? "max-w-[90%] justify-self-end rounded-br-md" : "rounded-bl-md"
        } ${
          itemClassName(item.kind)
        }`}
      >
        {showHeader ? (
          <div className="mb-1 flex min-w-0 flex-wrap items-center gap-2">
            <span className="min-w-0 break-words font-semibold">{title}</span>
            {meta ? (
              <span className="min-w-0 break-all font-mono text-small opacity-70">
                {meta}
              </span>
            ) : null}
          </div>
        ) : null}
        {showTimeline ? (
          <AgentTurnTimeline
            steps={processSteps}
            currentItem={isLiveModelItem(item) ? item : null}
            finalReply={isAssistantReplyItem(item) ? content : null}
          />
        ) : (
          <MarkdownText className={contentClassName} text={content} />
        )}
        {item.detail && !item.steps?.length ? (
          <details className="mt-2 min-w-0 overflow-hidden rounded-xl border border-book-border/70 bg-book-bg-paper/60">
            <summary className="cursor-pointer select-none px-2 py-1 text-small font-semibold text-book-text-sub hover:text-book-text-main">
              {item.detailLabel ?? "查看详情"}
            </summary>
            <pre className="whitespace-pre-wrap break-words border-t border-book-border/70 px-2 py-2 font-mono text-small leading-relaxed text-book-text-sub">
              {item.detail}
            </pre>
          </details>
        ) : null}
      </div>
      {item.kind === "user" ? <StreamIcon kind={item.kind} status={item.status} /> : null}
    </article>
  );
}

function AgentTurnTimeline({
  steps,
  currentItem,
  finalReply,
}: {
  steps: StreamItem[];
  currentItem: StreamItem | null;
  finalReply: string | null;
}) {
  const rows: TimelineRow[] = [
    ...timelineRowsForSteps(steps),
    ...(currentItem ? timelineRowsForItem(currentItem, `${currentItem.id}-current`) : []),
    finalReply
      ? {
          id: "assistant-final-reply",
          title: "",
          detail: finalReply,
          source: null,
          marker: "assistant" as const,
        }
      : null,
  ].filter((row): row is TimelineRow => row != null && hasTimelineContent(row));

  return (
    <div className="min-w-0 space-y-1.5">
      {rows.map((row) => (
        <TimelineRowView key={row.id} row={row} />
      ))}
    </div>
  );
}

interface TimelineRow {
  id: string;
  title: string;
  detail: string;
  source: StreamItem | null;
  marker: "assistant" | "tool" | "status" | "error";
  toolResult?: TimelineToolResult;
}

interface TimelineToolResult {
  title: string;
  detail: string;
  success: boolean;
}

function timelineRowsForSteps(steps: StreamItem[]): TimelineRow[] {
  const rows: TimelineRow[] = [];
  for (let index = 0; index < steps.length; index += 1) {
    const step = steps[index];
    const behavior = processStepBehavior(step);
    if (behavior.title.startsWith("调用工具：")) {
      const nextStep = steps[index + 1];
      const nextBehavior = nextStep ? processStepBehavior(nextStep) : null;
      const hasToolResult = nextStep?.activity === "tool_result" && nextBehavior != null;
      if (behavior.detail.trim()) {
        rows.push({
          id: `${step.id}-public-update`,
          title: "",
          detail: behavior.detail,
          source: step,
          marker: "assistant",
        });
      }
      rows.push({
        id: `${step.id}-tool-call`,
        title: behavior.title,
        detail: "",
        source: step,
        marker: toolMarker(nextStep, nextBehavior),
        toolResult: hasToolResult
          ? {
              title: nextBehavior.title,
              detail: nextBehavior.detail,
              success: !isFailedToolResult(nextStep, nextBehavior),
            }
          : undefined,
      });
      if (hasToolResult) index += 1;
      continue;
    }

    rows.push(...timelineRowsForStep(step, step.id));
  }
  return rows;
}

function timelineRowsForStep(step: StreamItem, idPrefix: string): TimelineRow[] {
  const behavior = processStepBehavior(step);
  return timelineRowsFromBehavior({
    idPrefix,
    title: behavior.title,
    detail: behavior.detail,
    source: step,
    marker: rowMarkerForStep(step),
  });
}

function timelineRowsForItem(item: StreamItem, idPrefix: string): TimelineRow[] {
  return timelineRowsFromBehavior({
    idPrefix,
    title: item.title ?? kindLabel(item.kind),
    detail: item.content,
    source: item,
    marker: rowMarkerForStep(item),
  });
}

function timelineRowsFromBehavior({
  idPrefix,
  title,
  detail,
  source,
  marker,
}: {
  idPrefix: string;
  title: string;
  detail: string;
  source: StreamItem;
  marker: TimelineRow["marker"];
}): TimelineRow[] {
  if (title.startsWith("调用工具：")) {
    const rows: TimelineRow[] = [];
    if (detail.trim()) {
      rows.push({
        id: `${idPrefix}-public-update`,
        title: "",
        detail,
        source,
        marker: "assistant",
      });
    }
    rows.push({
      id: `${idPrefix}-tool-call`,
      title,
      detail: "",
      source,
      marker: "tool",
    });
    return rows;
  }
  if (title.startsWith("工具返回：")) {
    return [
      {
        id: `${idPrefix}-tool-summary`,
        title,
        detail,
        source,
        marker,
      },
    ];
  }
  if (title === "Agent 说明" || title === "Agent 正在回复") {
    return [
      {
        id: idPrefix,
        title: "",
        detail,
        source,
        marker,
      },
    ];
  }
  return [
    {
      id: idPrefix,
      title,
      detail,
      source,
      marker,
    },
  ];
}

function TimelineRowView({ row }: { row: TimelineRow }) {
  if (isToolBoxRow(row)) {
    return <ToolBox row={row} />;
  }
  if (isActiveThinkingRow(row)) {
    return (
      <div className="flex min-w-0 items-center gap-1.5 text-book-text-sub">
        <LoadingSpinner size="sm" />
        <span className="min-w-0 break-words">{row.title}</span>
      </div>
    );
  }
  return (
    <div className="min-w-0">
      {row.title ? (
        <div className="break-words font-semibold text-book-text-main/80">
          {row.title}
        </div>
      ) : null}
      {shouldShowTimelineDetail(row) ? (
        <MarkdownText
          className="whitespace-pre-wrap break-words text-book-text-sub"
          text={row.detail}
        />
      ) : null}
    </div>
  );
}

function ToolBox({ row }: { row: TimelineRow }) {
  const success =
    row.marker !== "error" &&
    !row.title.startsWith("工具失败") &&
    row.toolResult?.success !== false;
  const statusClass = success
    ? "bg-success"
    : "bg-error";
  return (
    <div className="min-w-0 overflow-hidden rounded-xl border border-book-border/70 bg-book-bg-tertiary/35 px-2.5 py-2">
      <div className="flex min-w-0 items-start gap-1.5">
        <span className={`mt-1 h-2 w-2 shrink-0 rounded-[2px] ${statusClass}`} />
        <div className="min-w-0 flex-1">
          <div className="break-words font-semibold text-book-text-main/80">
            {row.title}
          </div>
          {row.detail.trim() ? (
            <MarkdownText
              className="mt-0.5 whitespace-pre-wrap break-words text-book-text-sub"
              text={row.detail}
            />
          ) : null}
          {row.toolResult?.detail.trim() ? (
            <MarkdownText
              className="mt-1 whitespace-pre-wrap break-words text-book-text-sub"
              text={row.toolResult.detail}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}

function isToolBoxRow(row: TimelineRow): boolean {
  return (
    row.marker === "tool" ||
    row.title.startsWith("调用工具") ||
    row.title.startsWith("工具返回") ||
    row.title.startsWith("工具失败") ||
    row.source?.activity === "tool_result"
  );
}

function hasTimelineContent(row: TimelineRow): boolean {
  return Boolean(
    row.title.trim() ||
      row.detail.trim() ||
      row.toolResult?.detail.trim() ||
      isActiveThinkingRow(row),
  );
}

function shouldShowTimelineDetail(row: TimelineRow): boolean {
  if (!row.detail.trim()) return false;
  if (isThinkingTimelineRow(row)) return false;
  return true;
}

function isThinkingTimelineRow(row: TimelineRow): boolean {
  return row.title.includes("思考");
}

function isActiveThinkingRow(row: TimelineRow): boolean {
  return (
    row.title.includes("思考") &&
    row.source?.activity === "llm_response" &&
    row.source.status === "running"
  );
}

function isLiveModelItem(item: StreamItem): boolean {
  return item.activity === "llm_response" && item.status === "running";
}

function isAssistantReplyItem(item: StreamItem): boolean {
  return item.kind === "assistant" && item.activity !== "llm_response";
}

function visibleProcessSteps(
  steps: StreamItem[] | undefined,
  isProcessItem: boolean,
  currentStep: StreamItem | null,
): StreamItem[] {
  if (!steps?.length) return [];
  const visible = steps.filter(isVisibleProcessStep);
  if (
    isProcessItem &&
    visible.length === 1 &&
    currentStep &&
    visible[0].id === currentStep.id
  ) {
    return [];
  }
  return visible;
}

function isVisibleProcessStep(step: StreamItem): boolean {
  if (step.kind === "error" || step.status === "error" || step.status === "cancelled") {
    return true;
  }
  if (step.activity === "llm_response") {
    return !isCompletedHiddenLlmStep(step);
  }
  if (step.activity === "tool_result") return true;
  if (step.activity === "progress") return false;
  return false;
}

function isCompletedHiddenLlmStep(step: StreamItem): boolean {
  if (step.activity !== "llm_response") return false;
  if (step.status === "running") return false;
  const title = processStepBehavior(step).title;
  return title.includes("思考") || title === "Agent 回复" || title === "分析结果";
}

function rowMarkerForStep(step: StreamItem): TimelineRow["marker"] {
  if (step.kind === "error" || step.status === "error" || step.status === "cancelled") {
    return "error";
  }
  if (step.activity === "tool_result") return "tool";
  if (step.activity === "llm_response") return "assistant";
  return "status";
}

function toolMarker(
  step: StreamItem | undefined,
  behavior: ReturnType<typeof processStepBehavior> | null,
): TimelineRow["marker"] {
  if (!step) return "tool";
  return isFailedToolResult(step, behavior) ? "error" : "tool";
}

function isFailedToolResult(
  step: StreamItem,
  behavior: ReturnType<typeof processStepBehavior> | null,
): boolean {
  return (
    step.kind === "error" ||
    step.status === "error" ||
    behavior?.title.startsWith("工具失败") === true
  );
}

function MarkdownText({ text, className }: { text: string; className?: string }) {
  const blocks = markdownBlocks(text);
  if (blocks.length === 0) return null;
  return (
    <div className={`min-w-0 ${className ?? ""}`}>
      {blocks.map((block, index) => {
        if (block.type === "ul") {
          return (
            <ul key={index} className="my-1 min-w-0 list-disc space-y-0.5 pl-4">
              {block.items.map((item, itemIndex) => (
                <li key={itemIndex}>{renderInlineMarkdown(item)}</li>
              ))}
            </ul>
          );
        }
        if (block.type === "ol") {
          return (
            <ol key={index} className="my-1 min-w-0 list-decimal space-y-0.5 pl-4">
              {block.items.map((item, itemIndex) => (
                <li key={itemIndex}>{renderInlineMarkdown(item)}</li>
              ))}
            </ol>
          );
        }
        if (block.type === "p") {
          return (
            <p key={index} className="my-1 min-w-0 first:mt-0 last:mb-0">
              {renderInlineMarkdown(block.text)}
            </p>
          );
        }
        return null;
      })}
    </div>
  );
}

type MarkdownBlock =
  | { type: "p"; text: string }
  | { type: "ul" | "ol"; items: string[] };

function markdownBlocks(text: string): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  let paragraph: string[] = [];
  let list: { type: "ul" | "ol"; items: string[] } | null = null;

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    blocks.push({ type: "p", text: paragraph.join("\n") });
    paragraph = [];
  };
  const flushList = () => {
    if (!list) return;
    blocks.push(list);
    list = null;
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      flushParagraph();
      flushList();
      continue;
    }
    const unordered = /^[-*]\s+(.+)$/.exec(trimmed);
    if (unordered) {
      flushParagraph();
      if (!list || list.type !== "ul") {
        flushList();
        list = { type: "ul", items: [] };
      }
      list.items.push(unordered[1]);
      continue;
    }
    const ordered = /^\d+[.)]\s+(.+)$/.exec(trimmed);
    if (ordered) {
      flushParagraph();
      if (!list || list.type !== "ol") {
        flushList();
        list = { type: "ol", items: [] };
      }
      list.items.push(ordered[1]);
      continue;
    }
    flushList();
    paragraph.push(line);
  }
  flushParagraph();
  flushList();
  return blocks;
}

function renderInlineMarkdown(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let lastIndex = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > lastIndex) {
      nodes.push(text.slice(lastIndex, match.index));
    }
    const value = match[0];
    if (value.startsWith("**")) {
      nodes.push(<strong key={nodes.length}>{value.slice(2, -2)}</strong>);
    } else {
      nodes.push(
        <code
          key={nodes.length}
          className="break-all rounded-md bg-book-bg-tertiary px-1 py-0.5 font-mono text-[0.92em]"
        >
          {value.slice(1, -1)}
        </code>,
      );
    }
    lastIndex = match.index + value.length;
  }
  if (lastIndex < text.length) {
    nodes.push(text.slice(lastIndex));
  }
  return nodes;
}

function isProcessBubble(item: StreamItem): boolean {
  return (
    item.id.startsWith("message-process-") ||
    item.id.startsWith("progress-turn-") ||
    item.title === "Agent 过程" ||
    item.title === "Agent 运行中"
  );
}

function currentProcessStep(steps: StreamItem[] | undefined): StreamItem | null {
  if (!steps?.length) return null;
  const reversed = [...steps].reverse();
  return (
    reversed.find(isMeaningfulProcessStep) ??
    reversed.find((step) => step.activity !== "agent_event" && !isGenericWaitingProgress(step)) ??
    steps[steps.length - 1]
  );
}

function isMeaningfulProcessStep(step: StreamItem): boolean {
  return step.activity === "tool_result" || step.activity === "llm_response";
}

function isGenericWaitingProgress(step: StreamItem): boolean {
  if (step.activity !== "progress") return false;
  return /等待.*Agent.*(决策|回复)/.test(step.content);
}

function StreamIcon({
  kind,
  status,
}: {
  kind: StreamItemKind;
  status?: ProgressDialogStatus;
}) {
  const className =
    kind === "user"
      ? "border-book-primary/30 bg-book-primary/10 text-book-primary"
      : kind === "error"
        ? "bg-error-bg text-error"
        : kind === "tool"
          ? "bg-success-bg text-success"
          : "bg-book-bg-tertiary text-book-text-sub";
  return (
    <span
      className={`mt-0.5 inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-book-border ${className}`}
    >
      {kind === "user" ? (
        <User size={13} />
      ) : kind === "tool" ? (
        <Wrench size={13} />
      ) : kind === "error" ? (
        <AlertCircle size={13} />
      ) : status === "success" ? (
        <CheckCircle2 size={13} />
      ) : status === "running" ? (
        <LoadingSpinner size="sm" />
      ) : (
        <Bot size={13} />
      )}
    </span>
  );
}
