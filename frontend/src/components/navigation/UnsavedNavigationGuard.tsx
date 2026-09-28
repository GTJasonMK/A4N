import { useEffect, useId } from "react";
import { useBeforeUnload, useBlocker } from "react-router-dom";
import { useI18n } from "../../i18n";
import { useDirty, useDirtyRegistry, useDirtyRegistryVersion } from "./dirtyRegistry";
import { Modal } from "../ui/Modal";
import { BookButton } from "../ui/BookButton";

interface UnsavedNavigationGuardProps {
  dirty?: boolean;
  name: string;
  action: string;
}

export const UnsavedNavigationGuard = ({
  dirty = false,
  name,
  action,
}: UnsavedNavigationGuardProps) => {
  const { t } = useI18n();
  const registry = useDirtyRegistry();
  useDirtyRegistryVersion();
  const id = useId();
  useDirty(`route-guard:${id}`, dirty);
  const isDirty = dirty || registry.hasDirty();
  const canDiscard = registry.canDiscardDirty();
  useBeforeUnload((event) => {
    if (!isDirty) return;
    event.preventDefault();
    event.returnValue = "";
  });
  const blocker = useBlocker(({ currentLocation, nextLocation }) =>
    isDirty && (
      currentLocation.pathname !== nextLocation.pathname ||
      currentLocation.search !== nextLocation.search ||
      currentLocation.hash !== nextLocation.hash
    ),
  );

  useEffect(() => {
    if (blocker.state === "blocked" && !isDirty) blocker.proceed();
  }, [blocker, isDirty]);

  if (blocker.state !== "blocked" || !isDirty) return null;
  return (
    <Modal
      isOpen
      onClose={blocker.reset}
      title={t("common.unsavedChangesTitle")}
      tone="warning"
      maxWidthClassName="max-w-xl"
      footer={(
        <>
          <BookButton variant="secondary" onClick={blocker.reset}>{t("common.cancel")}</BookButton>
          <BookButton
            variant="warning"
            disabled={!canDiscard}
            onClick={() => {
              void registry.discardDirty().then((discarded) => {
                if (discarded) blocker.proceed();
              });
            }}
          >
            {t("common.discardDraft")}
          </BookButton>
        </>
      )}
    >
      {canDiscard
        ? t("common.unsavedChangesMessage", { name, action })
        : t("common.operationInProgress")}
    </Modal>
  );
};
