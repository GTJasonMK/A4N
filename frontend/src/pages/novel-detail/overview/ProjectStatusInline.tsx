import React, { useEffect, useRef, useState } from "react";
import { GitBranch } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import { forwardProjectStatus, type NovelProject } from "../../../api/project";
import { listChapterOutlines, listParts } from "../../../api/outline";
import {
  getProjectForwardStatusHint,
  getProjectForwardTarget,
  getProjectStatusText,
  type ProjectForwardArtifacts,
} from "../../../utils/projectStatus";

interface ProjectStatusInlineProps {
  project: NovelProject;
  canForwardStatus: boolean;
  onMutated: () => Promise<void>;
}

export const ProjectStatusInline: React.FC<ProjectStatusInlineProps> = ({
  project,
  canForwardStatus,
  onMutated,
}) => {
  const { addToast } = useToast();
  const [busy, setBusy] = useState(false);
  const [forwardArtifacts, setForwardArtifacts] = useState<ProjectForwardArtifacts | null>(null);
  const [checkingForward, setCheckingForward] = useState(false);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(project.id);
  const advanceSeqRef = useRef(0);
  const busyRef = useRef(false);
  const currentText = getProjectStatusText(project.status);
  const next = canForwardStatus ? getProjectForwardTarget(project.status, forwardArtifacts) : null;
  const nextText = next ? getProjectStatusText(next) : null;
  const nextHint = getProjectForwardStatusHint(project.status, next);
  const terminal = project.status === "completed";
  const idleText = !canForwardStatus
    ? "当前账号仅可查看阶段状态"
    : terminal
      ? "已到达终态，无需推进"
      : "当前阶段尚不能推进";

  activeProjectIdRef.current = project.id;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      advanceSeqRef.current += 1;
      busyRef.current = false;
    };
  }, []);

  useEffect(() => {
    advanceSeqRef.current += 1;
    busyRef.current = false;
    setBusy(false);
  }, [project.id, project.status]);

  const isCurrentAdvance = (requestProjectId: number, requestSeq: number) =>
    mountedRef.current &&
    activeProjectIdRef.current === requestProjectId &&
    advanceSeqRef.current === requestSeq;

  useEffect(() => {
    let cancelled = false;
    if (!canForwardStatus || project.status !== "blueprint_ready") {
      setForwardArtifacts(null);
      setCheckingForward(false);
      return;
    }
    setCheckingForward(true);
    setForwardArtifacts(null);
    Promise.all([listParts(project.id), listChapterOutlines(project.id)])
      .then(([parts, outlines]) => {
        if (cancelled) return;
        setForwardArtifacts({
          partOutlines: parts.length,
          chapterOutlines: outlines.length,
        });
      })
      .catch((err) => {
        if (cancelled) return;
        addToast(extractErrorMessage(err, "检查下一阶段条件失败"), "error");
      })
      .finally(() => {
        if (!cancelled) setCheckingForward(false);
      });
    return () => {
      cancelled = true;
    };
  }, [addToast, canForwardStatus, project.id, project.status]);

  const handleAdvance = async () => {
    if (!canForwardStatus) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    if (!next || busyRef.current) return;
    const requestProjectId = project.id;
    const requestNext = next;
    const requestNextText = nextText ?? getProjectStatusText(requestNext);
    const requestSeq = advanceSeqRef.current + 1;
    advanceSeqRef.current = requestSeq;
    busyRef.current = true;
    setBusy(true);
    const ok = await confirmDialog({
      title: "推进项目状态",
      message: `将项目状态从「${currentText}」推进到「${requestNextText}」？\n\n提示：${nextHint}`,
      confirmText: "推进",
      dialogType: "warning",
    });
    if (!ok) {
      if (advanceSeqRef.current === requestSeq) busyRef.current = false;
      if (isCurrentAdvance(requestProjectId, requestSeq)) setBusy(false);
      return;
    }
    if (!isCurrentAdvance(requestProjectId, requestSeq)) {
      if (advanceSeqRef.current === requestSeq) busyRef.current = false;
      return;
    }
    try {
      await forwardProjectStatus(requestProjectId, requestNext);
      if (!isCurrentAdvance(requestProjectId, requestSeq)) return;
      addToast(`已推进到 ${requestNextText}`, "success");
      await onMutated();
      if (!isCurrentAdvance(requestProjectId, requestSeq)) return;
    } catch (err) {
      if (!isCurrentAdvance(requestProjectId, requestSeq)) return;
      addToast(extractErrorMessage(err, "推进状态失败"), "error");
    } finally {
      if (advanceSeqRef.current === requestSeq) busyRef.current = false;
      if (isCurrentAdvance(requestProjectId, requestSeq)) setBusy(false);
    }
  };

  return (
    <div className="my-3 border-y border-book-border/40 py-3">
      <div className="flex min-w-0 flex-col gap-2 text-xs sm:flex-row sm:items-center sm:gap-3">
        <div className="flex min-w-0 items-center gap-2.5">
          <GitBranch size={14} className="shrink-0 text-book-primary" />
          <div className="flex min-w-0 flex-col">
            <span className="text-small text-book-text-muted">当前阶段</span>
            <span className="text-sm font-semibold text-book-text-main">{currentText}</span>
          </div>
        </div>
        {checkingForward && (
          <span className="inline-flex flex-1 items-center gap-1.5 text-book-text-sub sm:border-l sm:border-book-border/45 sm:pl-3">
            <LoadingSpinner size="sm" /> 正在检查下一步…
          </span>
        )}
        {!checkingForward && canForwardStatus && next && (
          <>
            <div className="flex min-w-0 flex-1 flex-col border-t border-book-border/40 pt-2 sm:border-l sm:border-t-0 sm:pl-3 sm:pt-0">
              <span className="text-book-text-muted text-small">可推进至</span>
              <span className="text-sm font-semibold text-book-primary">{nextText}</span>
            </div>
            <BookButton
              variant="primary"
              onClick={handleAdvance}
              disabled={busy}
              size="sm"
              className="w-full shrink-0 sm:ml-auto sm:w-auto"
              title={`推进到${nextText}：${nextHint}`}
            >
              {busy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 推进中…
                </span>
              ) : (
                "推进阶段"
              )}
            </BookButton>
          </>
        )}
        {!checkingForward && !next && (
          <span className="flex-1 border-t border-book-border/40 pt-2 italic text-book-text-muted sm:border-l sm:border-t-0 sm:pl-3 sm:pt-0">
            {idleText}
          </span>
        )}
      </div>
    </div>
  );
};
