import { useCallback } from "react";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import type { ToastType } from "../../../components/feedback/Toast";
import { type MessageTranslator } from "../../../i18n";

type Ref<T> = { current: T };

interface UseOutlineTabGuardsOptions {
  canEditOutlines: boolean;
  addToast: (message: string, type: ToastType) => void;
  t: MessageTranslator;
  mountedRef: Ref<boolean>;
  activeProjectIdRef: Ref<number>;
  loadSeqRef: Ref<number>;
  actionSeqRef: Ref<number>;
  previewSeqRef: Ref<number>;
  actionBusyRef: Ref<boolean>;
  previewBusyRef: Ref<boolean>;
  confirmBusyRef: Ref<boolean>;
}

export const requestOptimizationDirection = (
  title: string,
  t: MessageTranslator,
) =>
  promptDialog({
    title,
    message: t("outline.dialogs.optimizationDirectionMessage"),
    placeholder: t("outline.dialogs.optimizationDirectionPlaceholder"),
    multiline: true,
    confirmText: t("outline.dialogs.startOptimization"),
  });

export function useOutlineTabGuards({
  canEditOutlines,
  addToast,
  t,
  mountedRef,
  activeProjectIdRef,
  loadSeqRef,
  actionSeqRef,
  previewSeqRef,
  actionBusyRef,
  previewBusyRef,
  confirmBusyRef,
}: UseOutlineTabGuardsOptions) {
  const isProjectCurrent = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [activeProjectIdRef, mountedRef],
  );
  const isLoadCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && loadSeqRef.current === requestSeq,
    [isProjectCurrent, loadSeqRef],
  );
  const isActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && actionSeqRef.current === requestSeq,
    [actionSeqRef, isProjectCurrent],
  );
  const isPreviewCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && previewSeqRef.current === requestSeq,
    [isProjectCurrent, previewSeqRef],
  );

  const beginAction = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (!canEditOutlines) {
        addToast(t("outline.readOnlyEdit"), "error");
        return null;
      }
      if (actionBusyRef.current) {
        addToast(t("outline.operationInProgress"), "error");
        return null;
      }
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      return actionSeqRef.current;
    },
    [actionBusyRef, actionSeqRef, addToast, canEditOutlines, isProjectCurrent, t],
  );
  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) actionBusyRef.current = false;
  }, [actionBusyRef, actionSeqRef]);
  const invalidatePreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
  }, [previewBusyRef, previewSeqRef]);
  const beginPreview = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (!canEditOutlines) {
        addToast(t("outline.readOnlyPreview"), "error");
        return null;
      }
      if (previewBusyRef.current) return null;
      previewBusyRef.current = true;
      previewSeqRef.current += 1;
      return previewSeqRef.current;
    },
    [addToast, canEditOutlines, isProjectCurrent, previewBusyRef, previewSeqRef, t],
  );
  const finishPreview = useCallback((requestSeq: number) => {
    if (previewSeqRef.current === requestSeq) previewBusyRef.current = false;
  }, [previewBusyRef, previewSeqRef]);
  const beginConfirmAction = useCallback(
    (requestProjectId: number): boolean => {
      if (!isProjectCurrent(requestProjectId)) return false;
      if (!canEditOutlines) {
        addToast(t("outline.readOnlyEdit"), "error");
        return false;
      }
      if (actionBusyRef.current || confirmBusyRef.current) {
        addToast(t("outline.operationInProgress"), "error");
        return false;
      }
      confirmBusyRef.current = true;
      return true;
    },
    [actionBusyRef, addToast, canEditOutlines, confirmBusyRef, isProjectCurrent, t],
  );
  const finishConfirmAction = useCallback(() => {
    confirmBusyRef.current = false;
  }, [confirmBusyRef]);

  return {
    beginAction,
    beginConfirmAction,
    beginPreview,
    finishAction,
    finishConfirmAction,
    finishPreview,
    invalidatePreview,
    isActionCurrent,
    isLoadCurrent,
    isPreviewCurrent,
    isProjectCurrent,
  };
}
