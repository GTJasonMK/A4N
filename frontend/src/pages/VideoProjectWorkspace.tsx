import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import { SourceSyncDialog } from "./derived/SourceSyncDialog";
import { getDerivedProjectSubtitle } from "../utils/projectRouting";

export const VideoProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const projectId = Number(id);

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
      addToast("视频项目参数无效", "error");
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
        addToast("当前项目不是视频项目", "error");
        navigate("/", { replace: true });
        return;
      }
      if (item.source_mode === "freeform") {
        navigate(`/video-project/${projectId}/studio`, { replace: true });
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
        addToast(extractErrorMessage(err, "加载视频项目失败"), "error");
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
        leading={<BackButton iconOnly title="返回项目列表" onClick={() => navigate("/")} />}
        icon={<Film size={15} />}
        title={project?.title || "视频项目"}
        subtitle={
          project
            ? getDerivedProjectSubtitle(project, "视频项目工作区")
            : "加载源文本信息中"
        }
        actions={
          <>
            {canEditProject ? (
              <BookIconButton
                label="同步源文本"
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
              onClick={() => navigate(`/video-project/${projectId}/studio`)}
              disabled={!project}
            >
              <Layers3 size={14} />
              画布模式
            </BookButton>
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

      <section aria-label="视频项目工作区" className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden a4n-page-container md:overflow-hidden">
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            加载视频项目
          </div>
        ) : project && !canEditProject ? (
          <EmptyHint text="当前账号没有编辑此视频项目的权限。" />
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
                  <span className="shrink-0 font-medium text-book-text-muted">来源章节</span>
                  <select
                    value={selectedSourceChapterId ?? ""}
                    onChange={(event) => setSelectedSourceChapterId(Number(event.target.value))}
                    className="pyqt-input h-9 min-w-0 flex-1 !px-2 !py-1.5 text-xs"
                  >
                    {chapters.map((chapter) => {
                      const title = sourceChapterTitle(chapter);
                      return (
                        <option key={chapter.id} value={chapter.source_chapter_id}>
                          第 {chapter.chapter_number} 章{title ? ` · ${title}` : ""}
                        </option>
                      );
                    })}
                  </select>
                </label>
                <aside aria-label="来源章节" className="a4n-flat-surface hidden max-h-full min-h-0 self-start overflow-y-auto p-2 custom-scrollbar md:block">
                  <div className="mb-2 hidden px-2 text-small font-medium text-book-text-muted md:block">
                    来源章节
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
                          <div className="font-medium">第 {chapter.chapter_number} 章</div>
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
                aria-label="当前来源章节"
                className="flex min-w-0 shrink-0 items-baseline gap-2 border-b border-book-border/45 px-1 pb-3 text-xs"
              >
                <span className="shrink-0 text-book-text-muted">来源章节</span>
                <span className="min-w-0 truncate font-medium text-book-text-main">
                  第 {selectedChapter.chapter_number} 章
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
          <EmptyHint text="当前视频项目没有可用的来源章节。" />
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
