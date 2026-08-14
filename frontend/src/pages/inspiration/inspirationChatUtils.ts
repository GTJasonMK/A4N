import type { ProgressDialogStatus } from "../../components/feedback/ProgressDialog";
import type { InspirationMessageSummary } from "../../api/generated/host";
import { parseInspirationResponse } from "./responseParser";
import type { Message } from "./shared";

export type Stage = "conversation" | "confirm" | "display";

export const DEFAULT_INPUT_PLACEHOLDER =
  "说说你的想法…  Enter 发送 / Shift+Enter 换行";
const BLUEPRINT_PROGRESS_TOTAL = 12;

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

export const msgId = (() => {
  let n = 0;
  return () => `m-${Date.now()}-${++n}`;
})();

export const welcomeMessage = (title: string): Message => ({
  id: msgId(),
  role: "assistant",
  content: `你好！我们来一起打磨「${title}」的故事。\n\n先告诉我：你最初的灵感是什么？可以是一个画面、一句台词、一个人物，或者一个让你着迷的"假设"。`,
  createdAt: Date.now(),
});

const parseHistoryTime = (value: string, messageId: number): number => {
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) {
    throw new Error(`灵感历史消息 ${messageId} 的 created_at 无效`);
  }
  return parsed;
};

export const normalizeHistoryMessage = (
  record: InspirationMessageSummary,
): Message => {
  const role =
    record.role === "user" ||
    record.role === "assistant"
      ? record.role
      : null;
  if (!role) throw new Error(`灵感历史消息 ${record.id} 的 role 无效`);
  const content = record.content.trim();
  if (!content) throw new Error(`灵感历史消息 ${record.id} 的 content 为空`);

  const base: Message = {
    id: `history-${record.id}`,
    role,
    content,
    rawContent: content,
    createdAt: parseHistoryTime(record.created_at, record.id),
  };

  if (role !== "assistant") return base;

  const parsed = parseInspirationResponse(content);

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

const firstString = (...values: unknown[]): string | undefined => {
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
