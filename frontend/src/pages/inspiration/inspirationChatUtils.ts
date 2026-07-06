import type { ProgressDialogStatus } from "../../components/feedback/ProgressDialog";
import type { InspirationMessage } from "../../api/inspiration";
import {
  clearBlueprintGenerationPending,
  hasRecentBlueprintGenerationPending,
  markBlueprintGenerationPending,
} from "../../utils/blueprintPending";
import {
  readPortableItem,
  writePortableItem,
} from "../../utils/portableStorage";
import { isJsonRecord as isRecord } from "../../utils/jsonRecord";
import { parseInspirationResponse } from "./responseParser";
import type { Message } from "./shared";

export type Stage = "conversation" | "confirm" | "display";

export type WorkflowEvent =
  | { type: "Chunk"; data: { content: string; index: number } }
  | {
      type: "Progress";
      data: {
        current: number;
        total: number;
        message: string;
        percent?: number;
      };
    }
  | { type: "Complete"; data: { summary?: string | null } }
  | {
      type: "Error";
      data: { code: string; message: string; recoverable: boolean };
    }
  | { type: string; data?: unknown };

export const DEFAULT_INPUT_PLACEHOLDER =
  "说说你的想法…  Enter 发送 / Shift+Enter 换行";
const CHAT_CACHE_TTL_MS = 10 * 60 * 1000;
const BLUEPRINT_PROGRESS_TOTAL = 12;

interface InspirationChatCache {
  updatedAt: number;
  messages: Message[];
  inputPlaceholder?: string;
}

export interface BlueprintProgressState {
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  error: string | null;
}

export const createBlueprintProgressState = (): BlueprintProgressState => ({
  status: "idle",
  current: 0,
  total: BLUEPRINT_PROGRESS_TOTAL,
  message: "准备生成首版蓝图…",
  error: null,
});

const chatCacheKey = (projectId: number) =>
  `a4n:inspiration:${projectId}:bootstrap:v1`;

export const msgId = (() => {
  let n = 0;
  return () => `m-${Date.now()}-${++n}`;
})();

