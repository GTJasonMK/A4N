import React, { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Gamepad2, GitCompare, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import {
  getProject,
  listSourceSnapshotChapters,
  resetProjectGeneratedData,
  type NovelProject,
  type SourceSnapshotChapter,
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

export const GalgameProjectWorkspace: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const projectId = Number(id);

  const [project, setProject] = useState<NovelProject | null>(null);
  const [chapters, setChapters] = useState<SourceSnapshotChapter[]>([]);
  const [loading, setLoading] = useState(true);
  const [sourceSyncOpen, setSourceSyncOpen] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [workspaceVersion, setWorkspaceVersion] = useState(0);
  const refreshRequestRef = useRef(0);
  const resetBusyRef = useRef(false);

  const refresh = useCallback(async () => {
    const requestId = refreshRequestRef.current + 1;
    refreshRequestRef.current = requestId;
    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast("GALGame 项目参数无效", "error");
      setProject(null);
      setChapters([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const item = await getProject(projectId);
      if (refreshRequestRef.current !== requestId) return;
      if (item.project_kind !== "galgame") {
        addToast("当前项目不是 GALGame 项目", "error");
        navigate("/", { replace: true });
        return;
      }
      const snapshotChapters = await listSourceSnapshotChapters(projectId);
      if (refreshRequestRef.current !== requestId) return;
      setProject(item);
      setChapters(snapshotChapters);
    } catch (err) {
      if (refreshRequestRef.current === requestId) {
        addToast(extractErrorMessage(err, "加载 GALGame 项目失败"), "error");
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

  const canEditProject =
    project?.my_role === "owner" || project?.my_role === "manager" || project?.my_role === "editor";
  const canResetProject = project?.my_role === "owner";

  const handleResetGeneratedData = useCallback(async () => {
    if (!project || resetBusyRef.current) return;
    resetBusyRef.current = true;
    const ok = await confirmDialog({
      title: "重置 Galgame 生成数据",
      message:
        "将删除当前 Galgame 剧本、场次、台词、选择支、视觉生成设置、已生成媒体、任务记录、优化会话和对话记录。\n\n会保留项目本身、来源内容、来源同步关系和模型配置。重置后需要重新生成剧本和视觉资产。\n\n此操作不可恢复，是否继续？",
      confirmText: "重置",
      cancelText: "取消",
      dialogType: "danger",
    });
    if (!ok) {
      resetBusyRef.current = false;
      return;
    }

    setResetting(true);
    setSourceSyncOpen(false);
    try {
      const reset = await resetProjectGeneratedData(project.id);
      const snapshotChapters = await listSourceSnapshotChapters(project.id);
      setProject(reset.project);
      setChapters(snapshotChapters);
      setWorkspaceVersion((value) => value + 1);
      addToast(
        `已重置 Galgame：删除 ${reset.deleted_galgame_scripts} 个剧本、${reset.deleted_galgame_scenes} 场、${reset.deleted_galgame_lines} 行、${reset.deleted_generation_specs} 个生成意图、${reset.deleted_media_assets} 个媒体资产。`,
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, "重置 Galgame 生成数据失败"), "error");
    } finally {
      resetBusyRef.current = false;
      setResetting(false);
    }
  }, [addToast, project]);

  return (
    <div className="h-full min-h-0 flex flex-col bg-book-bg">
      <DerivedProjectHeader
        leading={<BackButton iconOnly title="返回项目列表" onClick={() => navigate("/")} />}
        icon={<Gamepad2 size={15} />}
        title={project?.title || "GALGame 项目"}
        subtitle={
          project
            ? getDerivedProjectSubtitle(project, "GALGame 项目工作区")
            : "加载源文本信息中"
        }
        actions={
          <>
            {canEditProject ? (
              <BookIconButton
                label="同步来源"
                onClick={() => setSourceSyncOpen(true)}
                disabled={!project || resetting}
                variant="secondary"
                size="sm"
              >
                <GitCompare size={14} />
              </BookIconButton>
            ) : null}
            {canResetProject && !resetting ? (
              <Dropdown
                items={[
                  {
                    label: "重置生成数据",
                    onClick: handleResetGeneratedData,
                    danger: true,
                    icon: <Trash2 size={13} />,
                  },
                ]}
              />
            ) : null}
            {canResetProject && resetting ? (
              <BookButton
                variant="secondary"
                size="sm"
                disabled
              >
                <LoadingSpinner size="sm" />
                重置中
              </BookButton>
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

      <section aria-label="Galgame 项目工作区" className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden a4n-page-container md:overflow-hidden custom-scrollbar">
        {loading ? (
          <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            加载 GALGame 项目
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
