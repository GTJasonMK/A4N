import { useBeforeUnload, useBlocker } from "react-router-dom";
import { BookButton } from "../ui/BookButton";
import { Modal } from "../ui/Modal";
import { useI18n, type MessageTranslator } from "../../i18n";
import { confirmDialog } from "./ConfirmDialog";

export async function confirmDiscardMediaChanges(dirty: boolean, t: MessageTranslator) {
  return !dirty || confirmDialog({ title: t("common.unsavedChangesTitle"), message: t("common.mediaAuthoring.unsaved"), confirmText: t("common.discardDraft"), dialogType: "warning" });
}

export function UnsavedChangesGuard({ dirty, onDiscard }: { dirty: boolean; onDiscard: () => void }) {
  const { t } = useI18n();
  const blocker = useBlocker(dirty);
  useBeforeUnload((event) => {
    if (!dirty) return;
    event.preventDefault();
    event.returnValue = "";
  });
  if (blocker.state !== "blocked") return null;
  return (
    <Modal isOpen onClose={blocker.reset} title={t("common.unsavedChangesTitle")} tone="warning" maxWidthClassName="max-w-xl"
      footer={<>
        <BookButton variant="secondary" onClick={blocker.reset}>{t("common.cancel")}</BookButton>
        <BookButton variant="warning" onClick={() => { onDiscard(); blocker.proceed(); }}>{t("common.discardDraft")}</BookButton>
      </>}>
      {t("common.mediaAuthoring.unsaved")}
    </Modal>
  );
}
