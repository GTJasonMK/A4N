import { useId } from "react";
import { type MessageTranslator } from "../../i18n";
import { confirmDialog } from "./ConfirmDialog";
import { useDirty } from "../navigation/dirtyRegistry";

export async function confirmDiscardMediaChanges(dirty: boolean, t: MessageTranslator) {
  return !dirty || confirmDialog({ title: t("common.unsavedChangesTitle"), message: t("common.mediaAuthoring.unsaved"), confirmText: t("common.discardDraft"), dialogType: "warning" });
}

export function UnsavedChangesGuard(props: { dirty: boolean; saving?: boolean; onDiscard?: () => void }) {
  const id = useId();
  useDirty(`media-route-guard:${id}`, props.dirty, props.onDiscard);
  return null;
}
