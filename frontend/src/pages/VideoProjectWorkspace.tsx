import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Film, GitCompare, Layers3, Loader2 } from "lucide-react";
import {
  getProject,
  listSourceSnapshotChapters,
  type NovelProject,
  type SourceSnapshotChapter,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { useToast } from "../components/feedback/Toast";
import { BookButton } from "../components/ui/BookButton";
import { EmptyHint, type ChapterRow } from "./novel-detail/shared";
import { VideoTab } from "./novel-detail/VideoTab";
import { SourceSyncDialog } from "./derived/SourceSyncDialog";

export const VideoProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const projectId = Number(id);

  const [project, setProject] = useState<NovelProject | null>(null);
  const [chapters, setChapters] = useState<SourceSnapshotChapter[]>([]);
  const [selectedSourceChapterId, setSelectedSourceChapterId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [sourceSyncOpen, setSourceSyncOpen] = useState(false);
  const refreshRequestRef = useRef(0);

  const refresh = useCallback(async () => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast("视频项目 ID 无效", "error");
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

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <header className="a4n-page-header shrink-0 flex-col items-stretch !gap-3 py-3">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <div className="flex shrink-0 flex-wrap items-center gap-2">
            <BookButton variant="secondary" size="sm" onClick={() => navigate("/")}>
              <ArrowLeft size={14} />
              返回
            </BookButton>
            {canEditProject ? (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => setSourceSyncOpen(true)}
                disabled={!project}
              >
                <GitCompare size={14} />
                源文本同步
              </BookButton>
            ) : null}
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => navigate(`/video-project/${projectId}/studio`)}
              disabled={!project}
            >
              <Layers3 size={14} />
              画布模式
            </BookButton>
          </div>
          <div className="flex min-w-0 items-center gap-2">
            <Film size={16} className="shrink-0 text-book-primary" />
            <div className="min-w-0">
              <h1 className="truncate text-sm font-semibold text-book-text-main">
                {project?.title || "视频项目"}
              </h1>
              <p className="truncate text-[11px] text-book-text-muted">
                {project?.root_project_title
                  ? `基于：${project.root_project_title}`
                  : "加载源文本信息中"}
              </p>
            </div>
          </div>
        </div>
        {project ? (
          <div className="mt-3 grid gap-2 text-[11px] text-book-text-muted sm:grid-cols-3">
            <Info label="源快照" value={project.source_snapshot_created_at || "-"} />
            <Info label="同步状态" value={project.source_sync_status} />
            <Info label="章节范围" value={formatChapterScope(project.derived_chapter_scope)} />
          </div>
        ) : null}
      </header>

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

      <main className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-3 py-4 md:overflow-hidden md:px-5 md:py-5">
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <Loader2 size={14} className="animate-spin" />
            加载视频项目
          </div>
        ) : project && !canEditProject ? (
          <EmptyHint text="当前账号没有编辑此视频项目的权限。" />
        ) : project && chapters.length > 0 ? (
          <div className="grid min-h-0 gap-3 md:h-full md:grid-cols-[220px_minmax(0,1fr)] md:gap-4">
            <aside className="a4n-page-surface min-h-0 overflow-x-auto p-2 md:overflow-y-auto custom-scrollbar">
              <div className="mb-2 px-2 text-[11px] font-medium text-book-text-muted">
                源快照章节
              </div>
              <div className="flex min-w-max gap-1 md:block md:min-w-0 md:space-y-1">
                {chapters.map((chapter) => {
                  const active = chapter.source_chapter_id === selectedSourceChapterId;
                  return (
                    <button
                      key={chapter.id}
                      type="button"
                      onClick={() => setSelectedSourceChapterId(chapter.source_chapter_id)}
                      data-active={active ? "1" : undefined}
                      className="a4n-sidebar-item min-w-[9rem] text-xs md:min-w-0"
                    >
                      <div className="font-medium">第 {chapter.chapter_number} 章</div>
                      <div className="mt-0.5 truncate text-[11px] text-book-text-muted">
                        {chapter.title || "未命名章节"}
                      </div>
                    </button>
                  );
                })}
              </div>
            </aside>
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
          <EmptyHint text="当前视频项目没有可用的源快照章节。" />
        )}
      </main>
    </div>
  );
};

const Info: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="a4n-stat-tile">
    <div className="mb-1 text-[10px] text-book-text-muted">{label}</div>
    <div className="truncate text-xs text-book-text-main">{value}</div>
  </div>
);

function formatChapterScope(scope: unknown): string {
  if (!scope || typeof scope !== "object") return "-";
  const record = scope as Record<string, unknown>;
  const start = Number(record.start);
  const end = Number(record.end);
  if (Number.isInteger(start) && Number.isInteger(end)) return `第 ${start}-${end} 章`;
  return JSON.stringify(scope);
}
