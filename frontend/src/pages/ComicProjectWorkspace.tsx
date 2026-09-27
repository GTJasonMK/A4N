import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { GitCompare, PanelsTopLeft } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import {
  getProject,
  listSourceSnapshotChapters,
  type NovelProject,
  type SourceSnapshotChapterSummary,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { useToast } from "../components/feedback/Toast";
import { BookIconButton } from "../components/ui/BookButton";
import { BackButton } from "../components/ui/BackButton";
import { DerivedProjectHeader } from "../components/shell/DerivedProjectHeader";
import { MangaTab } from "./novel-detail/MangaTab";
import { SourceSyncDialog } from "./derived/SourceSyncDialog";
import { getDerivedProjectSubtitle } from "../utils/projectRouting";
import { useI18n } from "../i18n";

export const ComicProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const translatorRef = useRef(t);
  translatorRef.current = t;
  const projectId = Number(id);

  const subtitleLabels = useMemo(
    () => ({
      fallback: t("comic.projectWorkspace"),
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
  const refreshRequestRef = useRef(0);

  const refresh = useCallback(async () => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    setLoadError(null);
    if (!Number.isFinite(projectId) || projectId <= 0) {
      const message = translatorRef.current("comic.projectInvalid");
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
      if (item.project_kind !== "comic") {
        addToast(translatorRef.current("comic.notComicProject"), "error");
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
          translatorRef.current("comic.loadProjectFailed"),
        );
        setLoadError(message);
        addToast(message, "error");
      }
    } catch (err) {
      if (refreshRequestRef.current === requestId) {
        const message = extractErrorMessage(err, translatorRef.current("comic.loadProjectFailed"));
        setLoadError(message);
        addToast(message, "error");
      }
    } finally {
      if (refreshRequestRef.current === requestId) {
        setLoading(false);
      }
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

  const imageProjectIds = useMemo(() => {
    if (!project) return [];
    return project.root_project_id ? [project.id, project.root_project_id] : [project.id];
  }, [project]);
  const canEditProject =
    project?.my_role === "owner" || project?.my_role === "manager" || project?.my_role === "editor";

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <DerivedProjectHeader
        leading={<BackButton iconOnly title={t("a11y.back")} onClick={() => navigate("/")} />}
        icon={<PanelsTopLeft size={15} />}
        title={project?.title || t("novel.comicProject")}
        subtitle={
          project
            ? getDerivedProjectSubtitle(project, subtitleLabels)
            : loading ? t("video.sourceLoading") : null
        }
        actions={
          <>
            {canEditProject ? (
              <BookIconButton
                label={t("comic.syncSource")}
                onClick={() => setSourceSyncOpen(true)}
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

      <section aria-label={t("comic.workspace")} className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden a4n-page-container lg:overflow-hidden custom-scrollbar">
        {loadError ? <p role="alert" className="mb-3 text-sm text-error">{loadError}</p> : null}
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            {t("comic.loadingProject")}
          </div>
        ) : project ? (
          <MangaTab
            projectId={project.id}
            canEditManga={canEditProject}
            snapshotChapters={chapters}
            assetProjectId={project.root_project_id ?? project.id}
            imageProjectIds={imageProjectIds}
          />
        ) : null}
      </section>
    </div>
  );
};
