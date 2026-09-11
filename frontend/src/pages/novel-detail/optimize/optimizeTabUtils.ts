import type { ProgressDialogStatus } from "../../../components/feedback/ProgressDialog";
import type { ChapterSummary } from "../../../api/chapter";
import type {
  OptimizePatchSet,
  OptimizeSession,
  OptimizeTurn,
  OptimizeTurnEvent,
} from "../../../api/optimize";
import {
  isJsonRecord as isRecord,
  type JsonRecord,
} from "../../../utils/jsonRecord";
import { nonEmptyText } from "../../../utils/text";
import { formatTime } from "../../../i18n";
import type { Locale, MessageTranslator } from "../../../i18n";

export type StreamItemKind = "user" | "assistant" | "agent" | "tool" | "error";
type StreamItemActivity =
  | "assistant_update"
  | "llm_response"
  | "tool_call"
  | "tool_result"
  | "turn_error"
  | "agent_event";

export interface StreamItem {
  id: string;
  kind: StreamItemKind;
  activity?: StreamItemActivity;
  title?: string;
  content: string;
  streamSource?: string;
  steps?: StreamItem[];
  detail?: string | null;
  detailLabel?: string;
  timestamp?: number | string | null;
  taskId?: string | null;
  status?: ProgressDialogStatus;
  chapterId?: number | null;
  sessionId?: number | null;
  persistedTurnCountBefore?: number;
}

interface ProcessBehavior {
  title: string;
  detail: string;
}

interface OptimizeIssue {
  id: string;
  tool: string;
  description?: string | null;
  severity?: string | null;
  arguments?: unknown;
}

export type OptimizeSubmitKind = "message" | "analyze" | "propose_patches";

export const chapterOptionLabel = (
  chapter: ChapterSummary,
  t: MessageTranslator,
): string => {
  const title = nonEmptyText(chapter.title);
  return title
    ? t("optimize.chapterOption", { number: chapter.chapter_number, title })
    : t("optimize.chapterOptionUntitled", { number: chapter.chapter_number });
};

export const initialSelectedChapter = (
  chapterId: number | null | undefined,
): number | "" => (chapterId == null ? "" : chapterId);

export const latestPendingPatchSet = (
  patchSets: OptimizePatchSet[],
): OptimizePatchSet | null => patchSets.find((item) => item.status === "pending") ?? null;

export function persistedTurnCount(turns: OptimizeTurn[], text: string): number {
  const normalized = text.trim();
  return turns.filter((turn) => turn.user_text.trim() === normalized).length;
}

export function requireSessionName(
  name: string | null | undefined,
  t: MessageTranslator,
): string {
  const trimmed = name?.trim();
  if (!trimmed) {
    throw new Error(t("optimize.sessionNameTitle"));
  }
  return trimmed;
}

export const isNearBottom = (element: HTMLElement): boolean =>
  element.scrollHeight - element.scrollTop - element.clientHeight < 96;

export const isReusableAgentSession = (session: OptimizeSession): boolean =>
  session.status === "waiting_user" || session.status === "completed";

export const sessionDisplayName = (
  session: OptimizeSession | null | undefined,
  t: MessageTranslator,
): string => nonEmptyText(session?.name) || t("optimize.unnamedSession");

export function itemClassName(kind: StreamItemKind): string {
  switch (kind) {
    case "user":
      return "border-book-primary/30 bg-[rgb(var(--color-primary-pale))] text-book-text-main";
    case "assistant":
      return "border-book-border bg-book-bg-paper text-book-text-main";
    case "tool":
      return "border-success/40 bg-success-bg text-book-text-main";
    case "error":
      return "border-error/50 bg-error-bg text-book-text-main";
    default:
      return "border-book-border bg-book-bg-tertiary text-book-text-main";
  }
}

