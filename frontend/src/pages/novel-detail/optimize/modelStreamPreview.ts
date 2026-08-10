import {
  parseJsonObject,
  parseOptimizeIssues,
  patchOperationLabel,
  stringField,
  type StreamItemKind,
} from "./optimizeTabUtils";
import { isJsonRecord as isRecord, type JsonRecord } from "../../../utils/jsonRecord";

interface ModelStreamPreview {
  kind: StreamItemKind;
  title: string;
  content: string;
}

export function previewModelStream(raw: string): ModelStreamPreview {
  const parsed = parseJsonObject(raw);
  if (parsed) return previewParsedOutput(parsed);
  return previewPartialOutput(raw);
}

function previewParsedOutput(parsed: JsonRecord): ModelStreamPreview {
  const action = stringField(parsed, "action");
  if (action) {
    const publicUpdate = stringField(parsed, "public_update");
    const args = recordField(parsed, "arguments");
    return previewAgentDecision(action, args, publicUpdate);
  }

  if (Array.isArray(parsed.issues)) {
    return previewIssues(parsed.issues);
  }

  if (Array.isArray(parsed.patches)) {
    return previewPatches(parsed);
  }

  const reply = stringField(parsed, "reply");
  if (reply) {
    return {
      kind: "assistant",
      title: "Agent 正在回复",
      content: reply,
    };
  }

  const summary = stringField(parsed, "summary");
  if (summary) {
    return {
      kind: "agent",
      title: "正在整理结果",
      content: summary,
    };
  }

  return protocolError("模型输出不符合协议", "缺少可展示的 reply、issues、patches 或 Agent action");
}

function previewAgentDecision(
  action: string,
  args: JsonRecord | null,
  publicUpdate: string | null,
): ModelStreamPreview {
  switch (action) {
    case "think":
      return previewThinking(publicUpdate);
    case "run_tool":
      return previewToolCall(args, publicUpdate);
    case "final":
      return previewFinal(args);
    default:
      return protocolError("模型返回非法 action", action);
  }
}

function previewThinking(publicUpdate: string | null): ModelStreamPreview {
  if (publicUpdate) {
    return {
      kind: "agent",
      title: "Agent 说明",
      content: publicUpdate,
    };
  }
  return protocolError("模型决策缺少阶段说明", "think 决策没有返回 public_update");
}

function previewToolCall(
  args: JsonRecord | null,
  publicUpdate: string | null,
): ModelStreamPreview {
  const tool = args ? stringField(args, "tool") : null;
  const toolArgs = args ? recordField(args, "arguments") : null;
  if (!publicUpdate) {
    return protocolError("模型决策缺少阶段说明", "run_tool 决策没有返回 public_update");
  }
  if (tool === "get_project_information") {
    const mode = toolArgs ? stringField(toolArgs, "mode") : null;
    return {
      kind: "agent",
      title: mode === "search" ? "调用工具：搜索资料" : "调用工具：读取资料",
      content: publicUpdate,
    };
  }

  if (tool === "edit_article") {
    return {
      kind: "agent",
      title: "调用工具：修改正文",
      content: publicUpdate,
    };
  }

  return {
    kind: "agent",
    title: "正在调用工具",
    content: publicUpdate,
  };
}

function previewFinal(args: JsonRecord | null): ModelStreamPreview {
  const reply = args ? stringField(args, "reply") : null;
  if (reply) {
    return {
      kind: "assistant",
      title: "Agent 正在回复",
      content: reply,
    };
  }

  const issues = args && Array.isArray(args.issues) ? args.issues : null;
  if (issues) return previewIssues(issues);

  return protocolError("模型 final 参数无效", "final.arguments 缺少 reply 或 issues");
}

function previewIssues(rawIssues: unknown[]): ModelStreamPreview {
  const issues = parseOptimizeIssues(rawIssues);
  return {
    kind: "assistant",
    title: "正在整理分析结果",
    content: lines([
      issues.length === 0
        ? "本次分析未发现明确问题。"
        : `已发现 ${issues.length} 个问题：`,
      ...issues.map((issue, index) => {
        const severity = issue.severity ? ` / ${issue.severity}` : "";
        return `${index + 1}. ${issue.description || "未填写问题描述"}${severity}`;
      }),
    ]),
  };
}

function previewPatches(parsed: JsonRecord): ModelStreamPreview {
  const summary = stringField(parsed, "summary");
  const patches = Array.isArray(parsed.patches) ? parsed.patches : [];
  return {
    kind: "agent",
    title: "正在生成正文补丁",
    content: lines([
      summary ? `摘要：${summary}` : null,
      patches.length > 0 ? `已生成 ${patches.length} 处局部修改` : "没有生成可安全定位的修改",
      formatPatchReasons(patches),
    ]),
  };
}

function previewPartialOutput(raw: string): ModelStreamPreview {
  const reply = extractStringField(raw, "reply");
  if (reply) {
    return {
      kind: "assistant",
      title: "Agent 正在回复",
      content: reply,
    };
  }

  const action = extractStringField(raw, "action");
  const publicUpdate = extractStringField(raw, "public_update");
  if (action === "think") return previewPartialThinking(publicUpdate);
  if (action === "run_tool") return previewPartialToolCall(raw, publicUpdate);
  if (action === "final") return previewPartialFinal(raw);

  const summary = extractStringField(raw, "summary");
  if (summary || hasField(raw, "patches")) {
    return {
      kind: "agent",
      title: "正在生成正文补丁",
      content: lines([
        summary ? `摘要：${summary}` : null,
        partialPatchCount(raw),
        formatPartialPatchReasons(raw),
      ]),
    };
  }

  if (hasField(raw, "thought")) {
    return thinkingPreview();
  }

  return {
    kind: "agent",
    title: "模型正在输出",
    content: "",
  };
}

