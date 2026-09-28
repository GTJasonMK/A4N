import { type MessageTranslator } from "../../i18n";
import { confirmDialog } from "./ConfirmDialog";

export async function confirmDiscardMediaChanges(dirty: boolean, t: MessageTranslator) {
  return !dirty || confirmDialog({ title: t("common.unsavedChangesTitle"), message: t("common.mediaAuthoring.unsaved"), confirmText: t("common.discardDraft"), dialogType: "warning" });
}