export function kindLabel(kind: StreamItemKind, t: MessageTranslator): string {
  switch (kind) {
    case "user":
      return t("optimize.kindUser");
    case "assistant":
      return t("optimize.kindAssistant");
    case "tool":
      return t("optimize.kindTool");
    case "error":
      return t("optimize.kindError");
    default:
      return t("optimize.kindAgent");
  }
}

export function formatItemMeta(item: StreamItem, locale: Locale): string | null {
  if (typeof item.timestamp === "number") {
    return formatTime(item.timestamp, locale);
  }
  if (typeof item.timestamp === "string") {
    const time = new Date(item.timestamp).getTime();
    return Number.isNaN(time) ? null : formatTime(time, locale);
  }
  return null;
}

export function streamItemBelongsToContext(
  item: StreamItem,
  chapterId: number | null,
  sessionId: number | null,
): boolean {
  if (item.chapterId != null && item.chapterId !== chapterId) return false;
  if (item.sessionId == null) return true;
  return sessionId != null && item.sessionId === sessionId;
}

export function sortStreamActivityItems(items: StreamItem[]): StreamItem[] {
  return items
    .map((item, index) => ({ item, index, time: streamItemTimestampMs(item) }))
    .sort((left, right) => {
      if (left.time == null && right.time == null) return left.index - right.index;
      if (left.time == null) return 1;
      if (right.time == null) return -1;
      return left.time === right.time ? left.index - right.index : left.time - right.time;
    })
    .map(({ item }) => item);
}

export function turnsToConversationItems(
  turns: OptimizeTurn[],
  t: MessageTranslator,
): StreamItem[] {
  const items: StreamItem[] = [];
  for (const turn of turns) {
    items.push({
      id: `turn-${turn.id}-user`,
      kind: "user",
      content: turn.user_text,
      timestamp: turn.created_at,
      sessionId: turn.session_id,
    });

    const steps = turn.events.map((event) => turnEventToStreamItem(turn, event, t));
    const failed = turn.status === "failed" || turn.status === "cancelled";
    const finalReply = turn.final_reply?.trim();
    const content = finalReply || turn.error?.trim() || (turn.status === "running" ? t("workflow.running") : "");
    items.push({
      id: `turn-${turn.id}-agent`,
      kind: failed ? "error" : "assistant",
      content,
      timestamp: turn.updated_at,
      status: turnStatusToProgressStatus(turn.status),
      steps,
      sessionId: turn.session_id,
    });
  }
  return items;
}

function turnEventToStreamItem(
  turn: OptimizeTurn,
  event: OptimizeTurnEvent,
  t: MessageTranslator,
): StreamItem {
  const payload = event.payload == null ? null : JSON.stringify(event.payload, null, 2);
  const base = {
    id: `turn-event-${event.id}`,
    timestamp: event.created_at,
    status: turnStatusToProgressStatus(event.status),
    sessionId: turn.session_id,
  };

  if (event.kind === "assistant_update") {
    return {
      ...base,
      kind: "assistant",
      activity: "assistant_update",
      content: event.public_text,
      detail: payload,
    };
  }
  if (event.kind === "tool_call") {
    return {
      ...base,
      kind: "tool",
      activity: "tool_call",
      title: t("optimize.toolCall"),
      content: event.public_text,
      detail: payload,
    };
  }
  if (event.kind === "tool_result") {
    return {
      ...base,
      kind: event.status === "failed" ? "error" : "tool",
      activity: "tool_result",
      title: t("optimize.toolResult"),
      content: event.public_text,
      detail: payload,
    };
  }
  if (event.kind === "error") {
    return {
      ...base,
      kind: "error",
      activity: "turn_error",
      title: t("optimize.executionFailed"),
      content: event.public_text,
      detail: payload,
    };
  }

  return {
    ...base,
    kind: "agent",
    activity: "agent_event",
    title: event.kind,
    content: event.public_text,
    detail: payload,
  };
}

