import type { ProgressDialogStatus } from "../../components/feedback/ProgressDialog";
import type { InspirationMessageSummary } from "../../api/generated/host";
import { parseInspirationResponse } from "./responseParser";
import type { Message } from "./shared";
import type { MessageTranslator } from "../../i18n";

export type Stage = "conversation" | "confirm" | "display";

export interface BlueprintProgressState {
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  error: string | null;
}

export const createBlueprintProgressState = (initialMessage: string): BlueprintProgressState => ({
  status: "idle",
  current: 0,
  // The backend is the only source of the workflow's total; it is learned
  // from the first progress event instead of duplicated in the page.
  total: 0,
  message: initialMessage,
  error: null,
});

export const msgId = (() => {
  let n = 0;
  return () => `m-${Date.now()}-${++n}`;
})();

export const welcomeMessage = (title: string, t: MessageTranslator): Message => ({
  id: msgId(),
  role: "assistant",
  content: t("inspiration.welcome", { title }),
  createdAt: Date.now(),
});

export const generationFailureMessage = (id: string, message: string): Message => ({
  id,
  role: "system",
  content: `❌ ${message}`,
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

export const buildInspirationTranscript = (messages: Message[]): string => {
  const rows: string[] = [];

  for (const message of messages) {
    if (message.role === "system") continue;
    const text = (message.rawContent ?? message.content).trim();
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
