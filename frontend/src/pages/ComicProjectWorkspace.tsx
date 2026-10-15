import React, { useCallback, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { GitCompare, PanelsTopLeft, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import {
  canEditProjectMaterials,
  getProject,
  listSourceSnapshotChapterIndex,
  type NovelProject,
  type SourceSnapshotChapterIndex,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { isPositiveSafeInteger } from "../utils/numberRange";
import { useToast } from "../components/feedback/Toast";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { BackButton } from "../components/ui/BackButton";
import { DerivedProjectHeader } from "../components/shell/DerivedProjectHeader";
import { MangaTab } from "./novel-detail/MangaTab";
import { EmptyHint } from "./novel-detail/shared";
import { SourceSyncDialog } from "./derived/SourceSyncDialog";
import { useI18n } from "../i18n";
import { useNovelDetailDirtyRegistry } from "./novel-detail/novelDetailDirty";
import { confirmDiscardMediaChanges } from "../components/feedback/UnsavedChangesGuard";
import { UnsavedNavigationGuard } from "../components/navigation/UnsavedNavigationGuard";
import { getProjectLifecycleBlockedReason } from "../utils/projectStatus";
import { useProjectLifecycleSync } from "../hooks/useProjectLifecycleSync";

export const ComicProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const translatorRef = useRef(t);
  translatorRef.current = t;
  const projectId = Number(id);

  const [project, setProject] = useState<NovelProject | null>(null);
  const [chapters, setChapters] = useState<SourceSnapshotChapterIndex[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [sourceSyncOpen, setSourceSyncOpen] = useState(false);
  const [workspaceDirty, setWorkspaceDirty] = useState(false);
  const refreshRequestRef = useRef(0);

  const refresh = useCallback(async (options: { silent?: boolean } = {}): Promise<boolean> => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    setLoadError(null);
    if (!isPositiveSafeInteger(projectId)) {
      const message = translatorRef.current("comic.projectInvalid");
      setLoadError(message);
      setProject(null);
      setChapters([]);
      setLoading(false);
      return false;
    }
    if (!options.silent) setLoading(true);
    try {
      const item = await getProject(projectId);
      if (refreshRequestRef.current !== requestId) return false;
      if (item.project_kind !== "comic") {
        addToast(translatorRef.current("comic.notComicProject"), "error");
        navigate("/", { replace: true });
        return false;
      }
      if (refreshRequestRef.current !== requestId) return false;
      setProject(item);
      if (item.source_mode === "derived") {
        try {
          const nextChapters = await listSourceSnapshotChapterIndex(projectId);
          if (refreshRequestRef.current !== requestId) return false;
          setChapters(nextChapters);
        } catch (error) {
          const message = extractErrorMessage(
            error,
            translatorRef.current("comic.loadProjectFailed"),
          );
          setLoadError(message);
          setChapters([]);
          return false;
        }
      } else {
        setChapters([]);
      }
      return true;
    } catch (err) {
      if (refreshRequestRef.current === requestId) {
        const message = extractErrorMessage(err, translatorRef.current("comic.loadProjectFailed"));
        setLoadError(message);
        setProject(null);
        setChapters([]);
      }
      return false;
    } finally {
      if (refreshRequestRef.current === requestId) {
        setLoading(false);
      }
    }
  }, [addToast, navigate, projectId]);

  const handleLifecycleGateChange = useCallback((
    nextProject: NovelProject,
    previousGate: NovelProject["lifecycle_gate"] | null | undefined,
    previousContentRevision: number | undefined,
  ) => {
    setProject(nextProject);
    if (
      (previousGate !== "active" && nextProject.lifecycle_gate === "active")
      || (previousContentRevision != null && previousContentRevision !== nextProject.content_revision)
    ) {
      void refresh({ silent: true });
    }
  }, [refresh]);

  useProjectLifecycleSync(
    projectId,
    project?.lifecycle_gate,
    handleLifecycleGateChange,
    {
      enabled: project != null && !loading,
      contentRevision: project?.content_revision,
      updatedAt: project?.updated_at,
      onReadError: () => { void refresh({ silent: true }); },
    },
  );

  useLayoutEffect(() => {
    refreshRequestRef.current += 1;
    setProject(null);
    setChapters([]);
    setSourceSyncOpen(false);
    void refresh();
    return () => {
      refreshRequestRef.current += 1;
    };
  }, [refresh]);

  const imageProjectIds = useMemo(() => {
    if (!project) return [];
    return project.root_project_id ? [project.id, project.root_project_id] : [project.id];
  }, [project]);
  const lifecycleBlockedReason = project
    ? getProjectLifecycleBlockedReason(project, t)
    : null;
  const canEditProject = project ? canEditProjectMaterials(project) : false;

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <UnsavedNavigationGuard dirty={workspaceDirty} name={t("comic.workspace")} action={t("common.leavePage")} />
      <DerivedProjectHeader
        leading={<BackButton iconOnly title={t("a11y.back")} onClick={() => navigate("/")} />}
        icon={<PanelsTopLeft size={15} />}
        title={project?.title || t("novel.comicProject")}
        actions={
          <>
            {canEditProject && project?.source_mode === "derived" ? (
              <BookIconButton
                label={t("comic.syncSource")}
                onClick={() => { void (async () => { if (await confirmDiscardMediaChanges(workspaceDirty || dirtyRegistry.hasDirty(), t)) setSourceSyncOpen(true); })(); }}
                disabled={!project}
                variant="secondary"
                size="sm"
              >
                <GitCompare size={14} />
              </BookIconButton>
            ) : null}
          </>
        }
      />

      {lifecycleBlockedReason ? (
        <div role="status" className="shrink-0 border-b border-warning/30 bg-warning-bg px-4 py-2 text-small text-book-text-main">
          {lifecycleBlockedReason}
        </div>
      ) : null}

      {project ? (
        project.source_mode === "derived" ? <SourceSyncDialog
          open={sourceSyncOpen}
          projectId={project.id}
          projectTitle={project.title}
          sourceSyncStatus={project.source_sync_status}
          canEditSourceSync={canEditProject}
          onClose={() => setSourceSyncOpen(false)}
          onChanged={() => refresh({ silent: true })}
        /> : null
      ) : null}

      <section aria-label={t("comic.workspace")} className="flex min-h-0 flex-1 flex-col overflow-hidden a4n-page-container">
        {loadError ? (
          <div role="alert" className="mb-3 flex flex-wrap items-center gap-3 text-sm text-error">
            <p className="min-w-0 flex-1">{loadError}</p>
            <BookButton variant="secondary" size="sm" onClick={() => void refresh()} disabled={loading}>
              <RefreshCw size={13} /> {t("common.retry")}
            </BookButton>
          </div>
        ) : null}
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            {t("comic.loadingProject")}
          </div>
        ) : loadError ? null : project && !canEditProject ? (
          <EmptyHint text={lifecycleBlockedReason ?? t("comic.noEditPermission")} />
        ) : project ? (
          <MangaTab
            key={project.id}
            projectId={project.id}
            canEditManga={canEditProject}
            onSnapshotRefresh={() => {
              void (async () => {
                if (await confirmDiscardMediaChanges(workspaceDirty || dirtyRegistry.hasDirty(), t)) {
                  await refresh();
                }
              })();
            }}
            standalone={project.source_mode === "standalone"}
            onDirtyChange={setWorkspaceDirty}
            snapshotChapters={chapters}
            assetProjectId={project.root_project_id ?? project.id}
            imageProjectIds={imageProjectIds}
          />
        ) : null}
      </section>
    </div>
  );
};