function turnStatusToProgressStatus(status: string): ProgressDialogStatus | undefined {
  if (status === "running") return "running";
  if (status === "completed") return "success";
  if (status === "failed") return "error";
  if (status === "cancelled") return "cancelled";
  return undefined;
}

export function processStepBehavior(
  step: StreamItem,
  t: MessageTranslator,
): ProcessBehavior {
  if (step.kind === "error" || step.status === "error") {
    return {
      title: t("optimize.executionFailed"),
      detail: compactBehaviorText(step.content) || t("optimize.checkErrorDetails"),
    };
  }
  if (step.status === "cancelled") {
    return {
      title: t("optimize.taskStopped"),
      detail: compactBehaviorText(step.content) || t("optimize.stoppedByUser"),
    };
  }

  switch (step.activity) {
    case "llm_response":
      return llmBehavior(step.detail, t);
    case "tool_result":
      return toolBehavior(step.detail, t);
    case "agent_event":
      return { title: t("optimize.recordedStatus"), detail: compactBehaviorText(step.content) || "" };
    case "assistant_update":
      return { title: t("optimize.agentNote"), detail: step.content };
    case "tool_call":
      return turnToolCallBehavior(step, t);
    case "turn_error":
      return {
        title: t("optimize.executionFailed"),
        detail: compactBehaviorText(step.content) || t("optimize.checkErrorDetails"),
      };
    default:
      break;
  }

  if (step.status === "running") return { title: t("optimize.runningStep"), detail: compactBehaviorText(step.content) || "" };
  if (step.status === "success") return { title: t("optimize.completedStep"), detail: compactBehaviorText(step.content) || t("optimize.stepCompleted") };
  return { title: t("optimize.recordedStatus"), detail: compactBehaviorText(step.content) || "" };
}

function turnToolCallBehavior(step: StreamItem, t: MessageTranslator): ProcessBehavior {
  const parsed = step.detail ? parseJsonObject(step.detail) : null;
  const tool = parsed ? stringField(parsed, "tool") : null;
  if (step.activity === "tool_call" && step.title) {
    return { title: step.title, detail: "" };
  }
  return { title: tool ? t("optimize.callToolNamed", { tool }) : t("optimize.callTool"), detail: "" };
}

function llmBehavior(detail: string | null | undefined, t: MessageTranslator): ProcessBehavior {
  const parsed = detail ? parseJsonObject(detail) : null;
  const action = parsed && typeof parsed.action === "string" ? parsed.action : null;
  const publicUpdate = parsed ? stringField(parsed, "public_update") : null;
  const args = parsed && isRecord(parsed.arguments) ? parsed.arguments : null;

  switch (action) {
    case "think":
      return publicUpdate
        ? { title: t("optimize.agentNote"), detail: publicUpdate }
        : { title: t("optimize.modelStageMissing"), detail: t("optimize.thinkPublicUpdateMissing") };
    case "run_tool":
      return toolCallBehavior(args, publicUpdate, t);
    case "final":
      return finalDecisionBehavior(args, t);
    default:
      return {
        title: t("optimize.modelDecisionUnknown"),
        detail: action ? t("optimize.unknownAction", { action }) : t("optimize.missingAction"),
      };
  }
}

function toolCallBehavior(
  args: Record<string, unknown> | null,
  publicUpdate: string | null | undefined,
  t: MessageTranslator,
): ProcessBehavior {
  const tool = args ? stringField(args, "tool") : null;
  const toolArgs = args && isRecord(args.arguments) ? args.arguments : null;
  const detail = publicUpdate || t("optimize.runToolUpdateMissing");
  if (tool === "get_project_information") {
    const mode = toolArgs ? stringField(toolArgs, "mode") : null;
    return {
      title: mode === "search" ? t("optimize.callSearch") : t("optimize.callRead"),
      detail,
    };
  }
  if (tool === "edit_article") {
    return {
      title: t("optimize.callEditArticle"),
      detail,
    };
  }
  return { title: tool ? t("optimize.callToolNamed", { tool }) : t("optimize.callTool"), detail };
}