export const taskId = (projectId: number) =>
  `inspiration-${projectId}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

export const readChatCache = (
  projectId: number,
): InspirationChatCache | null => {
  const raw = readPortableItem(chatCacheKey(projectId));
  if (!raw) return null;
  const cached = JSON.parse(raw) as Partial<InspirationChatCache> | null;
  if (
    !cached ||
    !Array.isArray(cached.messages) ||
    typeof cached.updatedAt !== "number" ||
    !Number.isFinite(cached.updatedAt)
  ) {
    throw new Error("灵感对话缓存格式无效");
  }
  if (
    cached.inputPlaceholder !== undefined &&
    typeof cached.inputPlaceholder !== "string"
  ) {
    throw new Error("灵感对话缓存格式无效");
  }
  if (Date.now() - cached.updatedAt > CHAT_CACHE_TTL_MS) {
    return null;
  }
  return {
    updatedAt: cached.updatedAt,
    messages: cached.messages,
    inputPlaceholder: cached.inputPlaceholder,
  };
};

export const writeChatCache = (
  projectId: number,
  messages: Message[],
  inputPlaceholder: string,
) => {
  if (messages.length === 0) return;
  writePortableItem(
    chatCacheKey(projectId),
    JSON.stringify({
      updatedAt: Date.now(),
      messages,
      inputPlaceholder,
    } satisfies InspirationChatCache),
  );
};

export const markBlueprintPending = (projectId: number) => {
  markBlueprintGenerationPending("novel", projectId);
};

export const clearBlueprintPending = (projectId: number) => {
  clearBlueprintGenerationPending("novel", projectId);
};

export const readBlueprintPending = (projectId: number): boolean =>
  hasRecentBlueprintGenerationPending("novel", projectId);

export const welcomeMessage = (title: string): Message => ({
  id: msgId(),
  role: "assistant",
  content: `你好！我们来一起打磨「${title}」的故事。\n\n先告诉我：你最初的灵感是什么？可以是一个画面、一句台词、一个人物，或者一个让你着迷的"假设"。`,
  createdAt: Date.now(),
});

const parseHistoryTime = (value: string | null): number => {
  if (!value) return Date.now();
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : Date.now();
};

export const normalizeHistoryMessage = (
  record: InspirationMessage,
): Message | null => {
  const role =
    record.role === "user" ||
    record.role === "assistant" ||
    record.role === "system"
      ? record.role
      : null;
  if (!role) return null;
  const content = record.content?.trim();
  if (!content) return null;

  const base: Message = {
    id: `history-${record.id}`,
    role,
    content,
    rawContent: content,
    createdAt: parseHistoryTime(record.created_at),
  };

  if (role !== "assistant") return base;

  let parsed: ReturnType<typeof parseInspirationResponse>;
  try {
    parsed = parseInspirationResponse(content);
  } catch {
    return null;
  }

  return {
    ...base,
    content: parsed.content,
    rawContent: parsed.rawContent,
    options: parsed.options,
    isComplete: parsed.isComplete,
    conversationState: parsed.conversationState,
    progressSummary: parsed.progressSummary,
    nextQuestion: parsed.nextQuestion,
  };
};

export const firstString = (...values: unknown[]): string | undefined => {
  for (const value of values) {
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  return undefined;
};

const messageSourceText = (message: Message): string | null =>
  firstString(message.rawContent, message.content) ?? null;

export const buildInspirationTranscript = (messages: Message[]): string => {
  const rows: string[] = [];

  for (const message of messages) {
    if (message.role === "system") continue;
    const text = messageSourceText(message);
    if (!text) {
      throw new Error("灵感对话包含空消息，无法生成蓝图");
    }
    rows.push(`${message.role === "user" ? "作者" : "顾问"}: ${text}`);
  }

  if (rows.length === 0) {
    throw new Error("灵感对话缺少可生成蓝图的消息");
  }

  return rows.join("\n\n");
};

export const splitSummaryLines = (value?: string): string[] => {
  if (!value) return [];
  return value
    .replace(/\\n/g, "\n")
    .split(/\n|[;；]/)
    .map((line) => line.replace(/^[-*•\d.、\s]+/, "").trim())
    .filter(Boolean);
};

const nonNegativeInteger = (value: unknown): number | null => {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0)
    return null;
  return value;
};

const positiveInteger = (value: unknown): number | null => {
  if (typeof value !== "number" || !Number.isInteger(value) || value <= 0)
    return null;
  return value;
};

export const parseBlueprintProgressEvent = (data: unknown) => {
  if (!isRecord(data)) {
    throw new Error("蓝图进度事件格式无效");
  }

  const current = nonNegativeInteger(data.current);
  const total = positiveInteger(data.total);
  const message = firstString(data.message);
  if (current === null) {
    throw new Error("蓝图进度事件缺少 current");
  }
  if (total === null) {
    throw new Error("蓝图进度事件缺少 total");
  }
  if (!message) {
    throw new Error("蓝图进度事件缺少 message");
  }

  return { current, total, message };
};

export const eventMessageOrNull = (data: unknown): string | null => {
  if (!isRecord(data)) return null;
  return firstString(data.message, data.summary) ?? null;
};

export const eventMessage = (data: unknown, missingMessage: string): string =>
  eventMessageOrNull(data) ?? missingMessage;

const toChapterCount = (value: unknown): number | null => {
  if (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value >= 5 &&
    value <= 10000
  ) {
    return value;
  }
  if (typeof value === "string") {
    const parsed = Number.parseInt(value.trim(), 10);
    if (Number.isInteger(parsed) && parsed >= 5 && parsed <= 10000)
      return parsed;
  }
  return null;
};

export const findChapterCountInValue = (
  value: unknown,
  depth = 0,
): number | null => {
  if (depth > 4) return null;
  const direct = toChapterCount(value);
  if (direct) return direct;
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = findChapterCountInValue(item, depth + 1);
      if (found) return found;
    }
    return null;
  }
  if (!isRecord(value)) return null;

  for (const key of [
    "chapter_count",
    "chapterCount",
    "total_chapters",
    "target_chapters",
  ]) {
    const found = toChapterCount(value[key]);
    if (found) return found;
  }
  for (const item of Object.values(value)) {
    const found = findChapterCountInValue(item, depth + 1);
    if (found) return found;
  }
  return null;
};

export const extractChapterCountFromText = (value: string): number | null => {
  const patterns = [
    /(?:设置|设定|计划|章节数|篇幅|预计|预期|总共|共)[\s:：]*?(\d{1,5})\s*(?:章|章节|回|$)/g,
    /(?:写|创作|生成|做成|安排)\s*(\d{1,5})\s*(?:章|章节|回)/g,
    /(\d{1,5})\s*(?:章|章节|回)\s*(?:左右|上下|以内|以上|的故事|小说)?/g,
  ];
  for (const pattern of patterns) {
    const matches = Array.from(value.matchAll(pattern));
    for (let i = matches.length - 1; i >= 0; i -= 1) {
      const found = toChapterCount(matches[i][1]);
      if (found) return found;
    }
  }
  return null;
};

export const inferChapterCountFromMessages = (
  messages: Message[],
): number | null => {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const m = messages[i];
    const fromState = findChapterCountInValue(m.conversationState);
    if (fromState) return fromState;
    const text = messageSourceText(m);
    if (!text) continue;
    const fromText = extractChapterCountFromText(text);
    if (fromText) return fromText;
  }

  return null;
};
