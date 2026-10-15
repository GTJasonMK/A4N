import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { deleteImage, type GeneratedImage, type ImageRecordDeleteResult } from "../../../api/image";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../../components/feedback/Toast";
import type { MessageTranslator } from "../../../i18n";
import type { LoadVideoTabData } from "./useVideoTabLoading";

type AddToast = (message: string, type: ToastType) => void;

interface UseVideoImageActionsParams {
  t: MessageTranslator;
  addToast: AddToast;
  actionScopeKey: string | null;
  loadAll: LoadVideoTabData;
  ensureActionAvailable?: () => boolean;
  onImageDeleted: (result: ImageRecordDeleteResult) => void;
}

export const useVideoImageActions = ({
  t,
  addToast,
  actionScopeKey,
  loadAll,
  ensureActionAvailable,
  onImageDeleted,
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
      if (ensureActionAvailable && !ensureActionAvailable()) return;
      const ok = await confirmDialog({
        title: currentT()("video.material.deleteTitle"),
        message: currentT()("video.material.deleteMessage"),
        confirmText: currentT()("common.delete"),
        dialogType: "danger",
      });
      if (!ok || (ensureActionAvailable && !ensureActionAvailable())) return;
      if (!isCurrentActionScope(requestScope)) return;
      setImageBusyId(image.id);
      try {
        const result = await deleteImage(image.id);
        if (!isCurrentActionScope(requestScope)) return;
        onImageDeleted(result);
        addToast(currentT()("video.material.deleted"), "success");
        void Promise.resolve().then(() => loadAll({ showLoading: false })).then((refreshed) => {
          if (isCurrentActionScope(requestScope) && !refreshed) {
            addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
          }
        }).catch((error) => {
          if (isCurrentActionScope(requestScope)) {
            addToast(extractErrorMessage(error, currentT()("video.chapter.refreshVideoDataFailed")), "error");
          }
        });
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
      ensureActionAvailable,
      onImageDeleted,
      t,
    ],
  );

  return {
    imageBusyId,
    handleDeleteImage,
  };
};
