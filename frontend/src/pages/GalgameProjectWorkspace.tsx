import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Gamepad2, GitCompare, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import {
  canEditProjectMaterials,
  getProject,
  listSourceSnapshotChapters,
  resetProjectGeneratedData,
  type NovelProject,
  type SourceSnapshotChapterSummary,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { useToast } from "../components/feedback/Toast";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { BackButton } from "../components/ui/BackButton";
import { Dropdown } from "../components/ui/Dropdown";
import { DerivedProjectHeader } from "../components/shell/DerivedProjectHeader";
import { SourceSyncDialog } from "./derived/SourceSyncDialog";
import { GalgameTab } from "./novel-detail/GalgameTab";
import { getDerivedProjectSubtitle } from "../utils/projectRouting";
import { getProjectLifecycleBlockedReason } from "../utils/projectStatus";
import { useI18n } from "../i18n";

export const GalgameProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { t } = useI18n();
  const translatorRef = useRef(t);
  translatorRef.current = t;
  const projectId = Number(id);

  const subtitleLabels = useMemo(
    () => ({
      fallback: t("galgame.workspace"),
      sourceProject: (title: string) => t("novel.sourceProject", { title }),
      chapterScope: (start: number, end: number) =>
        start === end
          ? t("novel.chapterScopeSingle", { start })
          : t("novel.chapterScopeRange", { start, end }),
      sourceSyncStatus: (value: string | null | undefined) => {
        switch (String(value ?? "").trim().toLowerCase()) {
          case "current": return t("novel.sourceSyncCurrent");
          case "root_updated": return t("novel.sourceSyncRootUpdated");
          case "conflict": return t("novel.sourceSyncConflict");
          case "detached": return t("novel.sourceSyncDetached");
          default: return value?.trim() || t("novel.sourceSyncUnknown");
        }
      },
    }),
    [t],
  );

  const [project, setProject] = useState<NovelProject | null>(null);
  const [chapters, setChapters] = useState<SourceSnapshotChapterSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [sourceSyncOpen, setSourceSyncOpen] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [workspaceVersion, setWorkspaceVersion] = useState(0);
  const refreshRequestRef = useRef(0);
  const resetBusyRef = useRef(false);

  const refresh = useCallback(async () => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    setLoadError(null);
    if (!Number.isFinite(projectId) || projectId <= 0) {
      const message = translatorRef.current("galgame.projectInvalid");
      setLoadError(message);
      addToast(message, "error");
      setProject(null);
      setChapters([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const [projectResult, chaptersResult] = await Promise.allSettled([
        getProject(projectId),
        listSourceSnapshotChapters(projectId),
      ]);
      if (projectResult.status === "rejected") throw projectResult.reason;
      const item = projectResult.value;
      if (refreshRequestRef.current !== requestId) return;
      if (item.project_kind !== "galgame") {
        addToast(translatorRef.current("galgame.notProject"), "error");
        navigate("/", { replace: true });
        return;
      }
      if (refreshRequestRef.current !== requestId) return;
      setProject(item);
      if (chaptersResult.status === "fulfilled") {
        setChapters(chaptersResult.value);
      } else {
        const message = extractErrorMessage(
          chaptersResult.reason,
          translatorRef.current("galgame.loadFailed"),
        );
        setLoadError(message);
        addToast(message, "error");
      }
    } catch (err) {
      if (refreshRequestRef.current === requestId) {
        const message = extractErrorMessage(err, translatorRef.current("galgame.loadFailed"));
        setLoadError(message);
        addToast(message, "error");
      }
    } finally {
      if (refreshRequestRef.current === requestId) setLoading(false);
    }
  }, [addToast, navigate, projectId]);

  useEffect(() => {
    refreshRequestRef.current += 1;
    setProject(null);
    setChapters([]);
    setSourceSyncOpen(false);
    void refresh();
    return () => {
      refreshRequestRef.current += 1;
    };
  }, [refresh]);

  const lifecycleBlockedReason = project
    ? getProjectLifecycleBlockedReason(project, t)
    : null;
  const canEditProject = project ? canEditProjectMaterials(project) : false;
  const canResetProject = project?.my_role === "owner" && !lifecycleBlockedReason;
  const resetInProgress = resetting || project?.lifecycle_gate === "resetting";

  useEffect(() => {
    if (project?.lifecycle_gate !== "resetting") return;
    let checking = false;
    let errorReported = false;
    const timer = window.setInterval(() => {
      if (checking) return;
      checking = true;
      void getProject(projectId)
        .then(async (nextProject) => {
          if (nextProject.lifecycle_gate !== "active") return;
          window.clearInterval(timer);
          await refresh();
          setWorkspaceVersion((value) => value + 1);
          addToast(t("galgame.resetCompleted"), "success");
        })
        .catch((error) => {
          if (errorReported) return;
          errorReported = true;
          addToast(extractErrorMessage(error, t("galgame.resetQueryFailed")), "error");
        })
        .finally(() => {
          checking = false;
        });
    }, 1500);
    return () => window.clearInterval(timer);
  }, [addToast, project?.lifecycle_gate, projectId, refresh, t]);

  const handleResetGeneratedData = useCallback(async () => {
    if (!project || resetBusyRef.current) return;
    const blockedReason = getProjectLifecycleBlockedReason(project, t);
    if (blockedReason) {
      addToast(blockedReason, "info");
      return;
    }
    resetBusyRef.current = true;
    const ok = await confirmDialog({
      title: t("galgame.resetTitle"),
      message: t("galgame.resetMessage"),
      confirmText: t("common.reset"),
      cancelText: t("common.cancel"),
      dialogType: "danger",
    });
    if (!ok) {
      resetBusyRef.current = false;
      return;
    }

    setResetting(true);
    setSourceSyncOpen(false);
    try {
      const response = await resetProjectGeneratedData(project.id);
      const warnings = [...response.warnings];
      const formalResetCommitted = response.database_committed;
      let nextProject = response.result?.project ?? null;
      if (!nextProject) {
        try {
          nextProject = await getProject(project.id);
        } catch (error) {
          warnings.push(extractErrorMessage(error, t("galgame.refreshStatusFailed")));
        }
      }
      if (nextProject) {
        setProject(nextProject);
      } else if (response.status === "pending") {
        setProject((current) =>
          current ? { ...current, lifecycle_gate: "resetting" } : current,
        );
      }
      if (formalResetCommitted) {
        try {
          setChapters(await listSourceSnapshotChapters(project.id));
        } catch (error) {
          warnings.push(extractErrorMessage(error, t("galgame.refreshSourceChaptersFailed")));
        }
        setWorkspaceVersion((value) => value + 1);
      }

      const warningText = warnings.length > 0
        ? `${t("common.summarySeparator")}${warnings.join(t("common.summarySeparator"))}`
        : "";
      if (response.status === "pending") {
        addToast(
          formalResetCommitted
            ? t("galgame.resetPendingCommitted", { warnings: warningText })
            : t("galgame.resetPending", { warnings: warningText }),
          "info",
        );
      } else if (response.result) {
        addToast(
          t("galgame.resetSummary", {
            scripts: response.result.deleted_galgame_scripts,
            scenes: response.result.deleted_galgame_scenes,
            lines: response.result.deleted_galgame_lines,
            specs: response.result.deleted_generation_specs,
            media: response.result.deleted_media_assets,
            warnings: warningText,
          }),
          warnings.length > 0 ? "info" : "success",
        );
      } else {
        addToast(
          t("galgame.resetDone", { warnings: warningText }),
          warnings.length > 0 ? "info" : "success",
        );
      }
    } catch (err) {
      addToast(extractErrorMessage(err, t("errors.resetGenerationFailed")), "error");
    } finally {
      resetBusyRef.current = false;
      setResetting(false);
    }
  }, [addToast, project, t]);

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <DerivedProjectHeader
        leading={<BackButton iconOnly title={t("a11y.back")} onClick={() => navigate("/")} />}
        icon={<Gamepad2 size={15} />}
        title={project?.title || t("novel.galgameProject")}
        subtitle={
          project
            ? getDerivedProjectSubtitle(project, subtitleLabels)
            : loading ? t("video.sourceLoading") : null
        }
        actions={
          <>
            {canEditProject ? (
              <BookIconButton
                label={t("galgame.syncSource")}
                onClick={() => setSourceSyncOpen(true)}
                disabled={!project || resetInProgress}
                variant="secondary"
                size="sm"
              >
                <GitCompare size={14} />
              </BookIconButton>
            ) : null}
            {canResetProject && !resetInProgress ? (
              <Dropdown
                items={[
                  {
                    label: t("galgame.resetData"),
                    onClick: handleResetGeneratedData,
                    danger: true,
                    icon: <Trash2 size={13} />,
                  },
                ]}
              />
            ) : null}
            {project?.my_role === "owner" && resetInProgress ? (
              <BookButton
                variant="secondary"
                size="sm"
                disabled
              >
                <LoadingSpinner size="sm" />
                {t("galgame.resetFinishing")}
              </BookButton>
            ) : null}
          </>
        }
      />

      {lifecycleBlockedReason ? (
        <div
          role="status"
          className="shrink-0 border-b border-warning/30 bg-warning-bg px-4 py-2 text-small text-book-text-main"
        >
          {lifecycleBlockedReason}
        </div>
      ) : null}

      {project ? (
        <SourceSyncDialog
          open={sourceSyncOpen}
          projectId={project.id}
          projectTitle={project.title}
          sourceSyncStatus={project.source_sync_status}
          canEditSourceSync={canEditProject}
          onClose={() => setSourceSyncOpen(false)}
          onChanged={refresh}
        />
      ) : null}

      <section aria-label={t("galgame.workspace")} className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden a4n-page-container lg:overflow-hidden custom-scrollbar">
        {loadError ? <p role="alert" className="mb-3 text-sm text-error">{loadError}</p> : null}
        {loading && !project ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            {t("galgame.loadingProject")}
          </div>
        ) : project ? (
          <GalgameTab
            key={workspaceVersion}
            projectId={project.id}
            canEditGalgame={canEditProject}
            snapshotChapters={chapters}
          />
        ) : null}
      </section>
    </div>
  );
};
