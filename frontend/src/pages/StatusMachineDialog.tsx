import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { Modal } from "../components/ui/Modal";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import {
  forwardProjectStatus,
  getProjectStatusHistory,
  previewRollback,
  rollbackProjectStatus,
  type ProjectStatusHistory,
  type NovelProject,
  type RollbackPreview,
} from "../api/project";
import { listChapterOutlines, listParts } from "../api/outline";
import type { ProjectStatus } from "../utils/projectStatus";
import { ForwardActionCard } from "./status-machine-dialog/ForwardActionCard";
import { RollbackCard } from "./status-machine-dialog/RollbackCard";
import { StatusFlowView } from "./status-machine-dialog/StatusFlowView";
import { StatusHistoryPanel } from "./status-machine-dialog/StatusHistoryPanel";
import {
  getForwardTarget,
  getRollbackTargets,
  statusLabel,
} from "./status-machine-dialog/statusMachineUtils";

/**
 * 项目状态机操作对话框
 *
 * 对应 a4n-core/src/state/transitions.rs 中定义的合法转换：
 *   draft → blueprint_ready → (part_outlines_ready | chapter_outlines_ready)
 *                            → chapter_outlines_ready → writing → completed
 *
 * 暴露 4 个后端命令：
 *  - forward_project_status(target)
 *  - rollback_project_status(target)
 *  - get_project_status_history()
 *  - preview_rollback(target_status)  —— 回滚前预览将丢失的数据
 */

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
  const [history, setHistory] = useState<ProjectStatusHistory[]>([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [acting, setActing] = useState<string | null>(null);
  const [rollbackTarget, setRollbackTarget] = useState<ProjectStatus | null>(null);
  const [rollbackPreview, setRollbackPreview] = useState<RollbackPreview | null>(null);
  const [forwardArtifacts, setForwardArtifacts] = useState<{
    partOutlines: number;
    chapterOutlines: number;
  } | null>(null);
  const [loadingForwardArtifacts, setLoadingForwardArtifacts] = useState(false);
  const mountedRef = useRef(false);
  const scopeRef = useRef("closed");
  const activeProjectIdRef = useRef(project.id);
  const historySeqRef = useRef(0);
  const forwardArtifactsSeqRef = useRef(0);
  const rollbackPreviewSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const actingRef = useRef<string | null>(null);
  const rollbackPreviewInFlightKeyRef = useRef<string | null>(null);

  const forwardTarget = getForwardTarget(project.status, forwardArtifacts);
  const rollbackTargets = getRollbackTargets(project.status);
  const scopeKey = open ? `${project.id}:${project.status}` : "closed";

  activeProjectIdRef.current = project.id;

  const isCurrentScope = useCallback(
    (requestScope: string) => mountedRef.current && scopeRef.current === requestScope,
    [],
  );

  const isCurrentAction = useCallback(
    (
      requestScope: string,
      ref: React.MutableRefObject<number>,
      requestSeq: number,
    ) => isCurrentScope(requestScope) && ref.current === requestSeq,
    [isCurrentScope],
  );
  const isCurrentStatusMutation = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      mountedRef.current &&
      activeProjectIdRef.current === requestProjectId &&
      actionSeqRef.current === requestSeq,
    [],
  );

  const loadHistory = useCallback(async () => {
    const requestScope = scopeKey;
    const requestSeq = historySeqRef.current + 1;
    historySeqRef.current = requestSeq;
    setLoadingHistory(true);
    try {
      const data = await getProjectStatusHistory(project.id);
      if (!isCurrentAction(requestScope, historySeqRef, requestSeq)) return;
      setHistory(data);
    } catch (err) {
      if (isCurrentAction(requestScope, historySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "加载历史失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, historySeqRef, requestSeq)) {
        setLoadingHistory(false);
      }
    }
  }, [addToast, isCurrentAction, project.id, scopeKey]);

  const loadForwardArtifacts = useCallback(async () => {
    const requestScope = scopeKey;
    const requestSeq = forwardArtifactsSeqRef.current + 1;
    forwardArtifactsSeqRef.current = requestSeq;
    if (!canForwardStatus) {
      setForwardArtifacts(null);
      setLoadingForwardArtifacts(false);
      return;
    }
    if (project.status !== "blueprint_ready") {
      if (isCurrentAction(requestScope, forwardArtifactsSeqRef, requestSeq)) {
        setForwardArtifacts(null);
        setLoadingForwardArtifacts(false);
      }
      return;
    }
    setLoadingForwardArtifacts(true);
    try {
      const [parts, outlines] = await Promise.all([
        listParts(project.id),
        listChapterOutlines(project.id),
      ]);
      if (!isCurrentAction(requestScope, forwardArtifactsSeqRef, requestSeq)) return;
      setForwardArtifacts({
        partOutlines: parts.length,
        chapterOutlines: outlines.length,
      });
    } catch (err) {
      if (isCurrentAction(requestScope, forwardArtifactsSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "检查下一阶段条件失败"), "error");
        setForwardArtifacts(null);
      }
    } finally {
      if (isCurrentAction(requestScope, forwardArtifactsSeqRef, requestSeq)) {
        setLoadingForwardArtifacts(false);
      }
    }
  }, [addToast, canForwardStatus, isCurrentAction, project.id, project.status, scopeKey]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      historySeqRef.current += 1;
      forwardArtifactsSeqRef.current += 1;
      rollbackPreviewSeqRef.current += 1;
      actionSeqRef.current += 1;
      actingRef.current = null;
      rollbackPreviewInFlightKeyRef.current = null;
    };
  }, []);

  useEffect(() => {
    actionSeqRef.current += 1;
    actingRef.current = null;
    setActing(null);
  }, [project.id]);

  useEffect(() => {
    scopeRef.current = scopeKey;
    historySeqRef.current += 1;
    forwardArtifactsSeqRef.current += 1;
    rollbackPreviewSeqRef.current += 1;
    rollbackPreviewInFlightKeyRef.current = null;
    setRollbackTarget(null);
    setRollbackPreview(null);
    setForwardArtifacts(null);
    if (!open) {
      setHistory([]);
      setLoadingHistory(false);
      setLoadingForwardArtifacts(false);
      return;
    }
    void loadHistory();
    void loadForwardArtifacts();
  }, [open, scopeKey, loadForwardArtifacts, loadHistory]);

  const executeStatusAction = useCallback(
    async (key: string, fn: () => Promise<unknown>, ok: string) => {
      if (actingRef.current != null) return false;
      const requestProjectId = project.id;
      const requestSeq = actionSeqRef.current + 1;
      actionSeqRef.current = requestSeq;
      actingRef.current = key;
      setActing(key);
      try {
        await fn();
        if (!isCurrentStatusMutation(requestProjectId, requestSeq)) return false;
        addToast(ok, "success");
        if (open) {
          await loadHistory();
          if (!isCurrentStatusMutation(requestProjectId, requestSeq)) return false;
        }
        await onMutated();
        return true;
      } catch (err) {
        if (isCurrentStatusMutation(requestProjectId, requestSeq)) {
          addToast(extractErrorMessage(err, "操作失败"), "error");
        }
        return false;
      } finally {
        if (actionSeqRef.current === requestSeq) actingRef.current = null;
        if (isCurrentStatusMutation(requestProjectId, requestSeq)) {
          setActing(null);
        }
      }
    },
    [addToast, isCurrentStatusMutation, loadHistory, onMutated, open, project.id],
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
        `已推进至「${statusLabel(target)}」`,
      );
    },
    [addToast, canForwardStatus, executeStatusAction, project.id],
  );

  const handleRollbackPreview = useCallback(
    async (target: ProjectStatus) => {
      if (!canRollbackStatus) {
        addToast("当前账号没有回滚项目状态的权限", "error");
        return;
      }
      const requestScope = scopeKey;
      const requestKey = `${requestScope}:${target}`;
      if (rollbackPreviewInFlightKeyRef.current === requestKey) return;
      const requestSeq = rollbackPreviewSeqRef.current + 1;
      rollbackPreviewSeqRef.current = requestSeq;
      rollbackPreviewInFlightKeyRef.current = requestKey;
      setRollbackTarget(target);
      setRollbackPreview(null);
      try {
        const data = await previewRollback(project.id, target);
        if (!isCurrentAction(requestScope, rollbackPreviewSeqRef, requestSeq)) return;
        setRollbackPreview(data);
      } catch (err) {
        if (isCurrentAction(requestScope, rollbackPreviewSeqRef, requestSeq)) {
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
    [addToast, canRollbackStatus, isCurrentAction, project.id, scopeKey],
  );

  const clearRollbackPreview = useCallback(() => {
    rollbackPreviewSeqRef.current += 1;
    rollbackPreviewInFlightKeyRef.current = null;
    setRollbackTarget(null);
    setRollbackPreview(null);
  }, []);

  const handleConfirmRollback = useCallback(
    async () => {
      if (!canRollbackStatus) {
        addToast("当前账号没有回滚项目状态的权限", "error");
        return;
      }
      if (!rollbackTarget) return;
      const requestProjectId = project.id;
      const completed = await executeStatusAction(
        `rollback-${rollbackTarget}`,
        () => rollbackProjectStatus(project.id, rollbackTarget),
        `已回滚至「${statusLabel(rollbackTarget)}」`,
      );
      if (completed && mountedRef.current && activeProjectIdRef.current === requestProjectId) {
        clearRollbackPreview();
      }
    },
    [
      addToast,
      canRollbackStatus,
      clearRollbackPreview,
      executeStatusAction,
      project.id,
      rollbackTarget,
    ],
  );

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title="项目状态机"
      maxWidthClassName="max-w-2xl"
    >
      <div className="flex flex-col gap-5">
        <StatusFlowView currentStatus={project.status} />

        {loadingForwardArtifacts && (
          <div className="pyqt-card p-3 text-xs text-book-text-muted inline-flex items-center gap-2">
            <Loader2 size={12} className="animate-spin" /> 正在检查下一阶段条件…
          </div>
        )}

        {!loadingForwardArtifacts && forwardTarget && canForwardStatus && (
          <ForwardActionCard target={forwardTarget} acting={acting} onForward={handleForward} />
        )}

        {canRollbackStatus && (
          <RollbackCard
            targets={rollbackTargets}
            target={rollbackTarget}
            preview={rollbackPreview}
            acting={acting}
            onPreview={handleRollbackPreview}
            onCancel={clearRollbackPreview}
            onConfirm={handleConfirmRollback}
          />
        )}

        <StatusHistoryPanel history={history} loading={loadingHistory} />
      </div>
    </Modal>
  );
};
