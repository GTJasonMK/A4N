import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { GitCompare, PanelsTopLeft } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import {
  getProject,
  listSourceSnapshotChapters,
  type NovelProject,
  type SourceSnapshotChapter,
} from "../api/project";
import { extractErrorMessage } from "../api";
import { useToast } from "../components/feedback/Toast";
import { BookIconButton } from "../components/ui/BookButton";
import { BackButton } from "../components/ui/BackButton";
import { DerivedProjectHeader } from "../components/shell/DerivedProjectHeader";
import { MangaTab } from "./novel-detail/MangaTab";
import { SourceSyncDialog } from "./derived/SourceSyncDialog";
import { getDerivedProjectSubtitle } from "../utils/projectRouting";

export const ComicProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const addToast = useToast((state) => state.addToast);
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
      addToast("漫画项目参数无效", "error");
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

  const imageProjectIds = useMemo(() => {
    if (!project) return [];
    return project.root_project_id ? [project.id, project.root_project_id] : [project.id];
  }, [project]);
  const canEditProject =
    project?.my_role === "owner" || project?.my_role === "manager" || project?.my_role === "editor";

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <DerivedProjectHeader
        leading={<BackButton iconOnly title="返回项目列表" onClick={() => navigate("/")} />}
        icon={<PanelsTopLeft size={15} />}
        title={project?.title || "漫画项目"}
        subtitle={
          project
            ? getDerivedProjectSubtitle(project, "漫画项目工作区")
            : "加载源文本信息中"
        }
        actions={
          <>
            {canEditProject ? (
              <BookIconButton
                label="同步来源"
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

      <section aria-label="漫画项目工作区" className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden a4n-page-container md:overflow-hidden custom-scrollbar">
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
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
      </section>
    </div>
  );
};
