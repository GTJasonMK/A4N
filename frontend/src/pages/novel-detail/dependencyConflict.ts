import type { DownstreamConflictStrategy } from "../../api/dependencyConflict";
import { choiceDialog } from "../../components/feedback/ConfirmDialog";

type DownstreamConflictChoice = Exclude<DownstreamConflictStrategy, "reject">;

interface DownstreamConflictOptions {
  title: string;
  assetLabel: string;
  downstreamLabel: string;
  downstreamCount: number;
  operation: string;
}

export async function chooseDownstreamConflictStrategy({
  title,
  assetLabel,
  downstreamLabel,
  downstreamCount,
  operation,
}: DownstreamConflictOptions): Promise<DownstreamConflictChoice | null> {
  if (downstreamCount <= 0) return "keep_downstream";
  const result = await choiceDialog({
    title,
    message:
      `${assetLabel} 已派生 ${downstreamCount} 个${downstreamLabel}。\n\n` +
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
