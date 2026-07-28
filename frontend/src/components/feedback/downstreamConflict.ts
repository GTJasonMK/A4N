import { extractErrorMessage } from "../../api";
import type { DownstreamConflictStrategy } from "../../api/dependencyConflict";
import { choiceDialog } from "./ConfirmDialog";

type DownstreamConflictChoice = Exclude<DownstreamConflictStrategy, "reject">;

export interface DownstreamConflictPresentation {
  title: string;
  assetLabel: string;
  downstreamLabel: string;
  downstreamCount: number;
  operation: string;
  forceChoice?: boolean;
  itemLabel?: string;
  items?: string[];
  backendMessage?: string;
}

export type DownstreamConflictExecutionResult<T> =
  | { status: "completed"; value: T }
  | { status: "cancelled" };

interface DownstreamConflictExecutionOptions<T>
  extends DownstreamConflictPresentation {
  execute: (strategy: DownstreamConflictStrategy) => Promise<T>;
}

const downstreamConflictCountFromMessage = (message: string): number | null => {
  const matched = message.match(/已有\s*(\d+)\s*个下游派生产物/);
  if (!matched) return null;
  const count = Number(matched[1]);
  return Number.isSafeInteger(count) && count > 0 ? count : null;
};

const isDownstreamConflictMessage = (message: string): boolean =>
  message.includes("个下游派生产物") &&
  message.includes("请选择删除下游后继续，或保留下游继续");

async function chooseDownstreamConflictStrategy({
  title,
  assetLabel,
  downstreamLabel,
  downstreamCount,
  operation,
  itemLabel,
  items = [],
  backendMessage,
}: DownstreamConflictPresentation): Promise<DownstreamConflictChoice | null> {
  const itemSection =
    items.length > 0
      ? `\n\n${itemLabel || "涉及的记录"}：\n${items.map((item) => `- ${item}`).join("\n")}`
      : "";
  const backendSection = backendMessage
    ? `\n\n后端完整依赖检查：\n${backendMessage}`
    : "";
  const downstreamSummary =
    downstreamCount > 0
      ? `${assetLabel} 已派生 ${downstreamCount} 个${downstreamLabel}。`
      : `${assetLabel} 当前页面未识别到下游产物，但后端执行前仍会按完整依赖图再次检查。`;
  const result = await choiceDialog({
    title,
    message:
      `${downstreamSummary}${itemSection}${backendSection}\n\n` +
      `你可以删除这些下游派生产物后再${operation}，也可以只${operation}源资产并保留下游产物。` +
      "保留下游时，已有下游产物可能与新的源资产内容不一致，需要你后续按需重新生成或清理。",
    confirmText: "删除下游后继续",
    secondaryText: "保留下游继续",
    cancelText: "取消",
    dialogType: "warning",
  });
  if (result === "confirm") return "delete_downstream";
  if (result === "secondary") return "keep_downstream";
  return null;
}

export async function runWithDownstreamConflictResolution<T>({
  execute,
  ...presentation
}: DownstreamConflictExecutionOptions<T>): Promise<DownstreamConflictExecutionResult<T>> {
  let strategy: DownstreamConflictStrategy = "reject";
  if (presentation.forceChoice || presentation.downstreamCount > 0) {
    const selected = await chooseDownstreamConflictStrategy(presentation);
    if (selected == null) return { status: "cancelled" };
    strategy = selected;
  }

  try {
    return { status: "completed", value: await execute(strategy) };
  } catch (error) {
    if (strategy !== "reject") throw error;
    const message = extractErrorMessage(error, "下游依赖检查失败");
    if (!isDownstreamConflictMessage(message)) throw error;
    const selected = await chooseDownstreamConflictStrategy({
      ...presentation,
      downstreamCount:
        downstreamConflictCountFromMessage(message) ??
        Math.max(presentation.downstreamCount, 1),
      forceChoice: true,
      backendMessage: message,
    });
    if (selected == null) return { status: "cancelled" };
    return { status: "completed", value: await execute(selected) };
  }
}
