import { ThemedSelect } from "../components/ui/ThemedSelect";
import React, {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Film, GitCompare, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import {
  getProject,
  getSourceSnapshotChapter,
  listSourceSnapshotChapterIndex,
  type NovelProject,
  type SourceSnapshotChapterIndex,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { isPositiveSafeInteger } from "../utils/numberRange";
import { useToast } from "../components/feedback/Toast";
import { confirmDiscardMediaChanges } from "../components/feedback/UnsavedChangesGuard";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { BackButton } from "../components/ui/BackButton";
import { DerivedProjectHeader } from "../components/shell/DerivedProjectHeader";
import { EmptyHint, type ChapterRow } from "./novel-detail/shared";
import { VideoTab } from "./novel-detail/VideoTab";
import { formatNumber, useI18n } from "../i18n";
import { useNovelDetailDirtyRegistry } from "./novel-detail/novelDetailDirty";
import { UnsavedNavigationGuard } from "../components/navigation/UnsavedNavigationGuard";

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
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const translatorRef = useRef(t);
  translatorRef.current = t;
  const projectId = Number(id);

  const [project, setProject] = useState<NovelProject | null>(null);
  const [chapters, setChapters] = useState<SourceSnapshotChapterIndex[]>([]);
  const [selectedSourceChapterId, setSelectedSourceChapterId] = useState<number | null>(null);
  const [selectedChapterDetail, setSelectedChapterDetail] = useState<ChapterRow | null>(null);
  const [selectedChapterLoading, setSelectedChapterLoading] = useState(false);
  const [selectedChapterError, setSelectedChapterError] = useState<string | null>(null);
  const [workspaceDirty, setWorkspaceDirty] = useState(false);
  const selectSourceChapter = async (chapterId: number) => {
    if (chapterId === selectedSourceChapterId) return;
    if (!await confirmDiscardMediaChanges(workspaceDirty || dirtyRegistry.hasDirty(), t)) return;
    setWorkspaceDirty(false);
    setSelectedSourceChapterId(chapterId);
  };
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [sourceSyncOpen, setSourceSyncOpen] = useState(false);
  const refreshRequestRef = useRef(0);

  const refresh = useCallback(async (): Promise<boolean> => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    setLoadError(null);
    if (!isPositiveSafeInteger(projectId)) {
      const message = translatorRef.current("video.projectInvalid");
      setLoadError(message);
      setProject(null);
      setChapters([]);
      setSelectedSourceChapterId(null);
      setLoading(false);
      return false;
    }
    setLoading(true);
    try {
      const [projectResult, chaptersResult] = await Promise.allSettled([
        getProject(projectId),
        listSourceSnapshotChapterIndex(projectId),
      ]);
      if (projectResult.status === "rejected") throw projectResult.reason;
      const item = projectResult.value;
      if (refreshRequestRef.current !== requestId) return false;
      if (item.project_kind !== "video") {
        addToast(translatorRef.current("video.notVideoProject"), "error");
        navigate("/", { replace: true });
        return false;
      }
      if (item.source_mode !== "derived") {
        addToast(translatorRef.current("video.notVideoProject"), "error");
        navigate("/", { replace: true });
        return false;
      }
      if (refreshRequestRef.current !== requestId) return false;
      setProject(item);
      if (chaptersResult.status === "fulfilled") {
        const snapshotChapters = chaptersResult.value;
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
      } else {
        const message = extractErrorMessage(
          chaptersResult.reason,
          translatorRef.current("video.loadProjectFailed"),
        );
        setLoadError(message);
        setChapters([]);
        setSelectedSourceChapterId(null);
      }
      return chaptersResult.status === "fulfilled";
    } catch (err) {
      if (refreshRequestRef.current === requestId) {
        const message = extractErrorMessage(err, translatorRef.current("video.loadProjectFailed"));
        setLoadError(message);
        setProject(null);
        setChapters([]);
        setSelectedSourceChapterId(null);
      }
      return false;
    } finally {
      if (refreshRequestRef.current === requestId) {
        setLoading(false);
      }
    }
  }, [addToast, navigate, projectId]);

  useLayoutEffect(() => {
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

  const selectedChapterIndex = useMemo(
    () => chapters.find((chapter) => chapter.source_chapter_id === selectedSourceChapterId) ?? null,
    [chapters, selectedSourceChapterId],
  );
  const selectedChapter = useMemo<ChapterRow | null>(() => {
    if (!selectedChapterIndex) return null;
    if (
      selectedChapterDetail != null &&
      selectedChapterDetail.id === selectedChapterIndex.source_chapter_id &&
      selectedChapterDetail.snapshot_id === selectedChapterIndex.snapshot_id
    ) return selectedChapterDetail;
    return {
      id: selectedChapterIndex.source_chapter_id,
      snapshot_id: selectedChapterIndex.snapshot_id,
      chapter_number: selectedChapterIndex.chapter_number,
      title: selectedChapterIndex.title,
      status: "completed",
    };
  }, [selectedChapterDetail, selectedChapterIndex]);

  useEffect(() => {
    if (!project || !selectedChapterIndex) {
      setSelectedChapterDetail(null);
      setSelectedChapterLoading(false);
      setSelectedChapterError(null);
      return;
    }
    let active = true;
    setSelectedChapterDetail(null);
    setSelectedChapterLoading(true);
    setSelectedChapterError(null);
    void getSourceSnapshotChapter(
      project.id,
      selectedChapterIndex.source_chapter_id,
      selectedChapterIndex.snapshot_id,
    )
      .then((chapter) => {
        if (!active) return;
        setSelectedChapterDetail({
          id: chapter.source_chapter_id,
          snapshot_id: chapter.snapshot_id,
          chapter_number: chapter.chapter_number,
          title: chapter.title,
          content: chapter.content,
          content_hash: chapter.content_hash,
          summary: chapter.summary,
          analysis_data: chapter.analysis_data_json,
          status: "completed",
        });
      })
      .catch((error) => {
        if (!active) return;
        const message = extractErrorMessage(error, translatorRef.current("video.loadProjectFailed"));
        setSelectedChapterError(message);
        addToast(message, "error");
      })
      .finally(() => {
        if (active) setSelectedChapterLoading(false);
      });
    return () => {
      active = false;
    };
  }, [addToast, project, selectedChapterIndex]);

  const imageProjectIds = useMemo(() => {
    if (!project) return [];
    return project.root_project_id ? [project.id, project.root_project_id] : [project.id];
  }, [project]);
  const canEditProject =
    project?.my_role === "owner" || project?.my_role === "manager" || project?.my_role === "editor";
  const showChapterNavigation = chapters.length > 1;
  const handleBack = () => {
    navigate("/");
  };

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <UnsavedNavigationGuard dirty={workspaceDirty} name={t("video.projectWorkspace")} action={t("common.leavePage")} />
      <DerivedProjectHeader
        leading={<BackButton iconOnly title={t("a11y.back")} onClick={() => void handleBack()} />}
        icon={<Film size={15} />}
        title={project?.title || t("novel.videoProject")}
        actions={
          <>
            {canEditProject ? (
              <BookIconButton
                label={t("video.syncSource")}
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
            {t("video.loadingProject")}
          </div>
        ) : loadError ? null : project && !canEditProject ? (
          <EmptyHint text={t("video.noEditPermission")} />
        ) : project && chapters.length > 0 ? (
          <div
            className={
              showChapterNavigation
                ? "grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] gap-3 md:grid-rows-1 md:grid-cols-[220px_minmax(0,1fr)] md:gap-4"
                : "flex h-full min-h-0 flex-col gap-3"
            }
          >
            {showChapterNavigation ? (
              <>
                <label className="flex min-w-0 items-center gap-2 border-b border-book-border/45 pb-3 text-xs md:hidden">
                  <span className="shrink-0 font-medium text-book-text-muted">{t("video.sourceChapter")}</span>
                  <ThemedSelect
                    value={selectedSourceChapterId ?? ""}
                    onChange={(selectedValue) => { void selectSourceChapter(Number(selectedValue)); }}
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
                  </ThemedSelect>
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
                          onClick={() => { void selectSourceChapter(chapter.source_chapter_id); }}
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
            <section className="min-h-0 flex-1 overflow-hidden">
              {selectedChapterError ? (
                <div role="alert" className="flex flex-wrap items-center gap-3 text-sm text-error">
                  <p className="min-w-0 flex-1">{selectedChapterError}</p>
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => {
                      void (async () => {
                        if (await confirmDiscardMediaChanges(workspaceDirty || dirtyRegistry.hasDirty(), t)) {
                          await refresh();
                        }
                      })();
                    }}
                  >
                    <RefreshCw size={13} /> {t("common.retry")}
                  </BookButton>
                </div>
              ) : !selectedChapterIndex ? (
                <EmptyHint text={t("video.noSourceChapter")} />
              ) : selectedChapterLoading || selectedChapter?.content == null ? (
                <div role="status" className="inline-flex items-center gap-2 text-xs text-book-text-muted">
                  <LoadingSpinner size="sm" /> {t("common.loading")}
                </div>
              ) : (
                <VideoTab
                  key={`${project.id}:${chapters[0].snapshot_id}:${selectedSourceChapterId}`}
                  projectId={project.id}
                  canEdit={canEditProject}
                  onDirtyChange={setWorkspaceDirty}
                  fixedChapter={selectedChapter}
                  imageProjectIds={imageProjectIds}
                  materialProjectId={project.root_project_id ?? project.id}
                />
              )}
            </section>
          </div>
        ) : project ? (
          <EmptyHint text={t("video.noSourceChapter")} />
        ) : null}
      </section>
    </div>
  );
};

function sourceChapterTitle(chapter: SourceSnapshotChapterIndex): string | null {
  const title = chapter.title?.trim();
  if (!title) return null;
  const normalizedTitle = title.replace(/\s+/g, "");
  return normalizedTitle === `第${chapter.chapter_number}章` ? null : title;
}
