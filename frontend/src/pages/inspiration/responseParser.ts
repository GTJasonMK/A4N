import type { MessageOption } from "./shared";
import type { InspirationOption, InspirationResponse } from "../../api/generated/generationContracts";
import { isJsonRecord as isRecord } from "../../utils/jsonRecord";

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

const asStringArray = (value: unknown): string[] => {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => (typeof item === "string" ? item.trim() : ""))
    .filter(Boolean);
};

const parseResponseObject = (raw: string): InspirationResponse => {
  const trimmed = raw.trim();
  if (!trimmed.startsWith("{") || !trimmed.endsWith("}")) {
    throw new Error("灵感回复必须是完整 JSON 对象");
  }

  const parsed = JSON.parse(trimmed) as unknown;
  if (!isRecord(parsed)) {
    throw new Error("灵感回复必须是 JSON 对象");
  }

  // Final responses and history share the server's inspiration validator.
  // Streaming fragments use the separate preview projection below.
  return parsed as unknown as InspirationResponse;
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

const parseOption = (raw: InspirationOption): MessageOption => {
  const label = raw.label.trim();
  const description = raw.description.trim();
  const keyElements = raw.key_elements.map((item) => item.trim());
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

const extractOptions = (uiControl: InspirationResponse["ui_control"]): MessageOption[] | undefined => {
  return uiControl.options?.length ? uiControl.options.map(parseOption) : undefined;
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

  return {
    content: normalizeTextBlock(root.ai_message),
    rawContent: raw,
    options: extractOptions(root.ui_control),
    isComplete: root.is_complete,
    conversationState: root.conversation_state,
    progressSummary: asString(root.progress_summary),
    nextQuestion: asString(root.next_question),
    placeholder: asString(root.ui_control.placeholder),
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
