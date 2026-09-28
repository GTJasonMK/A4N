import { useBeforeUnload, useBlocker } from "react-router-dom";
import { BookButton } from "../ui/BookButton";
import { Modal } from "../ui/Modal";
import { useI18n, type MessageTranslator } from "../../i18n";
import { confirmDialog } from "./ConfirmDialog";

export async function confirmDiscardMediaChanges(dirty: boolean, t: MessageTranslator) {
  return !dirty || confirmDialog({ title: t("common.unsavedChangesTitle"), message: t("common.mediaAuthoring.unsaved"), confirmText: t("common.discardDraft"), dialogType: "warning" });
}

export function UnsavedChangesGuard({ dirty, saving = false, onDiscard }: { dirty: boolean; saving?: boolean; onDiscard: () => void }) {
  const { t } = useI18n();
  const blocker = useBlocker(dirty);
  useEffect(() => {
    if (!dirty && !saving && blocker.state === "blocked") blocker.proceed();
  }, [dirty, saving, blocker.state, blocker.proceed]);
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
        <BookButton variant="warning" disabled={saving} onClick={onDiscard}>{saving ? t("common.loading") : t("common.discardDraft")}</BookButton>
      </>}>
      {t("common.mediaAuthoring.unsaved")}
    </Modal>
  );
}
import { useEffect } from "react";
