import type { DownstreamConflictStrategy } from "../../api/generated/host";
import { choiceDialog } from "../../components/feedback/ConfirmDialog";
import type { MessageTranslator } from "../../i18n";

interface DownstreamConflictOptions {
  title: string;
  assetLabel: string;
  downstreamLabel: string;
  downstreamCount: number;
  operation: string;
  t: MessageTranslator;
}

export async function chooseDownstreamConflictStrategy({
  title,
  assetLabel,
  downstreamLabel,
  downstreamCount,
  operation,
  t,
}: DownstreamConflictOptions): Promise<DownstreamConflictStrategy | null> {
  if (downstreamCount <= 0) return "reject";
  const result = await choiceDialog({
    title,
    message: t("common.downstreamMessage", {
      asset: assetLabel,
      count: downstreamCount,
      downstream: downstreamLabel,
      operation,
    }),
    confirmText: t("common.deleteDownstreamContinue"),
    secondaryText: t("common.keepDownstreamContinue"),
    cancelText: t("common.cancel"),
    dialogType: "warning",
  });
  if (result === "confirm") return "delete_downstream";
  if (result === "secondary") return "keep_downstream";
  return null;
}
