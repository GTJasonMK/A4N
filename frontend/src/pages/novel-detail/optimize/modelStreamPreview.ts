import {
  parseJsonObject,
  parseOptimizeIssues,
  patchOperationLabel,
  stringField,
  type StreamItemKind,
} from "./optimizeTabUtils";
import { isJsonRecord as isRecord, type JsonRecord } from "../../../utils/jsonRecord";
import type { MessageTranslator } from "../../../i18n";

interface ModelStreamPreview {
  kind: StreamItemKind;
  title: string;
  content: string;
}

export function previewModelStream(raw: string, t: MessageTranslator): ModelStreamPreview {
  const parsed = parseJsonObject(raw);
  if (parsed) return previewParsedOutput(parsed, t);
  return previewPartialOutput(raw, t);
}

function previewParsedOutput(parsed: JsonRecord, t: MessageTranslator): ModelStreamPreview {
  const action = stringField(parsed, "action");
  if (action) {
    const publicUpdate = stringField(parsed, "public_update");
    const args = recordField(parsed, "arguments");
    return previewAgentDecision(action, args, publicUpdate, t);
  }

  if (Array.isArray(parsed.issues)) {
    return previewIssues(parsed.issues, t);
  }

  if (Array.isArray(parsed.patches)) {
    return previewPatches(parsed, t);
  }

  const reply = stringField(parsed, "reply");
  if (reply) {
    return {
      kind: "assistant",
      title: t("optimize.agentReply"),
      content: reply,
    };
  }

  const summary = stringField(parsed, "summary");
  if (summary) {
    return {
      kind: "agent",
      title: t("optimize.summaryPreparing"),
      content: summary,
    };
  }

  return protocolError(t("optimize.protocolInvalid"), t("optimize.protocolMissingFields"));
}

function previewAgentDecision(
  action: string,
  args: JsonRecord | null,
  publicUpdate: string | null,
  t: MessageTranslator,
): ModelStreamPreview {
  switch (action) {
    case "think":
      return previewThinking(publicUpdate, t);
    case "run_tool":
      return previewToolCall(args, publicUpdate, t);
    case "final":
      return previewFinal(args, t);
    default:
      return protocolError(t("optimize.invalidAction"), action);
  }
}

function previewThinking(publicUpdate: string | null, t: MessageTranslator): ModelStreamPreview {
  if (publicUpdate) {
    return {
      kind: "agent",
      title: t("optimize.agentNote"),
      content: publicUpdate,
    };
  }
  return protocolError(t("optimize.modelStageMissing"), t("optimize.thinkPublicUpdateMissing"));
}

function previewToolCall(
  args: JsonRecord | null,
  publicUpdate: string | null,
  t: MessageTranslator,
): ModelStreamPreview {
  const tool = args ? stringField(args, "tool") : null;
  const toolArgs = args ? recordField(args, "arguments") : null;
  if (!publicUpdate) {
    return protocolError(t("optimize.modelStageMissing"), t("optimize.runToolUpdateMissing"));
  }
  if (tool === "get_project_information") {
    const mode = toolArgs ? stringField(toolArgs, "mode") : null;
    return {
      kind: "agent",
      title: mode === "search" ? t("optimize.callSearch") : t("optimize.callRead"),
      content: publicUpdate,
    };
  }

  if (tool === "edit_article") {
    return {
      kind: "agent",
      title: t("optimize.callEditArticle"),
      content: publicUpdate,
    };
  }

  return {
    kind: "agent",
    title: t("optimize.callTool"),
    content: publicUpdate,
  };
}

function previewFinal(args: JsonRecord | null, t: MessageTranslator): ModelStreamPreview {
  const reply = args ? stringField(args, "reply") : null;
  if (reply) {
    return {
      kind: "assistant",
      title: t("optimize.agentReply"),
      content: reply,
    };
  }

  const issues = args && Array.isArray(args.issues) ? args.issues : null;
  if (issues) return previewIssues(issues, t);

  return protocolError(t("optimize.invalidFinalArguments"), t("optimize.missingReplyOrIssues"));
}

function previewIssues(rawIssues: unknown[], t: MessageTranslator): ModelStreamPreview {
  const issues = parseOptimizeIssues(rawIssues);
  return {
    kind: "assistant",
    title: t("optimize.analysisPreparing"),
    content: lines([
      issues.length === 0
        ? t("optimize.noIssues")
        : t("optimize.issueCount", { count: issues.length }),
      ...issues.map((issue, index) =>
        t("optimize.issueLine", {
          index: index + 1,
          description: issue.description || t("optimize.issueDescriptionMissing"),
          tool: issue.tool,
          severity: issue.severity ? ` / ${issue.severity}` : "",
          id: issue.id,
        }),
      ),
    ]),
  };
}

