import type {
  ProgressDialogStatus,
  ProgressTimelineItem,
} from "../../../components/feedback/ProgressDialog";
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

export type StreamItemKind = "user" | "assistant" | "agent" | "tool" | "error";
type StreamItemActivity =
  | "assistant_update"
  | "llm_response"
  | "tool_call"
  | "tool_result"
  | "turn_error"
  | "agent_event"
  | "progress";

export interface StreamItem {
  id: string;
  kind: StreamItemKind;
  activity?: StreamItemActivity;
  title?: string;
  content: string;
  steps?: StreamItem[];
  detail?: string | null;
  detailLabel?: string;
  timestamp?: number | string | null;
  taskId?: string | null;
  status?: ProgressDialogStatus;
  current?: number;
  total?: number;
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

export const chapterOptionLabel = (chapter: ChapterSummary): string => {
  const title = nonEmptyText(chapter.title);
  return title
    ? `第 ${chapter.chapter_number} 章 · ${title}`
    : `第 ${chapter.chapter_number} 章`;
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

export function requireSessionName(name: string | null | undefined): string {
  const trimmed = name?.trim();
  if (!trimmed) {
    throw new Error("请先输入会话名称");
  }
  return trimmed;
}

export const isNearBottom = (element: HTMLElement): boolean =>
  element.scrollHeight - element.scrollTop - element.clientHeight < 96;

export const isReusableAgentSession = (session: OptimizeSession): boolean =>
  session.status === "waiting_user" || session.status === "completed";

export const sessionDisplayName = (session: OptimizeSession | null | undefined): string =>
  nonEmptyText(session?.name) || "未命名会话";

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

export function kindLabel(kind: StreamItemKind): string {
  switch (kind) {
    case "user":
      return "用户";
    case "assistant":
      return "Agent";
    case "tool":
      return "工具";
    case "error":
      return "错误";
    default:
      return "行为";
  }
}

export function formatItemMeta(item: StreamItem): string | null {
  if (item.current != null && item.total != null) {
    return `${item.current}/${Math.max(1, item.total)}`;
  }
  if (typeof item.timestamp === "number") {
    return formatTime(item.timestamp);
  }
  if (typeof item.timestamp === "string") {
    const time = new Date(item.timestamp).getTime();
    return Number.isNaN(time) ? null : formatTime(time);
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

export function progressEventToStreamItem(event: ProgressTimelineItem): StreamItem {
  const title = progressEventTitle(event);
  return {
    id: `progress-${event.id}`,
    kind: event.status === "error" ? "error" : event.status === "success" ? "tool" : "agent",
    activity: "progress",
    title,
    content: event.message || progressStatusLabel(event.status),
    timestamp: event.timestamp,
    status: event.status,
    current: event.current,
    total: event.total,
  };
}

export function turnsToConversationItems(turns: OptimizeTurn[]): StreamItem[] {
  const items: StreamItem[] = [];
  for (const turn of turns) {
    items.push({
      id: `turn-${turn.id}-user`,
      kind: "user",
      content: turn.user_text,
      timestamp: turn.created_at,
      sessionId: turn.session_id,
    });

    const steps = turn.events.map((event) => turnEventToStreamItem(turn, event));
    const failed = turn.status === "failed" || turn.status === "cancelled";
    const finalReply = turn.final_reply?.trim();
    const content = finalReply || turn.error?.trim() || (turn.status === "running" ? "正在处理" : "");
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

function turnEventToStreamItem(turn: OptimizeTurn, event: OptimizeTurnEvent): StreamItem {
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
      title: "工具调用",
      content: event.public_text,
      detail: payload,
    };
  }
  if (event.kind === "tool_result") {
    return {
      ...base,
      kind: event.status === "failed" ? "error" : "tool",
      activity: "tool_result",
      title: "工具结果",
      content: event.public_text,
      detail: payload,
    };
  }
  if (event.kind === "error") {
    return {
      ...base,
      kind: "error",
      activity: "turn_error",
      title: "执行失败",
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

export function processStepBehavior(step: StreamItem): ProcessBehavior {
  if (step.kind === "error" || step.status === "error") {
    return { title: "执行失败", detail: compactBehaviorText(step.content) || "请查看错误信息" };
  }
  if (step.status === "cancelled") {
    return { title: "任务已停止", detail: compactBehaviorText(step.content) || "用户已停止本次任务" };
  }

  switch (step.activity) {
    case "llm_response":
      return llmBehavior(step.detail);
    case "tool_result":
      return toolBehavior(step.detail);
    case "progress":
      return progressBehavior(step);
    case "agent_event":
      return { title: "记录运行状态", detail: compactBehaviorText(step.content) || "" };
    case "assistant_update":
      return { title: "Agent 说明", detail: step.content };
    case "tool_call":
      return turnToolCallBehavior(step);
    case "turn_error":
      return { title: "执行失败", detail: compactBehaviorText(step.content) || "请查看错误信息" };
    default:
      break;
  }

  if (step.status === "running") return { title: "正在执行", detail: compactBehaviorText(step.content) || "" };
  if (step.status === "success") return { title: "执行完成", detail: compactBehaviorText(step.content) || "本步已完成" };
  return { title: "记录运行状态", detail: compactBehaviorText(step.content) || "" };
}

function turnToolCallBehavior(step: StreamItem): ProcessBehavior {
  const parsed = step.detail ? parseJsonObject(step.detail) : null;
  const tool = parsed ? stringField(parsed, "tool") : null;
  if (step.content.startsWith("调用工具：")) {
    return { title: step.content, detail: "" };
  }
  return { title: tool ? `调用工具：${tool}` : "调用工具", detail: "" };
}

function llmBehavior(detail?: string | null): ProcessBehavior {
  const parsed = detail ? parseJsonObject(detail) : null;
  const action = parsed && typeof parsed.action === "string" ? parsed.action : null;
  const publicUpdate = parsed ? stringField(parsed, "public_update") : null;
  const args = parsed && isRecord(parsed.arguments) ? parsed.arguments : null;

  switch (action) {
    case "think":
      return publicUpdate
        ? { title: "Agent 说明", detail: publicUpdate }
        : { title: "模型决策缺少阶段说明", detail: "think 决策没有返回 public_update" };
    case "run_tool":
      return toolCallBehavior(args, publicUpdate);
    case "final":
      return finalDecisionBehavior(args);
    default:
      return { title: "模型决策无法识别", detail: action ? `未知 action：${action}` : "缺少 action" };
  }
}

function toolCallBehavior(
  args: Record<string, unknown> | null,
  publicUpdate?: string | null,
): ProcessBehavior {
  const tool = args ? stringField(args, "tool") : null;
  const toolArgs = args && isRecord(args.arguments) ? args.arguments : null;
  const detail = publicUpdate || "run_tool 决策没有返回 public_update";
  if (tool === "get_project_information") {
    const mode = toolArgs ? stringField(toolArgs, "mode") : null;
    return {
      title: mode === "search" ? "调用工具：搜索资料" : "调用工具：读取资料",
      detail,
    };
  }
  if (tool === "edit_article") {
    return {
      title: "调用工具：修改正文",
      detail,
    };
  }
  return { title: "调用工具", detail: tool ? `${tool}：${detail}` : detail };
}

function finalDecisionBehavior(args: Record<string, unknown> | null): ProcessBehavior {
  const reply = args ? stringField(args, "reply") : null;
  if (reply) {
    return { title: "Agent 回复", detail: reply };
  }
  if (args && Array.isArray(args.issues)) {
    return { title: "分析结果", detail: formatIssueList(args.issues) };
  }
  return { title: "模型 final 参数无效", detail: "final.arguments 缺少 reply 或 issues" };
}

function toolBehavior(detail?: string | null): ProcessBehavior {
  const parsed = detail ? parseJsonObject(detail) : null;
  if (!parsed) return { title: "工具结果无法解析", detail: "" };
  const tool = typeof parsed.tool === "string" ? parsed.tool : null;
  const result = isRecord(parsed.result) ? parsed.result : parsed;
  const resultTool = typeof result.tool === "string" ? result.tool : tool;
  const success = typeof parsed.success === "boolean" ? parsed.success : null;
  if (success === false) {
    const error = stringField(parsed, "error") ?? stringField(result, "error") ?? "工具执行失败";
    return { title: `工具失败：${runtimeToolLabel(resultTool)}`, detail: error };
  }

  if (resultTool === "get_project_information") {
    const mode = stringField(result, "mode");
    const source = stringField(result, "source");
    const matches = Array.isArray(result.matches) ? result.matches.length : null;
    return {
      title: mode === "search" ? "工具返回：搜索结果" : "工具返回：读取结果",
      detail:
        matches != null
          ? `找到 ${matches} 条相关信息`
          : source
            ? `已获得${informationSourceLabel(source)}。`
            : "已获得所需上下文",
    };
  }
  if (resultTool === "apply_chapter_patches") {
    return { title: "工具返回：补丁已应用", detail: patchCountDetail("已写入", result) };
  }
  if (resultTool === "edit_article") {
    const mode = stringField(result, "mode");
    if (mode === "applied") return { title: "工具返回：正文已写回", detail: patchCountDetail("已应用", result) };
    if (mode === "pending_confirmation") {
      return { title: "工具返回：补丁待审阅", detail: patchCountDetail("已生成待审阅", result) };
    }
    if (mode === "no_change") return { title: "工具返回：无需修改", detail: "没有生成可应用补丁" };
    return { title: "工具返回：补丁提案", detail: patchCountDetail("生成", result) };
  }
  return { title: "工具返回", detail: resultTool || "已获得工具结果" };
}

function progressBehavior(step: StreamItem): ProcessBehavior {
  const text = compactBehaviorText(step.content);
  if (isGenericWaitingProgress(step)) {
    return {
      title: "正在思考",
      detail: "",
    };
  }
  if (text?.includes("读取优化会话上下文")) {
    return { title: "读取上下文", detail: "准备章节、会话和项目状态" };
  }
  if (text?.includes("渲染") || text?.includes("准备")) {
    return { title: "整理任务输入", detail: text };
  }
  if (step.status === "running") {
    return { title: text || "正在执行", detail: progressDetail(step) };
  }
  if (step.status === "success") {
    return { title: text || "执行完成", detail: progressDetail(step) };
  }
  return { title: text || "更新状态", detail: progressDetail(step) };
}

export function isGenericWaitingProgress(step: StreamItem): boolean {
  return step.activity === "progress" && /等待.*Agent.*(决策|回复)/.test(step.content);
}

function progressDetail(step: StreamItem): string {
  if (step.current != null && step.total != null) {
    return `进度 ${step.current}/${Math.max(1, step.total)}`;
  }
  return "";
}

function patchCountDetail(verb: string, payload: Record<string, unknown>): string {
  const count =
    numberField(payload, "patch_count") ??
    numberField(payload, "applied_count") ??
    (Array.isArray(payload.patches) ? payload.patches.length : null);
  return count == null ? `${verb}正文修改` : `${verb} ${count} 处修改`;
}

function informationSourceLabel(source: string): string {
  switch (source) {
    case "current_chapter":
      return "当前章节正文";
    case "current_chapter_outline":
      return "当前章节章纲";
    case "project_detail":
      return "项目设定";
    case "blueprint":
      return "故事蓝图";
    case "part_outlines":
      return "分卷大纲";
    case "chapter_outlines":
      return "章节大纲";
    case "character_profiles":
      return "角色资料";
    case "foreshadowing_index":
      return "伏笔索引";
    case "previous_chapters":
      return "前文章节";
    case "next_chapters":
      return "后文章节";
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

export function statusLabel(status: string): string {
  switch (status) {
    case "analyzing":
      return "分析中";
    case "waiting_user":
      return "等待指令";
    case "fixing":
      return "修复中";
    case "completed":
      return "已完成，可继续";
    case "cancelled":
      return "已取消";
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

function formatTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString("zh-CN", { hour12: false });
}

function progressEventTitle(event: ProgressTimelineItem): string {
  const message = event.message || "";
  if (event.status === "error") return "调用失败";
  if (event.status === "cancelled") return "调用停止";
  if (message.includes("调用模型") || message.includes("等待 Agent 回复")) return "LLM 调用";
  if (message.includes("模型已返回") || message.includes("Agent 已回复")) return "LLM 返回";
  if (message.includes("局部补丁")) return "局部补丁";
  if (message.includes("修复问题") || message.includes("工具")) return "工具调用";
  if (message.includes("已处理问题") || message.includes("修复完成")) return "工具结果";
  return progressStatusLabel(event.status);
}

function progressStatusLabel(status: ProgressDialogStatus): string {
  switch (status) {
    case "running":
      return "执行中";
    case "success":
      return "完成";
    case "error":
      return "失败";
    case "cancelled":
      return "已停止";
    default:
      return "等待";
  }
}

function runtimeToolLabel(tool: string | null): string {
  switch (tool) {
    case "get_project_information":
      return "获取项目信息";
    case "edit_article":
      return "修改正文";
    case "apply_chapter_patches":
      return "应用局部补丁";
    default:
      return tool || "工具";
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

function formatIssueList(rawIssues: unknown[]): string {
  const issues = parseOptimizeIssues(rawIssues);
  if (issues.length === 0) return "本次分析未发现明确问题。";
  return [
    `发现 ${issues.length} 个问题：`,
    ...issues.map((issue, index) => {
      const severity = issue.severity ? ` / ${issue.severity}` : "";
      return `${index + 1}. ${issue.description || "（无描述）"}（${issue.tool}${severity}，id: ${issue.id}）`;
    }),
  ].join("\n");
}

export function patchOperationLabel(operation: string): string {
  switch (operation) {
    case "replace":
      return "修改";
    case "delete":
      return "删除";
    case "insert_before":
      return "前插";
    case "insert_after":
      return "后插";
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
