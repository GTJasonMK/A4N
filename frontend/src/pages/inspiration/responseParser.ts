import type { MessageOption } from "./shared";
import { isJsonRecord as isRecord, type JsonRecord } from "../../utils/jsonRecord";

interface ParsedInspirationResponse {
  content: string;
  rawContent: string;
  options?: MessageOption[];
  isComplete: boolean;
  conversationState: Record<string, unknown>;
  progressSummary?: string;
  nextQuestion?: string;
  placeholder?: string;
}

interface StreamingInspirationPreview {
  content: string;
  options?: MessageOption[];
  optionsPending?: boolean;
}

const asString = (value: unknown): string | undefined => {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
};

const isStringOrNull = (value: unknown): value is string | null =>
  typeof value === "string" || value === null;

const asStringArray = (value: unknown): string[] => {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => (typeof item === "string" ? item.trim() : ""))
    .filter(Boolean);
};

const rejectUnknownKeys = (source: JsonRecord, allowed: string[], context: string) => {
  for (const key of Object.keys(source)) {
    if (!allowed.includes(key)) {
      throw new Error(`${context} 包含未支持字段 ${key}`);
    }
  }
};

const requireKey = (source: JsonRecord, key: string, context: string): unknown => {
  if (!Object.prototype.hasOwnProperty.call(source, key)) {
    throw new Error(`${context} 缺少字段 ${key}`);
  }
  return source[key];
};

const parseResponseObject = (raw: string): JsonRecord => {
  const trimmed = raw.trim();
  if (!trimmed.startsWith("{") || !trimmed.endsWith("}")) {
    throw new Error("灵感回复必须是完整 JSON 对象");
  }

  const parsed = JSON.parse(trimmed) as unknown;
  if (!isRecord(parsed)) {
    throw new Error("灵感回复必须是 JSON 对象");
  }

  rejectUnknownKeys(
    parsed,
    [
      "ai_message",
      "next_question",
      "progress_summary",
      "ui_control",
      "conversation_state",
      "is_complete",
    ],
    "灵感回复",
  );
  return parsed;
};

const normalizeStreamingOption = (raw: unknown): MessageOption | null => {
  if (!isRecord(raw)) return null;

  const label = asString(raw.label);
  if (!label) return null;

  const description = asString(raw.description);
  const keyElements = asStringArray(raw.key_elements);

  const valueParts = [`选择：${label}`];
  if (description) valueParts.push(description);
  if (keyElements.length > 0) valueParts.push(`关键要素：${keyElements.join("、")}`);

  return {
    label,
    description,
    keyElements: keyElements.length > 0 ? keyElements : undefined,
    value: valueParts.join("\n"),
  };
};

const parseOption = (raw: unknown, index: number): MessageOption => {
  if (!isRecord(raw)) {
    throw new Error(`ui_control.options[${index}] 必须是 JSON 对象`);
  }
  rejectUnknownKeys(
    raw,
    ["id", "label", "description", "key_elements"],
    `ui_control.options[${index}]`,
  );

  const optionId = asString(raw.id);
  const label = asString(raw.label);
  const description = asString(raw.description);
  if (!optionId) throw new Error(`ui_control.options[${index}].id 不能为空`);
  if (!Array.isArray(raw.key_elements)) {
    throw new Error(`ui_control.options[${index}].key_elements 必须是数组`);
  }
  if (!label) throw new Error(`ui_control.options[${index}].label 不能为空`);
  if (!description) throw new Error(`ui_control.options[${index}].description 不能为空`);

  const keyElements = raw.key_elements.map((item, itemIndex) => {
    if (typeof item !== "string" || !item.trim()) {
      throw new Error(`ui_control.options[${index}].key_elements[${itemIndex}] 必须是非空字符串`);
    }
    return item.trim();
  });
  if (keyElements.length < 2 || keyElements.length > 3) {
    throw new Error(`ui_control.options[${index}].key_elements 必须包含 2-3 个元素`);
  }
  const valueParts = [`选择：${label}`];
  valueParts.push(description);
  valueParts.push(`关键要素：${keyElements.join("、")}`);

  return {
    label,
    description,
    keyElements,
    value: valueParts.join("\n"),
  };
};

const extractOptions = (uiControl: JsonRecord): MessageOption[] | undefined => {
  rejectUnknownKeys(uiControl, ["type", "options", "placeholder"], "ui_control");
  const type = asString(uiControl.type);
  if (
    !type ||
    !["single_choice", "text_input", "info_display", "inspired_options"].includes(type)
  ) {
    throw new Error("ui_control.type 无效");
  }

  const rawOptions = Array.isArray(uiControl.options) ? uiControl.options : undefined;
  if (uiControl.options !== undefined && !Array.isArray(uiControl.options)) {
    throw new Error("ui_control.options 必须是数组");
  }
  if (type === "inspired_options" && (!rawOptions || rawOptions.length === 0)) {
    throw new Error("ui_control.options 不能为空");
  }
  if (!rawOptions) return undefined;

  const options = rawOptions.map((item, index) => parseOption(item, index));

  return options.length > 0 ? options : undefined;
};

const normalizeTextBlock = (value: string): string =>
  value
    .replace(/\\n/g, "\n")
    .split("\n")
    .map((line) => line.trimEnd())
    .join("\n")
    .trim();