function previewPatches(parsed: JsonRecord, t: MessageTranslator): ModelStreamPreview {
  const summary = stringField(parsed, "summary");
  const patches = Array.isArray(parsed.patches) ? parsed.patches : [];
  return {
    kind: "agent",
    title: t("optimize.patchPreparing"),
    content: lines([
      summary ? t("optimize.patchSummary", { summary }) : null,
      patches.length > 0 ? t("optimize.patchCount", { count: patches.length }) : t("optimize.noSafeChanges"),
      formatPatchReasons(patches, t),
    ]),
  };
}

function previewPartialOutput(raw: string, t: MessageTranslator): ModelStreamPreview {
  const reply = extractStringField(raw, "reply");
  if (reply) {
    return {
      kind: "assistant",
      title: t("optimize.agentReply"),
      content: reply,
    };
  }

  const action = extractStringField(raw, "action");
  const publicUpdate = extractStringField(raw, "public_update");
  if (action === "think") return previewPartialThinking(publicUpdate, t);
  if (action === "run_tool") return previewPartialToolCall(raw, publicUpdate, t);
  if (action === "final") return previewPartialFinal(raw, t);

  const summary = extractStringField(raw, "summary");
  if (summary || hasField(raw, "patches")) {
    return {
      kind: "agent",
      title: t("optimize.patchPreparing"),
      content: lines([
        summary ? t("optimize.patchSummary", { summary }) : null,
        partialPatchCount(raw, t),
        formatPartialPatchReasons(raw),
      ]),
    };
  }

  if (hasField(raw, "thought")) {
    return thinkingPreview(t);
  }

  return {
    kind: "agent",
    title: t("optimize.partialOutput"),
    content: "",
  };
}

function previewPartialThinking(publicUpdate: string | null, t: MessageTranslator): ModelStreamPreview {
  if (publicUpdate) {
    return {
      kind: "agent",
      title: t("optimize.agentNote"),
      content: publicUpdate,
    };
  }
  return thinkingPreview(t);
}

function previewPartialToolCall(raw: string, publicUpdate: string | null, t: MessageTranslator): ModelStreamPreview {
  const tool = extractStringField(raw, "tool");
  const content = publicUpdate ?? "";
  if (tool === "get_project_information") {
    const mode = extractStringField(raw, "mode");
    return {
      kind: "agent",
      title: mode === "search" ? t("optimize.callSearch") : t("optimize.callRead"),
      content,
    };
  }

  if (tool === "edit_article") {
    return {
      kind: "agent",
      title: t("optimize.callEditArticle"),
      content,
    };
  }

  return {
    kind: "agent",
    title: t("optimize.toolSelecting"),
    content,
  };
}

function thinkingPreview(t: MessageTranslator): ModelStreamPreview {
  return {
    kind: "agent",
    title: t("optimize.thinking"),
    content: "",
  };
}

function previewPartialFinal(raw: string, t: MessageTranslator): ModelStreamPreview {
  if (hasField(raw, "issues")) {
    const descriptions = extractStringFields(raw, "description");
    return {
      kind: "assistant",
      title: t("optimize.analysisPreparing"),
      content: lines([
        descriptions.length > 0 ? t("optimize.identifiedIssues") : t("optimize.preparingIssues"),
        ...descriptions.map((description, index) => `${index + 1}. ${description}`),
      ]),
    };
  }

  return {
    kind: "agent",
    title: t("optimize.responsePreparing"),
    content: "",
  };
}

function formatPatchReasons(patches: unknown[], t: MessageTranslator): string | null {
  const linesOut = patches
    .map((patch, index) => {
      if (!isRecord(patch)) return null;
      const reason = stringField(patch, "reason");
      const operation = stringField(patch, "operation");
      if (!reason && !operation) return null;
      const description = reason || t("optimize.localChange");
      return operation
        ? `${index + 1}. ${t("optimize.patchReasonWithOperation", {
            operation: patchOperationLabel(operation, t),
            reason: description,
          })}`
        : `${index + 1}. ${description}`;
    })
    .filter((line): line is string => line != null);
  return linesOut.length > 0 ? linesOut.join("\n") : null;
}

function formatPartialPatchReasons(raw: string): string | null {
  const reasons = extractStringFields(raw, "reason");
  if (reasons.length === 0) return null;
  return reasons.map((reason, index) => `${index + 1}. ${reason}`).join("\n");
}

function partialPatchCount(raw: string, t: MessageTranslator): string | null {
  const count = countField(raw, "patch_id");
  return count > 0 ? t("optimize.draftedChanges", { count }) : null;
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
