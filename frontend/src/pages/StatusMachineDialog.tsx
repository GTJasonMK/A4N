import React, { useCallback, useLayoutEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../api";
import {
  forwardProjectStatus,
  getProjectStatusMachine,
  previewRollback,
  rollbackProjectStatus,
  type NovelProject,
  type ProjectRollbackPreview,
  type ProjectStatus,
  type ProjectStatusMachineSummary,
  type ProjectStatusMutationResult,
} from "../api/project";
import { useToast } from "../components/feedback/Toast";
import { BookButton } from "../components/ui/BookButton";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { Modal } from "../components/ui/Modal";
import { ForwardActionCard } from "./status-machine-dialog/ForwardActionCard";
import { RollbackCard } from "./status-machine-dialog/RollbackCard";
import { StatusFlowView } from "./status-machine-dialog/StatusFlowView";
import { StatusHistoryPanel } from "./status-machine-dialog/StatusHistoryPanel";
import { useI18n } from "../i18n";
import { projectStatusLabel } from "./status-machine-dialog/statusLabels";

interface StatusMachineDialogProps {
  open: boolean;
  onClose: () => void;
  project: NovelProject;
  canForwardStatus: boolean;
  canRollbackStatus: boolean;
  onMutated: () => Promise<boolean | void> | boolean | void;
  onRollbackCommitted: () => void;
}

export const StatusMachineDialog: React.FC<StatusMachineDialogProps> = ({
  open,
  onClose,
  project,
  canForwardStatus,
  canRollbackStatus,
  onMutated,
  onRollbackCommitted,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const [machine, setMachine] = useState<ProjectStatusMachineSummary | null>(null);
  const [machineError, setMachineError] = useState<string | null>(null);
  const [loadingMachine, setLoadingMachine] = useState(false);
  const [acting, setActing] = useState<string | null>(null);
  const [rollbackTarget, setRollbackTarget] = useState<ProjectStatus | null>(null);
  const [rollbackExpectedFrom, setRollbackExpectedFrom] = useState<ProjectStatus | null>(null);
  const [rollbackPreview, setRollbackPreview] = useState<ProjectRollbackPreview | null>(null);
  const mountedRef = useRef(false);
  const scopeRef = useRef("closed");
  const activeProjectIdRef = useRef(project.id);
  const machineSeqRef = useRef(0);
  const rollbackPreviewSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const actingRef = useRef<string | null>(null);
  const rollbackPreviewInFlightKeyRef = useRef<string | null>(null);
  const scopeKey = open
    ? [
        project.id,
        project.status,
        project.lifecycle_gate,
        project.updated_at,
        project.my_role,
      ].join(":")
    : "closed";
  const actionBusy = acting !== null;

  const isCurrentScope = useCallback(
    (requestScope: string) => mountedRef.current && scopeRef.current === requestScope,
    [],
  );

  const isCurrentRequest = useCallback(
    (requestScope: string, ref: React.MutableRefObject<number>, requestSeq: number) =>
      isCurrentScope(requestScope) && ref.current === requestSeq,
    [isCurrentScope],
  );

  const isCurrentMutation = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      mountedRef.current &&
      activeProjectIdRef.current === requestProjectId &&
      actionSeqRef.current === requestSeq,
    [],
  );

  const loadMachine = useCallback(async (): Promise<boolean> => {
    const requestScope = scopeKey;
    const requestSeq = machineSeqRef.current + 1;
    machineSeqRef.current = requestSeq;
    setLoadingMachine(true);
    setMachineError(null);
    setMachine(null);
    rollbackPreviewSeqRef.current += 1;
    rollbackPreviewInFlightKeyRef.current = null;
    setRollbackTarget(null);
    setRollbackExpectedFrom(null);
    setRollbackPreview(null);
    try {
      const data = await getProjectStatusMachine(project.id);
      if (!isCurrentRequest(requestScope, machineSeqRef, requestSeq)) return false;
      setMachine(data);
      setMachineError(null);
      return true;
    } catch (err) {
      if (isCurrentRequest(requestScope, machineSeqRef, requestSeq)) {
        setMachineError(extractErrorMessage(err, t("statusMachine.loadFailed")));
      }
      return false;
    } finally {
      if (isCurrentRequest(requestScope, machineSeqRef, requestSeq)) {
        setLoadingMachine(false);
      }
    }
  }, [addToast, isCurrentRequest, project.id, scopeKey, t]);

  const clearRollbackPreview = useCallback(() => {
    rollbackPreviewSeqRef.current += 1;
    rollbackPreviewInFlightKeyRef.current = null;
    setRollbackTarget(null);
    setRollbackExpectedFrom(null);
    setRollbackPreview(null);
  }, []);

  useLayoutEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      machineSeqRef.current += 1;
      rollbackPreviewSeqRef.current += 1;
      actionSeqRef.current += 1;
      actingRef.current = null;
      rollbackPreviewInFlightKeyRef.current = null;
    };
  }, []);

  useLayoutEffect(() => {
    activeProjectIdRef.current = project.id;
    scopeRef.current = scopeKey;
    machineSeqRef.current += 1;
    rollbackPreviewSeqRef.current += 1;
    actionSeqRef.current += 1;
    actingRef.current = null;
    rollbackPreviewInFlightKeyRef.current = null;
    setActing(null);
    setRollbackTarget(null);
    setRollbackExpectedFrom(null);
    setRollbackPreview(null);
    setMachine(null);
    setMachineError(null);
    if (!open) {
      setLoadingMachine(false);
      return;
    }
    void loadMachine();
  }, [loadMachine, open, scopeKey]);

  const executeStatusAction = useCallback(
    async (
      key: string,
      action: () => Promise<ProjectStatusMutationResult>,
      successMessage: string,
    ) => {
      if (actingRef.current != null) return false;
      const requestProjectId = project.id;
      const requestSeq = actionSeqRef.current + 1;
      actionSeqRef.current = requestSeq;
      actingRef.current = key;
      setActing(key);
      try {
        const result = await action();
        if (!isCurrentMutation(requestProjectId, requestSeq)) return false;
        clearRollbackPreview();
        addToast(successMessage, "success");
        if (result.post_commit_errors.length > 0) {
          addToast(
            t("statusMachine.postCommitFailed", {
              details: result.post_commit_errors.join("; "),
            }),
            "error",
          );
        }
        void Promise.resolve(onMutated())
          .then((refreshed) => {
            if (refreshed === false && isCurrentMutation(requestProjectId, requestSeq)) {
              addToast(t("statusMachine.refreshFailed"), "error");
            }
          })
          .catch(() => {
            if (isCurrentMutation(requestProjectId, requestSeq)) {
              addToast(t("statusMachine.refreshFailed"), "error");
            }
          });
        if (open && !await loadMachine()) {
          if (isCurrentMutation(requestProjectId, requestSeq)) {
            addToast(t("statusMachine.stateRefreshFailed"), "error");
          }
        }
        if (!isCurrentMutation(requestProjectId, requestSeq)) return false;
        return true;
      } catch (err) {
        if (isCurrentMutation(requestProjectId, requestSeq)) {
          addToast(extractErrorMessage(err, t("statusMachine.actionFailed")), "error");
          clearRollbackPreview();
          void (async () => {
            let refreshFailed = false;
            try {
              refreshFailed = (await onMutated()) === false;
            } catch {
              refreshFailed = true;
            }
            if (!isCurrentMutation(requestProjectId, requestSeq)) return;
            if (open && !await loadMachine()) refreshFailed = true;
            if (!isCurrentMutation(requestProjectId, requestSeq)) return;
            if (refreshFailed) addToast(t("statusMachine.stateRefreshFailed"), "error");
          })();
        }
        return false;
      } finally {
        if (actionSeqRef.current === requestSeq) actingRef.current = null;
        if (isCurrentMutation(requestProjectId, requestSeq)) setActing(null);
      }
    },
    [
      addToast,
      clearRollbackPreview,
      isCurrentMutation,
      loadMachine,
      onMutated,
      open,
      project.id,
      t,
    ],
  );

  const handleForward = useCallback(
    async (target: ProjectStatus) => {
      if (!canForwardStatus) {
        addToast(t("statusMachine.noForwardPermission"), "error");
        return;
      }
      await executeStatusAction(
        `forward-${target}`,
        () => forwardProjectStatus(project.id, target),
        t("statusMachine.forwardSuccess", { status: projectStatusLabel(target, t) }),
      );
    },
    [addToast, canForwardStatus, executeStatusAction, project.id, t],
  );

  const handleRollbackPreview = useCallback(
    async (target: ProjectStatus) => {
      if (!canRollbackStatus || !machine) {
        addToast(t("statusMachine.noRollbackPermission"), "error");
        return;
      }
      const requestScope = scopeKey;
      const expectedFrom = machine.current_status;
      const requestKey = `${requestScope}:${expectedFrom}:${target}`;
      if (rollbackPreviewInFlightKeyRef.current === requestKey) return;
      const requestSeq = rollbackPreviewSeqRef.current + 1;
      rollbackPreviewSeqRef.current = requestSeq;
      rollbackPreviewInFlightKeyRef.current = requestKey;
      setRollbackTarget(target);
      setRollbackExpectedFrom(expectedFrom);
      setRollbackPreview(null);
      try {
        const data = await previewRollback(project.id, expectedFrom, target);
        if (!isCurrentRequest(requestScope, rollbackPreviewSeqRef, requestSeq)) return;
        setRollbackPreview(data);
      } catch (err) {
        if (isCurrentRequest(requestScope, rollbackPreviewSeqRef, requestSeq)) {
          addToast(extractErrorMessage(err, t("statusMachine.previewFailed")), "error");
          clearRollbackPreview();
          void (async () => {
            let refreshFailed = false;
            try {
              refreshFailed = (await onMutated()) === false;
            } catch {
              refreshFailed = true;
            }
            if (!isCurrentScope(requestScope)) return;
            if (!await loadMachine()) refreshFailed = true;
            if (refreshFailed && isCurrentScope(requestScope)) {
              addToast(t("statusMachine.stateRefreshFailed"), "error");
            }
          })();
        }
      } finally {
        if (
          rollbackPreviewInFlightKeyRef.current === requestKey &&
          rollbackPreviewSeqRef.current === requestSeq
        ) {
          rollbackPreviewInFlightKeyRef.current = null;
        }
      }
    },
    [
      addToast,
      canRollbackStatus,
      clearRollbackPreview,
      isCurrentRequest,
      isCurrentScope,
      loadMachine,
      machine,
      onMutated,
      project.id,
      scopeKey,
      t,
    ],
  );

  const handleConfirmRollback = useCallback(async () => {
    if (!canRollbackStatus) {
      addToast(t("statusMachine.noRollbackPermission"), "error");
      return;
    }
    const target = rollbackTarget;
    const expectedFrom = rollbackExpectedFrom;
    if (!target || !rollbackPreview || expectedFrom === null) return;
    const requestProjectId = project.id;
    const completed = await executeStatusAction(
      `rollback-${target}`,
      () => rollbackProjectStatus(project.id, expectedFrom, target),
      t("statusMachine.rollbackSuccess", { status: projectStatusLabel(target, t) }),
    );
    if (completed && mountedRef.current && activeProjectIdRef.current === requestProjectId) {
      clearRollbackPreview();
      onRollbackCommitted();
    } else if (mountedRef.current && activeProjectIdRef.current === requestProjectId) {
      clearRollbackPreview();
    }
  }, [
    addToast,
    canRollbackStatus,
    clearRollbackPreview,
    executeStatusAction,
    onRollbackCommitted,
    project.id,
    rollbackExpectedFrom,
    rollbackPreview,
    rollbackTarget,
    t,
  ]);

  return (
    <Modal
      isOpen={open}
      onClose={() => {
        if (!actionBusy) onClose();
      }}
      title={t("statusMachine.title")}
      subtitle={project.title}
      maxWidthClassName="max-w-xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!actionBusy}
      closeOnEscape={!actionBusy}
      showCloseButton={!actionBusy}
    >
      {loadingMachine && !machine ? (
        <div className="a4n-flat-surface inline-flex items-center gap-2 p-3 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> {t("statusMachine.loading")}
        </div>
      ) : machineError ? (
        <div role="alert" className="a4n-flat-surface flex flex-col items-start gap-3 p-3 text-xs text-error">
          <span>{machineError}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void loadMachine()} disabled={loadingMachine}>
            <RefreshCw size={13} className={loadingMachine ? "animate-spin" : undefined} />
            {t("common.retry")}
          </BookButton>
        </div>
      ) : machine ? (
        <div className="flex flex-col gap-3">
          <StatusFlowView currentStatus={machine.current_status} />

          {canForwardStatus &&
            machine.forward_targets.map((target) => (
              <ForwardActionCard
                key={target}
                target={target}
                acting={acting}
                onForward={handleForward}
              />
            ))}

          {canRollbackStatus && (
            <RollbackCard
              targets={machine.rollback_targets}
              target={rollbackTarget}
              preview={rollbackPreview}
              acting={acting}
              onPreview={handleRollbackPreview}
              onCancel={clearRollbackPreview}
              onConfirm={handleConfirmRollback}
            />
          )}

          <StatusHistoryPanel history={machine.history} />
        </div>
      ) : (
        <div className="a4n-flat-surface p-3 text-xs text-danger">{t("statusMachine.loadFailed")}</div>
      )}
    </Modal>
  );
};