function finalDecisionBehavior(args: Record<string, unknown> | null, t: MessageTranslator): ProcessBehavior {
  const reply = args ? stringField(args, "reply") : null;
  if (reply) {
    return { title: t("optimize.agentReply"), detail: reply };
  }
  if (args && Array.isArray(args.issues)) {
    return { title: t("optimize.analysisResult"), detail: formatIssueList(args.issues, t) };
  }
  return { title: t("optimize.invalidFinalArguments"), detail: t("optimize.missingReplyOrIssues") };
}

function toolBehavior(detail: string | null | undefined, t: MessageTranslator): ProcessBehavior {
  const parsed = detail ? parseJsonObject(detail) : null;
  if (!parsed) return { title: t("optimize.toolResultUnparseable"), detail: "" };
  const tool = typeof parsed.tool === "string" ? parsed.tool : null;
  const result = isRecord(parsed.result) ? parsed.result : parsed;
  const resultTool = typeof result.tool === "string" ? result.tool : tool;
  const success = typeof parsed.success === "boolean" ? parsed.success : null;
  if (success === false) {
    const error = stringField(parsed, "error") ?? stringField(result, "error") ?? t("optimize.toolExecutionFailed");
    return { title: t("optimize.toolFailed", { tool: runtimeToolLabel(resultTool, t) }), detail: error };
  }

  if (resultTool === "get_project_information") {
    const mode = stringField(result, "mode");
    const source = stringField(result, "source");
    const matches = Array.isArray(result.matches) ? result.matches.length : null;
    return {
      title: mode === "search" ? t("optimize.toolSearchResult") : t("optimize.toolReadResult"),
      detail:
        matches != null
          ? t("optimize.matchesFound", { count: matches })
          : source
            ? t("optimize.sourceReceived", { source: informationSourceLabel(source, t) })
            : t("optimize.contextReceived"),
    };
  }
  if (resultTool === "apply_chapter_patches") {
    return { title: t("optimize.appliedPatchResult"), detail: patchCountDetail("written", result, t) };
  }
  if (resultTool === "edit_article") {
    const mode = stringField(result, "mode");
    if (mode === "applied") return { title: t("optimize.toolReturned"), detail: patchCountDetail("applied", result, t) };
    if (mode === "pending_confirmation") {
      return { title: t("optimize.pendingPatchResult"), detail: patchCountDetail("review", result, t) };
    }
    if (mode === "no_change") return { title: t("optimize.noChangeResult"), detail: t("optimize.noPatchGenerated") };
    return { title: t("optimize.patchProposalResult"), detail: patchCountDetail("generated", result, t) };
  }
  return { title: t("optimize.toolReturned"), detail: resultTool || t("optimize.toolResultReceived") };
}

function patchCountDetail(
  verb: "written" | "applied" | "review" | "generated",
  payload: Record<string, unknown>,
  t: MessageTranslator,
): string {
  const count =
    numberField(payload, "patch_count") ??
    numberField(payload, "applied_count") ??
    (Array.isArray(payload.patches) ? payload.patches.length : null);
  if (count == null) {
    return t(
      verb === "written"
        ? "optimize.wroteChanges"
        : verb === "applied"
          ? "optimize.appliedChanges"
          : verb === "review"
            ? "optimize.generatedReviewChanges"
            : "optimize.generatedChanges",
    );
  }
  const label =
    verb === "written"
      ? t("optimize.verbWritten")
      : verb === "applied"
        ? t("optimize.verbApplied")
        : verb === "review"
          ? t("optimize.verbReviewGenerated")
          : t("optimize.verbGenerated");
  return t("optimize.changesCount", { verb: label, count });
}

