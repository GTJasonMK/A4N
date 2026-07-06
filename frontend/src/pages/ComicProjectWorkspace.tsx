import React, { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, GitCompare, Loader2, PanelsTopLeft } from "lucide-react";
import {
  getProject,
  listSourceSnapshotChapters,
  type NovelProject,
  type SourceSnapshotChapter,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { useToast } from "../components/feedback/Toast";
import { BookButton } from "../components/ui/BookButton";
import { MangaTab } from "./novel-detail/MangaTab";
import { SourceSyncDialog } from "./derived/SourceSyncDialog";

export const ComicProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const projectId = Number(id);

  const [project, setProject] = useState<NovelProject | null>(null);
  const [chapters, setChapters] = useState<SourceSnapshotChapter[]>([]);
  const [loading, setLoading] = useState(true);
  const [sourceSyncOpen, setSourceSyncOpen] = useState(false);
  const refreshRequestRef = useRef(0);

  const refresh = useCallback(async () => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast("漫画项目 ID 无效", "error");
      setProject(null);
      setChapters([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const item = await getProject(projectId);
      if (refreshRequestRef.current !== requestId) return;
      if (item.project_kind !== "comic") {
        addToast("当前项目不是漫画项目", "error");
        navigate("/", { replace: true });
        return;
      }
      const snapshotChapters = await listSourceSnapshotChapters(projectId);
      if (refreshRequestRef.current !== requestId) return;
      setProject(item);
      setChapters(snapshotChapters);
    } catch (err) {
      if (refreshRequestRef.current === requestId) {
        addToast(extractErrorMessage(err, "加载漫画项目失败"), "error");
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

  const imageProjectIds = project?.root_project_id
    ? [project.id, project.root_project_id]
    : project
      ? [project.id]
      : [];
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
          </div>
          <div className="flex min-w-0 items-center gap-2">
            <PanelsTopLeft size={16} className="shrink-0 text-book-primary" />
            <div className="min-w-0">
              <h1 className="truncate text-sm font-semibold text-book-text-main">
                {project?.title || "漫画项目"}
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

      <main className="min-h-0 flex-1 overflow-y-auto px-3 py-4 sm:px-4 md:px-5 md:py-5 custom-scrollbar">
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <Loader2 size={14} className="animate-spin" />
            加载漫画项目
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
