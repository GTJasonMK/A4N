import React, {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Film, GitCompare, Layers3 } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import {
  getProject,
  listSourceSnapshotChapters,
  type NovelProject,
  type SourceSnapshotChapterSummary,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { useToast } from "../components/feedback/Toast";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { BackButton } from "../components/ui/BackButton";
import { DerivedProjectHeader } from "../components/shell/DerivedProjectHeader";
import { EmptyHint, type ChapterRow } from "./novel-detail/shared";
import { VideoTab } from "./novel-detail/VideoTab";
import {
  getDerivedProjectSubtitle,
  getProjectStudioRoute,
} from "../utils/projectRouting";
import { formatNumber, useI18n } from "../i18n";

const SourceSyncDialog = lazy(() =>
  import("./derived/SourceSyncDialog").then((module) => ({
    default: module.SourceSyncDialog,
  })),
);

export const VideoProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const projectId = Number(id);

  const subtitleLabels = useMemo(
    () => ({
      fallback: t("video.projectWorkspace"),
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
  const [selectedSourceChapterId, setSelectedSourceChapterId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [sourceSyncOpen, setSourceSyncOpen] = useState(false);
  const refreshRequestRef = useRef(0);

  const refresh = useCallback(async () => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast(t("video.projectInvalid"), "error");
      setProject(null);
      setChapters([]);
      setSelectedSourceChapterId(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const item = await getProject(projectId);
      if (refreshRequestRef.current !== requestId) return;
      if (item.project_kind !== "video") {
        addToast(t("video.notVideoProject"), "error");
        navigate("/", { replace: true });
        return;
      }
      if (item.source_mode === "freeform") {
        navigate(getProjectStudioRoute(projectId), { replace: true });
        return;
      }
      const snapshotChapters = await listSourceSnapshotChapters(projectId);
      if (refreshRequestRef.current !== requestId) return;
      setProject(item);
      setChapters(snapshotChapters);
      setSelectedSourceChapterId((current) => {
        if (
          current != null &&
          snapshotChapters.some((chapter) => chapter.source_chapter_id === current)
        ) {
          return current;
        }
        return snapshotChapters[0]?.source_chapter_id ?? null;
      });
    } catch (err) {
      if (refreshRequestRef.current === requestId) {
        addToast(extractErrorMessage(err, t("video.loadProjectFailed")), "error");
      }
    } finally {
      if (refreshRequestRef.current === requestId) {
        setLoading(false);
      }
    }
  }, [addToast, navigate, projectId, t]);

  useEffect(() => {
    refreshRequestRef.current += 1;
    setProject(null);
    setChapters([]);
    setSelectedSourceChapterId(null);
    setSourceSyncOpen(false);
    void refresh();
    return () => {
      refreshRequestRef.current += 1;
    };
  }, [refresh]);

  const selectedChapter = useMemo<ChapterRow | null>(() => {
    const snapshotChapter = chapters.find(
      (chapter) => chapter.source_chapter_id === selectedSourceChapterId,
    );
    if (!snapshotChapter) return null;
    return {
      id: snapshotChapter.source_chapter_id,
      chapter_number: snapshotChapter.chapter_number,
      title: snapshotChapter.title,
      content: snapshotChapter.content,
      summary: snapshotChapter.summary,
      status: "completed",
      analysis_data: snapshotChapter.analysis_data_json,
    };
  }, [chapters, selectedSourceChapterId]);

  const imageProjectIds = useMemo(() => {
    if (!project) return [];
    return project.root_project_id ? [project.id, project.root_project_id] : [project.id];
  }, [project]);
  const canEditProject =
    project?.my_role === "owner" || project?.my_role === "manager" || project?.my_role === "editor";
  const showChapterNavigation = chapters.length > 1;

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <DerivedProjectHeader
        leading={<BackButton iconOnly title={t("a11y.back")} onClick={() => navigate("/")} />}
        icon={<Film size={15} />}
        title={project?.title || t("novel.videoProject")}
        subtitle={
          project
            ? getDerivedProjectSubtitle(project, subtitleLabels)
            : t("video.sourceLoading")
        }
        actions={
          <>
            {canEditProject ? (
              <BookIconButton
                label={t("video.syncSource")}
                onClick={() => setSourceSyncOpen(true)}
                disabled={!project}
                variant="secondary"
                size="sm"
              >
                <GitCompare size={14} />
              </BookIconButton>
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => navigate(getProjectStudioRoute(projectId))}
              disabled={!project}
            >
              <Layers3 size={14} />
              {t("video.canvasMode")}
            </BookButton>
          </>
        }
      />

      {project && sourceSyncOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <SourceSyncDialog
            open
            projectId={project.id}
            projectTitle={project.title}
            sourceSyncStatus={project.source_sync_status}
            canEditSourceSync={canEditProject}
            onClose={() => setSourceSyncOpen(false)}
            onChanged={refresh}
          />
        </Suspense>
      ) : null}

      <section aria-label={t("video.projectWorkspace")} className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden a4n-page-container md:overflow-hidden">
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            {t("video.loadingProject")}
          </div>
        ) : project && !canEditProject ? (
          <EmptyHint text={t("video.noEditPermission")} />
        ) : project && chapters.length > 0 ? (
          <div
            className={
              showChapterNavigation
                ? "grid min-h-0 gap-3 md:h-full md:grid-cols-[220px_minmax(0,1fr)] md:gap-4"
                : "flex min-h-0 flex-col gap-3 md:h-full"
            }
          >
            {showChapterNavigation ? (
              <>
                <label className="flex min-w-0 items-center gap-2 border-b border-book-border/45 pb-3 text-xs md:hidden">
                  <span className="shrink-0 font-medium text-book-text-muted">{t("video.sourceChapter")}</span>
                  <select
                    value={selectedSourceChapterId ?? ""}
                    onChange={(event) => setSelectedSourceChapterId(Number(event.target.value))}
                    className="pyqt-input h-9 min-w-0 flex-1 !px-2 !py-1.5 text-xs"
                  >
                    {chapters.map((chapter) => {
                      const title = sourceChapterTitle(chapter);
                      return (
                        <option key={chapter.id} value={chapter.source_chapter_id}>
                          {t("writing.chapterLabel", { number: formatNumber(chapter.chapter_number, locale) })}{title ? ` · ${title}` : ""}
                        </option>
                      );
                    })}
                  </select>
                </label>
                <aside aria-label={t("video.sourceChapter")} className="a4n-flat-surface hidden max-h-full min-h-0 self-start overflow-y-auto p-2 custom-scrollbar md:block">
                  <div className="mb-2 hidden px-2 text-small font-medium text-book-text-muted md:block">
                    {t("video.sourceChapter")}
                  </div>
                  <div className="space-y-1">
                    {chapters.map((chapter) => {
                      const active = chapter.source_chapter_id === selectedSourceChapterId;
                      const title = sourceChapterTitle(chapter);
                      return (
                        <button
                          key={chapter.id}
                          type="button"
                          onClick={() => setSelectedSourceChapterId(chapter.source_chapter_id)}
                          data-active={active ? "1" : undefined}
                          className="a4n-sidebar-item text-xs"
                        >
                          <div className="font-medium">{t("writing.chapterLabel", { number: formatNumber(chapter.chapter_number, locale) })}</div>
                          {title ? (
                            <div className="mt-0.5 truncate text-small text-book-text-muted">
                              {title}
                            </div>
                          ) : null}
                        </button>
                      );
                    })}
                  </div>
                </aside>
              </>
            ) : selectedChapter ? (
              <div
                aria-label={t("video.currentSourceChapter")}
                className="flex min-w-0 shrink-0 items-baseline gap-2 border-b border-book-border/45 px-1 pb-3 text-xs"
              >
                <span className="shrink-0 text-book-text-muted">{t("video.sourceChapter")}</span>
                <span className="min-w-0 truncate font-medium text-book-text-main">
                  {t("writing.chapterLabel", { number: formatNumber(selectedChapter.chapter_number, locale) })}
                  {selectedChapter.title ? ` · ${selectedChapter.title}` : ""}
                </span>
              </div>
            ) : null}
            <section className="min-h-[60vh] overflow-hidden md:min-h-0">
              <VideoTab
                projectId={project.id}
                fixedChapter={selectedChapter}
                imageProjectIds={imageProjectIds}
                materialProjectId={project.root_project_id ?? project.id}
              />
            </section>
          </div>
        ) : (
          <EmptyHint text={t("video.noSourceChapter")} />
        )}
      </section>
    </div>
  );
};

function sourceChapterTitle(chapter: SourceSnapshotChapterSummary): string | null {
  const title = chapter.title?.trim();
  if (!title) return null;
  const normalizedTitle = title.replace(/\s+/g, "");
  return normalizedTitle === `第${chapter.chapter_number}章` ? null : title;
}