function previewPartialThinking(publicUpdate: string | null): ModelStreamPreview {
  if (publicUpdate) {
    return {
      kind: "agent",
      title: "Agent 说明",
      content: publicUpdate,
    };
  }
  return thinkingPreview();
}

function previewPartialToolCall(raw: string, publicUpdate: string | null): ModelStreamPreview {
  const tool = extractStringField(raw, "tool");
  const content = publicUpdate ?? "";
  if (tool === "get_project_information") {
    const mode = extractStringField(raw, "mode");
    return {
      kind: "agent",
      title: mode === "search" ? "调用工具：搜索资料" : "调用工具：读取资料",
      content,
    };
  }

  if (tool === "edit_article") {
    return {
      kind: "agent",
      title: "调用工具：修改正文",
      content,
    };
  }

  return {
    kind: "agent",
    title: "正在选择工具",
    content,
  };
}

function thinkingPreview(): ModelStreamPreview {
  return {
    kind: "agent",
    title: "正在思考",
    content: "",
  };
}

function previewPartialFinal(raw: string): ModelStreamPreview {
  if (hasField(raw, "issues")) {
    const descriptions = extractStringFields(raw, "description");
    return {
      kind: "assistant",
      title: "正在整理分析结果",
      content: lines([
        descriptions.length > 0 ? "已识别的问题：" : "正在整理问题列表",
        ...descriptions.map((description, index) => `${index + 1}. ${description}`),
      ]),
    };
  }

  return {
    kind: "agent",
    title: "正在整理回复",
    content: "",
  };
}

function formatPatchReasons(patches: unknown[]): string | null {
  const linesOut = patches
    .map((patch, index) => {
      if (!isRecord(patch)) return null;
      const reason = stringField(patch, "reason");
      const operation = stringField(patch, "operation");
      if (!reason && !operation) return null;
      return `${index + 1}. ${operation ? `${patchOperationLabel(operation)}：` : ""}${
        reason || "局部修改"
      }`;
    })
    .filter((line): line is string => line != null);
  return linesOut.length > 0 ? linesOut.join("\n") : null;
}

function formatPartialPatchReasons(raw: string): string | null {
  const reasons = extractStringFields(raw, "reason");
  if (reasons.length === 0) return null;
  return reasons.map((reason, index) => `${index + 1}. ${reason}`).join("\n");
}

function partialPatchCount(raw: string): string | null {
  const count = countField(raw, "patch_id");
  return count > 0 ? `已起草 ${count} 处局部修改` : null;
}

function lines(items: Array<string | null | undefined>): string {
  const content = items.filter((item): item is string => Boolean(item?.trim())).join("\n");
  return content;
}

function protocolError(title: string, content: string): ModelStreamPreview {
  return {
    kind: "error",
    title,
    content,
  };
}

function recordField(record: JsonRecord, key: string): JsonRecord | null {
  const value = record[key];
  return isRecord(value) ? value : null;
}

function hasField(raw: string, field: string): boolean {
  return raw.includes(`"${field}"`);
}

function countField(raw: string, field: string): number {
  const needle = `"${field}"`;
  let count = 0;
  let offset = 0;
  while (offset < raw.length) {
    const index = raw.indexOf(needle, offset);
    if (index < 0) break;
    count += 1;
    offset = index + needle.length;
  }
  return count;
}

function extractStringField(raw: string, field: string): string | null {
  const values = extractStringFields(raw, field);
  return values.length > 0 ? values[values.length - 1] : null;
}

function extractStringFields(raw: string, field: string): string[] {
  const needle = `"${field}"`;
  const values: string[] = [];
  let offset = 0;
  while (offset < raw.length) {
    const keyIndex = raw.indexOf(needle, offset);
    if (keyIndex < 0) break;
    const colonIndex = raw.indexOf(":", keyIndex + needle.length);
    if (colonIndex < 0) break;
    const valueStart = nextNonWhitespace(raw, colonIndex + 1);
    if (raw[valueStart] === '"') {
      values.push(decodeJsonStringPrefix(raw, valueStart + 1));
    }
    offset = keyIndex + needle.length;
  }
  return values.map((value) => value.trim()).filter(Boolean);
}

function nextNonWhitespace(raw: string, start: number): number {
  let index = start;
  while (index < raw.length && /\s/.test(raw[index])) index += 1;
  return index;
}

function decodeJsonStringPrefix(raw: string, start: number): string {
  let output = "";
  for (let index = start; index < raw.length; index += 1) {
    const char = raw[index];
    if (char === '"') break;
    if (char !== "\\") {
      output += char;
      continue;
    }

    const escaped = raw[index + 1];
    if (!escaped) break;
    index += 1;
    switch (escaped) {
      case "n":
        output += "\n";
        break;
      case "r":
        output += "\r";
        break;
      case "t":
        output += "\t";
        break;
      case '"':
      case "\\":
      case "/":
        output += escaped;
        break;
      case "u": {
        const hex = raw.slice(index + 1, index + 5);
        if (!/^[0-9a-fA-F]{4}$/.test(hex)) return output;
        output += String.fromCharCode(Number.parseInt(hex, 16));
        index += 4;
        break;
      }
      default:
        output += escaped;
    }
  }
  return output;
}