function informationSourceLabel(source: string, t: MessageTranslator): string {
  switch (source) {
    case "current_chapter":
      return t("optimize.sourceCurrentChapter");
    case "current_chapter_outline":
      return t("optimize.sourceCurrentOutline");
    case "project_detail":
      return t("optimize.sourceProjectDetail");
    case "blueprint":
      return t("optimize.sourceBlueprint");
    case "part_outlines":
      return t("optimize.sourcePartOutlines");
    case "chapter_outlines":
      return t("optimize.sourceChapterOutlines");
    case "character_profiles":
      return t("optimize.sourceCharacters");
    case "foreshadowing_index":
      return t("optimize.sourceForeshadowing");
    case "previous_chapters":
      return t("optimize.sourcePreviousChapters");
    case "next_chapters":
      return t("optimize.sourceNextChapters");
    default:
      return source;
  }
}

function compactBehaviorText(text?: string | null): string | null {
  const value = text?.replace(/\s+/g, " ").trim();
  if (!value) return null;
  return value.length > 48 ? `${value.slice(0, 48)}...` : value;
}

export function parseOptimizeIssues(raw: unknown): OptimizeIssue[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((item): item is Record<string, unknown> => typeof item === "object" && item !== null)
    .filter((item) => typeof item.id === "string" && typeof item.tool === "string")
    .map((item) => ({
      id: item.id as string,
      tool: item.tool as string,
      description: typeof item.description === "string" ? item.description : null,
      severity: typeof item.severity === "string" ? item.severity : null,
      arguments: item.arguments,
    }));
}

export function statusLabel(status: string, t: MessageTranslator): string {
  switch (status) {
    case "analyzing":
      return t("optimize.statusAnalyzing");
    case "waiting_user":
      return t("optimize.statusWaiting");
    case "fixing":
      return t("optimize.statusFixing");
    case "completed":
      return t("optimize.statusCompleted");
    case "cancelled":
      return t("optimize.statusCancelled");
    default:
      return status;
  }
}

function streamItemTimestampMs(item: StreamItem): number | null {
  if (typeof item.timestamp === "number") return item.timestamp;
  if (typeof item.timestamp === "string") {
    const time = new Date(item.timestamp).getTime();
    return Number.isNaN(time) ? null : time;
  }
  return null;
}

function runtimeToolLabel(tool: string | null, t: MessageTranslator): string {
  switch (tool) {
    case "get_project_information":
      return t("optimize.toolReadResult");
    case "edit_article":
      return t("optimize.callEditArticle");
    case "apply_chapter_patches":
      return t("optimize.applyPatches");
    default:
      return tool || t("optimize.kindTool");
  }
}

export function stringField(record: JsonRecord, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function numberField(record: Record<string, unknown>, key: string): number | null {
  const value = record[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function formatIssueList(rawIssues: unknown[], t: MessageTranslator): string {
  const issues = parseOptimizeIssues(rawIssues);
  if (issues.length === 0) return t("optimize.noIssues");
  return [
    t("optimize.issueCount", { count: issues.length }),
    ...issues.map((issue, index) => {
      const severity = issue.severity ? ` / ${issue.severity}` : "";
      return t("optimize.issueLine", {
        index: index + 1,
        description: issue.description || t("optimize.issueDescriptionMissing"),
        tool: issue.tool,
        severity,
        id: issue.id,
      });
    }),
  ].join("\n");
}

export function patchOperationLabel(operation: string, t: MessageTranslator): string {
  switch (operation) {
    case "replace":
      return t("optimize.operationReplace");
    case "delete":
      return t("optimize.operationDelete");
    case "insert_before":
      return t("optimize.operationInsertBefore");
    case "insert_after":
      return t("optimize.operationInsertAfter");
    default:
      return operation;
  }
}

export function parseJsonObject(content: string): JsonRecord | null {
  try {
    const parsed = JSON.parse(content);
    return isRecord(parsed) ? parsed : null;
  } catch {
    return null;
  }
}
