import { useEffect } from "react";
import { AlertCircle, Bot, CheckCircle2, User, Wrench } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { usePacedStreamText } from "../../../hooks/usePacedStreamText";
import { MarkdownText } from "../../../components/ui/MarkdownText";
import { useI18n, type MessageTranslator } from "../../../i18n";

import {
  formatItemMeta,
  itemClassName,
  kindLabel,
  processStepBehavior,
  type StreamItem,
  type StreamItemKind,
} from "./optimizeTabUtils";
import { previewModelStream } from "./modelStreamPreview";
import type { ProgressDialogStatus } from "../../../components/feedback/ProgressDialog";

export function StreamMessage({
  item,
  onStreamDrained,
}: {
  item: StreamItem;
  onStreamDrained?: (taskId: string) => void;
}) {
  const { locale, t } = useI18n();
  const hasStreamSource = item.streamSource != null;
  const live = hasStreamSource && item.status === "running";
  const pacedStream = usePacedStreamText(
    item.streamSource ?? item.content,
    live,
    hasStreamSource,
  );
  const pacedPreview = hasStreamSource && pacedStream.displayedText
    ? previewModelStream(pacedStream.displayedText, t)
    : null;
  const displayStatus = hasStreamSource && !pacedStream.isDrained
    ? "running"
    : item.status;
  const displayItem = hasStreamSource
    ? pacedPreview
      ? {
          ...item,
          kind: pacedPreview.kind,
          title: pacedPreview.title,
          content: pacedPreview.content,
          status: displayStatus,
        }
      : {
          ...item,
          kind: "agent" as const,
          title: t("optimize.thinking"),
          content: "",
          status: displayStatus,
        }
    : item;
  useEffect(() => {
    if (
      !hasStreamSource
      || live
      || !pacedStream.isDrained
      || !item.taskId
    ) return;
    onStreamDrained?.(item.taskId);
  }, [
    hasStreamSource,
    item.taskId,
    live,
    onStreamDrained,
    pacedStream.isDrained,
  ]);
  const meta = formatItemMeta(item, locale);
  const currentStep = currentProcessStep(displayItem.steps);
  const currentBehavior = currentStep ? processStepBehavior(currentStep, t) : null;
  const isProcessItem = isProcessBubble(displayItem) && currentBehavior != null;
  const title = isProcessItem
    ? currentBehavior.title
    : displayItem.title ?? kindLabel(displayItem.kind, t);
  const content = isProcessItem ? currentBehavior.detail : displayItem.content;
  const contentClassName = "whitespace-pre-wrap break-words";
  const processSteps = visibleProcessSteps(displayItem.steps, isProcessItem, currentStep, t);
  const showTimeline = displayItem.kind !== "user"
    && (processSteps.length > 0 || hasStreamSource);
  const showHeader = !showTimeline;
  return (
    <article
      className={`grid min-w-0 items-start gap-2 ${
        displayItem.kind === "user"
          ? "grid-cols-[minmax(0,1fr)_auto]"
          : "grid-cols-[auto_minmax(0,1fr)]"
      }`}
    >
      {displayItem.kind !== "user" ? <StreamIcon kind={displayItem.kind} status={displayItem.status} /> : null}
      <div
        className={`min-w-0 max-w-[96%] overflow-hidden rounded-2xl border px-3 py-2.5 text-xs leading-relaxed ${
          displayItem.kind === "user" ? "max-w-[90%] justify-self-end rounded-br-md" : "rounded-bl-md"
        } ${
          itemClassName(displayItem.kind)
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
            currentItem={hasStreamSource ? displayItem : null}
            finalReply={isAssistantReplyItem(displayItem) ? content : null}
            showThinking={pacedStream.isThinking && live}
            t={t}
          />
        ) : (
          <MarkdownText className={contentClassName} text={content} />
        )}
        {displayItem.detail && !displayItem.steps?.length ? (
          <details className="mt-2 min-w-0 overflow-hidden rounded-xl border border-book-border/70 bg-book-bg-paper/60">
            <summary className="cursor-pointer select-none px-2 py-1 text-small font-semibold text-book-text-sub hover:text-book-text-main">
              {displayItem.detailLabel ?? t("optimize.detail")}
            </summary>
            <pre className="whitespace-pre-wrap break-words border-t border-book-border/70 px-2 py-2 font-mono text-small leading-relaxed text-book-text-sub">
              {displayItem.detail}
            </pre>
          </details>
        ) : null}
      </div>
      {displayItem.kind === "user" ? <StreamIcon kind={displayItem.kind} status={displayItem.status} /> : null}
    </article>
  );
}

function AgentTurnTimeline({
  steps,
  currentItem,
  finalReply,
  showThinking,
  t,
}: {
  steps: StreamItem[];
  currentItem: StreamItem | null;
  finalReply: string | null;
  showThinking: boolean;
  t: MessageTranslator;
}) {
  const rows: TimelineRow[] = [
    ...timelineRowsForSteps(steps, t),
    ...(currentItem ? timelineRowsForItem(currentItem, `${currentItem.id}-current`, t) : []),
    finalReply
      ? {
          id: "assistant-final-reply",
          title: "",
          detail: finalReply,
          source: null,
          marker: "assistant" as const,
        }
      : null,
  ].filter((row): row is TimelineRow => row != null && hasTimelineContent(row, t));

  return (
    <div className="min-w-0 space-y-1.5">
      {rows.map((row) => (
        <TimelineRowView key={row.id} row={row} t={t} />
      ))}
      {showThinking && !rows.some((row) => isActiveThinkingRow(row, t)) ? (
        <div className="flex min-w-0 items-center gap-1.5 text-book-text-sub">
          <LoadingSpinner size="sm" />
          <span>{t("optimize.thinking")}</span>
        </div>
      ) : null}
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

function timelineRowsForSteps(steps: StreamItem[], t: MessageTranslator): TimelineRow[] {
  const rows: TimelineRow[] = [];
  for (let index = 0; index < steps.length; index += 1) {
    const step = steps[index];
    const behavior = processStepBehavior(step, t);
    if (step.activity === "tool_call" || behavior.title.startsWith(t("optimize.callTool"))) {
      const nextStep = steps[index + 1];
      const nextBehavior = nextStep ? processStepBehavior(nextStep, t) : null;
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
        marker: toolMarker(nextStep, nextBehavior, t),
        toolResult: hasToolResult
          ? {
              title: nextBehavior.title,
              detail: nextBehavior.detail,
              success: !isFailedToolResult(nextStep, nextBehavior, t),
            }
          : undefined,
      });
      if (hasToolResult) index += 1;
      continue;
    }

    rows.push(...timelineRowsForStep(step, step.id, t));
  }
  return rows;
}

function timelineRowsForStep(step: StreamItem, idPrefix: string, t: MessageTranslator): TimelineRow[] {
  const behavior = processStepBehavior(step, t);
  return timelineRowsFromBehavior({
    idPrefix,
    title: behavior.title,
    detail: behavior.detail,
    source: step,
    marker: rowMarkerForStep(step),
    t,
  });
}

function timelineRowsForItem(item: StreamItem, idPrefix: string, t: MessageTranslator): TimelineRow[] {
  return timelineRowsFromBehavior({
    idPrefix,
    title: item.title ?? kindLabel(item.kind, t),
    detail: item.content,
    source: item,
    marker: rowMarkerForStep(item),
    t,
  });
}

function timelineRowsFromBehavior({
  idPrefix,
  title,
  detail,
  source,
  marker,
  t,
}: {
  idPrefix: string;
  title: string;
  detail: string;
  source: StreamItem;
  marker: TimelineRow["marker"];
  t: MessageTranslator;
}): TimelineRow[] {
  if (source?.activity === "tool_call" || title.startsWith(t("optimize.callTool"))) {
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
  if (source?.activity === "tool_result" || title.startsWith(t("optimize.toolReturned"))) {
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
  if (
    title === t("optimize.agentNote") ||
    title === t("optimize.agentReply")
  ) {
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

function TimelineRowView({ row, t }: { row: TimelineRow; t: MessageTranslator }) {
  if (isToolBoxRow(row)) {
    return <ToolBox row={row} />;
  }
  if (isActiveThinkingRow(row, t)) {
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
      {shouldShowTimelineDetail(row, t) ? (
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
    row.toolResult?.success !== false;
  const statusClass = success
    ? "bg-success"
    : "bg-error";
  const summary = row.toolResult?.detail.trim() || row.detail.trim();
  return (
    <div className="min-w-0 overflow-hidden rounded-xl border border-book-border/70 bg-book-bg-tertiary/35 px-2.5 py-1.5">
      <div className="flex min-w-0 items-center gap-1.5 whitespace-nowrap">
        <span className={`h-2 w-2 shrink-0 rounded-[2px] ${statusClass}`} />
        <span
          className="min-w-0 truncate font-semibold text-book-text-main/80"
          title={row.title}
        >
          {row.title}
        </span>
        {summary ? (
          <>
            <span className="shrink-0 text-book-text-muted" aria-hidden="true">
              ·
            </span>
            <span
              className="min-w-0 truncate text-book-text-sub"
              title={summary}
            >
              {summary}
            </span>
          </>
        ) : null}
      </div>
    </div>
  );
}

function isToolBoxRow(row: TimelineRow): boolean {
  return (
    row.marker === "tool" ||
    row.source?.activity === "tool_call" ||
    row.source?.activity === "tool_result" ||
    row.source?.kind === "tool"
  );
}

function hasTimelineContent(row: TimelineRow, t: MessageTranslator): boolean {
  return Boolean(
    row.title.trim() ||
      row.detail.trim() ||
      row.toolResult?.detail.trim() ||
      isActiveThinkingRow(row, t),
  );
}

function shouldShowTimelineDetail(row: TimelineRow, t: MessageTranslator): boolean {
  if (!row.detail.trim()) return false;
  if (isThinkingTimelineRow(row, t)) return false;
  return true;
}

function isThinkingTimelineRow(row: TimelineRow, t: MessageTranslator): boolean {
  return row.title === t("optimize.thinking");
}

function isActiveThinkingRow(row: TimelineRow, t: MessageTranslator): boolean {
  return (
    row.title === t("optimize.thinking") &&
    row.source?.activity === "llm_response" &&
    row.source.status === "running"
  );
}

function isAssistantReplyItem(item: StreamItem): boolean {
  return item.kind === "assistant" && item.activity !== "llm_response";
}

function visibleProcessSteps(
  steps: StreamItem[] | undefined,
  isProcessItem: boolean,
  currentStep: StreamItem | null,
  t: MessageTranslator,
): StreamItem[] {
  if (!steps?.length) return [];
  const visible = steps.filter((step) => isVisibleProcessStep(step, t));
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

function isVisibleProcessStep(step: StreamItem, t: MessageTranslator): boolean {
  if (step.kind === "error" || step.status === "error" || step.status === "cancelled") {
    return true;
  }
  if (step.activity === "llm_response") {
    return !isCompletedHiddenLlmStep(step, t);
  }
  if (step.activity === "tool_result") return true;
  return false;
}

function isCompletedHiddenLlmStep(step: StreamItem, t: MessageTranslator): boolean {
  if (step.activity !== "llm_response") return false;
  if (step.status === "running") return false;
  const title = processStepBehavior(step, t).title;
  return title === t("optimize.thinking") || title === t("optimize.agentReply") || title === t("optimize.analysisResult");
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
  t: MessageTranslator,
): TimelineRow["marker"] {
  if (!step) return "tool";
  return isFailedToolResult(step, behavior, t) ? "error" : "tool";
}

function isFailedToolResult(
  step: StreamItem,
  behavior: ReturnType<typeof processStepBehavior> | null,
  t: MessageTranslator,
): boolean {
  const failedPrefix = t("optimize.toolFailed", { tool: "" });
  return (
    step.kind === "error" ||
    step.status === "error" ||
    behavior?.title.startsWith(failedPrefix) === true
  );
}

function isProcessBubble(item: StreamItem): boolean {
  return item.id.startsWith("message-process-");
}

export function currentProcessStep(
  steps: StreamItem[] | undefined,
): StreamItem | null {
  if (!steps?.length) return null;
  let latestNonAgentStep: StreamItem | null = null;
  for (let index = steps.length - 1; index >= 0; index -= 1) {
    const step = steps[index];
    if (isMeaningfulProcessStep(step)) return step;
    if (latestNonAgentStep == null && step.activity !== "agent_event") {
      latestNonAgentStep = step;
    }
  }
  return latestNonAgentStep ?? steps[steps.length - 1];
}

function isMeaningfulProcessStep(step: StreamItem): boolean {
  return step.activity === "tool_result" || step.activity === "llm_response";
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
