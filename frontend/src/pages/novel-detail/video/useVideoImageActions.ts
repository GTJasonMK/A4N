import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { deleteImage, type GeneratedImage } from "../../../api/image";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../../components/feedback/Toast";
import type { MessageTranslator } from "../../../i18n";

type AddToast = (message: string, type: ToastType) => void;

interface UseVideoImageActionsParams {
  t: MessageTranslator;
  addToast: AddToast;
  actionScopeKey: string | null;
  loadAll: () => Promise<boolean>;
}

export const useVideoImageActions = ({
  t,
  addToast,
  actionScopeKey,
  loadAll,
}: UseVideoImageActionsParams) => {
  const tRef = useRef(t);
  tRef.current = t;
  const currentT = () => tRef.current;
  const [imageBusyId, setImageBusyId] = useState<number | null>(null);
  const actionScopeRef = useRef(actionScopeKey);

  useEffect(() => {
    actionScopeRef.current = actionScopeKey;
    setImageBusyId(null);
  }, [actionScopeKey]);

  const isCurrentActionScope = useCallback(
    (scope: string | null) => actionScopeRef.current === scope,
    [],
  );

  const handleDeleteImage = useCallback(
    async (image: GeneratedImage) => {
      const requestScope = actionScopeKey;
      const ok = await confirmDialog({
        title: currentT()("video.material.deleteTitle"),
        message: currentT()("video.material.deleteMessage"),
        confirmText: currentT()("common.delete"),
        dialogType: "danger",
      });
      if (!ok) return;
      if (!isCurrentActionScope(requestScope)) return;
      setImageBusyId(image.id);
      try {
        await deleteImage(image.id);
        if (!isCurrentActionScope(requestScope)) return;
        const refreshed = await loadAll();
        if (!isCurrentActionScope(requestScope)) return;
        if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
        addToast(currentT()("video.material.deleted"), "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, currentT()("video.material.deleteFailed")), "error");
      } finally {
        if (isCurrentActionScope(requestScope)) setImageBusyId(null);
      }
    },
    [
      actionScopeKey,
      addToast,
      isCurrentActionScope,
      loadAll,
      t,
    ],
  );

  return {
    imageBusyId,
    handleDeleteImage,
  };
};
