import React, { useCallback, useEffect, useRef, useState } from "react";
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
} from "../api/project";
import { PROJECT_STATUS_LABELS } from "../api/generated/host";
import { useToast } from "../components/feedback/Toast";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { Modal } from "../components/ui/Modal";
import { ForwardActionCard } from "./status-machine-dialog/ForwardActionCard";
import { RollbackCard } from "./status-machine-dialog/RollbackCard";
import { StatusFlowView } from "./status-machine-dialog/StatusFlowView";
import { StatusHistoryPanel } from "./status-machine-dialog/StatusHistoryPanel";

interface StatusMachineDialogProps {
  open: boolean;
  onClose: () => void;
  project: NovelProject;
  canForwardStatus: boolean;
  canRollbackStatus: boolean;
  onMutated: () => Promise<void> | void;
}

export const StatusMachineDialog: React.FC<StatusMachineDialogProps> = ({
  open,
  onClose,
  project,
  canForwardStatus,
  canRollbackStatus,
  onMutated,
}) => {
  const addToast = useToast((state) => state.addToast);
  const [machine, setMachine] = useState<ProjectStatusMachineSummary | null>(null);
  const [loadingMachine, setLoadingMachine] = useState(false);
  const [acting, setActing] = useState<string | null>(null);
  const [rollbackTarget, setRollbackTarget] = useState<ProjectStatus | null>(null);
  const [rollbackPreview, setRollbackPreview] = useState<ProjectRollbackPreview | null>(null);
  const mountedRef = useRef(false);
  const scopeRef = useRef("closed");
  const activeProjectIdRef = useRef(project.id);
  const machineSeqRef = useRef(0);
  const rollbackPreviewSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const actingRef = useRef<string | null>(null);
  const rollbackPreviewInFlightKeyRef = useRef<string | null>(null);
  const scopeKey = open ? String(project.id) : "closed";
  const actionBusy = acting !== null;

  activeProjectIdRef.current = project.id;

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

  const loadMachine = useCallback(async () => {
    const requestScope = scopeKey;
    const requestSeq = machineSeqRef.current + 1;
    machineSeqRef.current = requestSeq;
    setLoadingMachine(true);
    try {
      const data = await getProjectStatusMachine(project.id);
      if (!isCurrentRequest(requestScope, machineSeqRef, requestSeq)) return;
      setMachine(data);
    } catch (err) {
      if (isCurrentRequest(requestScope, machineSeqRef, requestSeq)) {
        setMachine(null);
        addToast(extractErrorMessage(err, "加载项目状态机失败"), "error");
      }
    } finally {
      if (isCurrentRequest(requestScope, machineSeqRef, requestSeq)) {
        setLoadingMachine(false);
      }
    }
  }, [addToast, isCurrentRequest, project.id, scopeKey]);

  useEffect(() => {
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

  useEffect(() => {
    scopeRef.current = scopeKey;
    machineSeqRef.current += 1;
    rollbackPreviewSeqRef.current += 1;
    actionSeqRef.current += 1;
    actingRef.current = null;
    rollbackPreviewInFlightKeyRef.current = null;
    setActing(null);
    setRollbackTarget(null);
    setRollbackPreview(null);
    setMachine(null);
    if (!open) {
      setLoadingMachine(false);
      return;
    }
    void loadMachine();
  }, [loadMachine, open, scopeKey]);

  const executeStatusAction = useCallback(
    async (key: string, action: () => Promise<unknown>, successMessage: string) => {
      if (actingRef.current != null) return false;
      const requestProjectId = project.id;
      const requestSeq = actionSeqRef.current + 1;
      actionSeqRef.current = requestSeq;
      actingRef.current = key;
      setActing(key);
      try {
        await action();
        if (!isCurrentMutation(requestProjectId, requestSeq)) return false;
        addToast(successMessage, "success");
        await onMutated();
        if (!isCurrentMutation(requestProjectId, requestSeq)) return false;
        if (open) await loadMachine();
        return true;
      } catch (err) {
        if (isCurrentMutation(requestProjectId, requestSeq)) {
          addToast(extractErrorMessage(err, "操作失败"), "error");
        }
        return false;
      } finally {
        if (actionSeqRef.current === requestSeq) actingRef.current = null;
        if (isCurrentMutation(requestProjectId, requestSeq)) setActing(null);
      }
    },
    [addToast, isCurrentMutation, loadMachine, onMutated, open, project.id],
  );

  const handleForward = useCallback(
    async (target: ProjectStatus) => {
      if (!canForwardStatus) {
        addToast("当前账号没有推进项目状态的权限", "error");
        return;
      }
      await executeStatusAction(
        `forward-${target}`,
        () => forwardProjectStatus(project.id, target),
        `已推进至「${PROJECT_STATUS_LABELS[target]}」`,
      );
    },
    [addToast, canForwardStatus, executeStatusAction, project.id],
  );

  const handleRollbackPreview = useCallback(
    async (target: ProjectStatus) => {
      if (!canRollbackStatus || !machine) {
        addToast("当前账号没有回滚项目状态的权限", "error");
        return;
      }
      const requestScope = scopeKey;
      const requestKey = `${requestScope}:${machine.current_status}:${target}`;
      if (rollbackPreviewInFlightKeyRef.current === requestKey) return;
      const requestSeq = rollbackPreviewSeqRef.current + 1;
      rollbackPreviewSeqRef.current = requestSeq;
      rollbackPreviewInFlightKeyRef.current = requestKey;
      setRollbackTarget(target);
      setRollbackPreview(null);
      try {
        const data = await previewRollback(project.id, target);
        if (!isCurrentRequest(requestScope, rollbackPreviewSeqRef, requestSeq)) return;
        setRollbackPreview(data);
      } catch (err) {
        if (isCurrentRequest(requestScope, rollbackPreviewSeqRef, requestSeq)) {
          addToast(extractErrorMessage(err, "预览失败"), "error");
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
    [addToast, canRollbackStatus, isCurrentRequest, machine, project.id, scopeKey],
  );

  const clearRollbackPreview = useCallback(() => {
    rollbackPreviewSeqRef.current += 1;
    rollbackPreviewInFlightKeyRef.current = null;
    setRollbackTarget(null);
    setRollbackPreview(null);
  }, []);

  const handleConfirmRollback = useCallback(async () => {
    if (!canRollbackStatus) {
      addToast("当前账号没有回滚项目状态的权限", "error");
      return;
    }
    if (!rollbackTarget) return;
    const requestProjectId = project.id;
    const completed = await executeStatusAction(
      `rollback-${rollbackTarget}`,
      () => rollbackProjectStatus(project.id, rollbackTarget),
      `已回滚至「${PROJECT_STATUS_LABELS[rollbackTarget]}」`,
    );
    if (completed && mountedRef.current && activeProjectIdRef.current === requestProjectId) {
      clearRollbackPreview();
    }
  }, [
    addToast,
    canRollbackStatus,
    clearRollbackPreview,
    executeStatusAction,
    project.id,
    rollbackTarget,
  ]);

  return (
    <Modal
      isOpen={open}
      onClose={() => {
        if (!actionBusy) onClose();
      }}
      title="项目阶段"
      subtitle={project.title}
      maxWidthClassName="max-w-xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!actionBusy}
      closeOnEscape={!actionBusy}
      showCloseButton={!actionBusy}
    >
      {loadingMachine && !machine ? (
        <div className="a4n-flat-surface inline-flex items-center gap-2 p-3 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> 正在加载项目状态机…
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
        <div className="a4n-flat-surface p-3 text-xs text-danger">项目状态机加载失败。</div>
      )}
    </Modal>
  );
};