/** 从不完整的流式 JSON 中提取 ai_message 已输出的文本片段 */
const extractPartialAiMessage = (raw: string): string | null => {
  // 定位 "ai_message" 键值对的值起始位置
  const keyMatch = raw.match(/"ai_message"\s*:\s*"/);
  if (!keyMatch) return null;

  const startIdx = keyMatch.index! + keyMatch[0].length;
  let result = "";
  let i = startIdx;
  let escaped = false;

  while (i < raw.length) {
    const ch = raw[i];
    if (escaped) {
      switch (ch) {
        case "n": result += "\n"; break;
        case "t": result += "\t"; break;
        case "r": result += "\r"; break;
        case "\\": result += "\\"; break;
        case '"': result += '"'; break;
        default: result += ch; break;
      }
      escaped = false;
    } else if (ch === "\\") {
      escaped = true;
    } else if (ch === '"') {
      // 已闭合的字符串值——完整提取
      return result.trim() ? result : null;
    } else {
      result += ch;
    }
    i++;
  }

  // 到达末尾未闭合——返回已提取的部分
  return result.trim() ? result : null;
};

/** 从字符位置开始提取平衡的 JSON 对象或数组，失败返回 null */
const extractBalancedJson = (
  raw: string,
  startIdx: number,
): { json: string; endIdx: number } | null => {
  let depth = 0;
  let i = startIdx;
  let inString = false;
  let escaped = false;

  while (i < raw.length) {
    const ch = raw[i];
    if (escaped) {
      escaped = false;
    } else if (ch === "\\" && inString) {
      escaped = true;
    } else if (ch === '"') {
      inString = !inString;
    } else if (!inString) {
      if (ch === "{" || ch === "[") depth++;
      else if (ch === "}" || ch === "]") {
        depth--;
        if (depth === 0) {
          return { json: raw.slice(startIdx, i + 1), endIdx: i + 1 };
        }
      }
    }
    i++;
  }

  return null;
};

/** 从流式 JSON 中提取已完成的选项对象和正在生成的状态 */
const extractPartialOptions = (
  raw: string,
): { ready: MessageOption[]; pending: boolean } | null => {
  const match = raw.match(/"options"\s*:\s*\[/);
  if (!match) return null;

  const startIdx = match.index! + match[0].length;
  const ready: MessageOption[] = [];
  let pending = true;
  let i = startIdx;

  while (i < raw.length) {
    // 跳过空白
    while (i < raw.length && /\s/.test(raw[i])) i++;
    if (i >= raw.length) break;

    if (raw[i] === "]") {
      pending = false;
      break;
    }
    if (raw[i] === ",") {
      i++;
      continue;
    }
    if (raw[i] === "{") {
      const balanced = extractBalancedJson(raw, i);
      if (balanced) {
        try {
          const parsed = JSON.parse(balanced.json);
          const opt = normalizeStreamingOption(parsed);
          if (opt) ready.push(opt);
        } catch {
          // 格式损坏则跳过
        }
        i = balanced.endIdx;
      } else {
        break;
      }
    } else {
      i++;
    }
  }

  return { ready, pending };
};

export const parseInspirationResponse = (raw: string): ParsedInspirationResponse => {
  const root = parseResponseObject(raw);
  const message = asString(root.ai_message);
  if (!message) {
    throw new Error("灵感回复缺少 ai_message");
  }

  const nextQuestionValue = requireKey(root, "next_question", "灵感回复");
  const progressSummaryValue = requireKey(root, "progress_summary", "灵感回复");
  if (!isStringOrNull(nextQuestionValue)) {
    throw new Error("next_question 必须是字符串或 null");
  }
  if (!isStringOrNull(progressSummaryValue)) {
    throw new Error("progress_summary 必须是字符串或 null");
  }
  const nextQuestion = nextQuestionValue?.trim() || undefined;
  const progressSummary = progressSummaryValue?.trim() || undefined;

  const isCompleteValue = requireKey(root, "is_complete", "灵感回复");
  if (typeof isCompleteValue !== "boolean") {
    throw new Error("is_complete 必须是布尔值");
  }
  const isComplete = isCompleteValue;
  const conversationStateValue = requireKey(root, "conversation_state", "灵感回复");
  if (!isRecord(conversationStateValue)) {
    throw new Error("conversation_state 必须是 JSON 对象");
  }
  const conversationState = conversationStateValue;
  if (isComplete) {
    const chapterCount = conversationState.chapter_count;
    if (
      typeof chapterCount !== "number"
      || !Number.isInteger(chapterCount)
      || chapterCount < 5
      || chapterCount > 10000
    ) {
      throw new Error("完成态 conversation_state.chapter_count 必须是 5-10000 的整数");
    }
    if (!asString(conversationState.genre)) {
      throw new Error("完成态 conversation_state.genre 必须是非空字符串");
    }
  }
  const uiControlValue = requireKey(root, "ui_control", "灵感回复");
  if (!isRecord(uiControlValue)) {
    throw new Error("ui_control 必须是 JSON 对象");
  }
  const uiControl = uiControlValue;

  return {
    content: normalizeTextBlock(message),
    rawContent: raw,
    options: extractOptions(uiControl),
    isComplete,
    conversationState,
    progressSummary,
    nextQuestion,
    placeholder: asString(uiControl.placeholder),
  };
};

export const getStreamingResponsePreview = (raw: string): StreamingInspirationPreview => {
  const trimmed = raw.trimStart();
  if (!trimmed) return { content: "正在接收回复..." };

  const partial = extractPartialAiMessage(trimmed);
  const optResult = extractPartialOptions(trimmed);
  if (partial) {
    return {
      content: normalizeTextBlock(partial),
      options: optResult?.ready.length ? optResult.ready : undefined,
      optionsPending: optResult?.pending ?? false,
    };
  }

  if (
    trimmed.startsWith("{") ||
    /"(?:ai_message|ui_control|conversation_state|is_complete)"\s*:/.test(trimmed)
  ) {
    return {
      content: "正在接收结构化回复...",
      options: optResult?.ready.length ? optResult.ready : undefined,
      optionsPending: optResult?.pending ?? false,
    };
  }

  return { content: trimmed };
};
