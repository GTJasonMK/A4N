import React, { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { FileText, GitBranch } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import type { NovelProject } from "../../api/project";
import { getBlueprint, refineBlueprint, type Blueprint } from "../../api/blueprint";
import { BlueprintVersionsPanel } from "./overview/BlueprintVersionsPanel";
import { ProjectCollaborationPanel } from "./overview/ProjectCollaborationPanel";
import { ProjectOverviewCard } from "./overview/ProjectOverviewCard";
import { SynopsisCard } from "./overview/SynopsisCard";

interface OverviewTabProps {
  project: NovelProject;
  onRefresh: () => Promise<void>;
}

export const OverviewTab: React.FC<OverviewTabProps> = ({ project, onRefresh }) => {
  const { addToast } = useToast();
  const navigate = useNavigate();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [refining, setRefining] = useState(false);
  const [synopsisOpen, setSynopsisOpen] = useState(false);
  const [versionsOpen, setVersionsOpen] = useState(false);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(project.id);
  const loadSeqRef = useRef(0);
  const refineSeqRef = useRef(0);
  const refiningRef = useRef(false);
  const mountedRef = useRef(false);

  activeProjectIdRef.current = project.id;

  const isCurrentProject = useCallback((projectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === projectId;
  }, []);

  const isCurrentLoad = useCallback((projectId: number, seq: number) => {
    return isCurrentProject(projectId) && loadSeqRef.current === seq;
  }, [isCurrentProject]);

  const isCurrentRefine = useCallback((projectId: number, seq: number) => {
    return isCurrentProject(projectId) && refineSeqRef.current === seq;
  }, [isCurrentProject]);

  const canEditProject =
    project.my_role === "owner" || project.my_role === "manager" || project.my_role === "editor";

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      refineSeqRef.current += 1;
      refiningRef.current = false;
    };
  }, []);

  const load = useCallback(async () => {
    const requestProjectId = project.id;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const bp = await getBlueprint(requestProjectId);
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setBlueprint(bp);
    } catch (err) {
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setBlueprint(null);
      addToast(extractErrorMessage(err, "加载蓝图失败"), "error");
    } finally {
      if (isCurrentLoad(requestProjectId, requestSeq)) {
        setLoadedProjectId(requestProjectId);
        setLoading(false);
      }
    }
  }, [addToast, isCurrentLoad, project.id]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    refineSeqRef.current += 1;
    refiningRef.current = false;
    setSynopsisOpen(false);
    setVersionsOpen(false);
    setRefining(false);
  }, [project.id]);

  const handleRefine = async () => {
    if (!canEditProject) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    if (refiningRef.current) return;
    const requestProjectId = project.id;
    const requestSeq = refineSeqRef.current + 1;
    refineSeqRef.current = requestSeq;
    refiningRef.current = true;
    setRefining(true);
    try {
      await refineBlueprint({
        project_id: requestProjectId,
        focus: null,
        variables: null,
      });
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      addToast("蓝图已优化", "success");
      await load();
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      await onRefresh();
    } catch (err) {
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      addToast(extractErrorMessage(err, "蓝图优化失败"), "error");
    } finally {
      if (refineSeqRef.current === requestSeq) refiningRef.current = false;
      if (isCurrentRefine(requestProjectId, requestSeq)) setRefining(false);
    }
  };

  const handleCreateBlueprint = () => {
    if (!canEditProject) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    navigate(`/inspiration/${project.id}`);
  };

  const displayBlueprint = loadedProjectId === project.id ? blueprint : null;
  const displayLoading = loading || loadedProjectId !== project.id;

  return (
    <div className="content-main flex flex-col gap-4">
      <ProjectOverviewCard
        project={project}
        blueprint={displayBlueprint}
        canEditProject={canEditProject}
        onOpenVersions={() => setVersionsOpen(true)}
        onRefresh={onRefresh}
      />

      <ProjectCollaborationPanel project={project} onRefresh={onRefresh} />

      <SynopsisCard
        blueprint={displayBlueprint}
        loading={displayLoading}
        refining={refining}
        canEditBlueprint={canEditProject}
        onOpenSynopsis={() => setSynopsisOpen(true)}
        onRefine={() => void handleRefine()}
        onCreateBlueprint={handleCreateBlueprint}
      />

      <Modal
        isOpen={synopsisOpen}
        onClose={() => setSynopsisOpen(false)}
        title="完整剧情梗概"
        icon={<FileText size={15} />}
        maxWidthClassName="max-w-3xl"
        bodyClassName="px-5 py-5"
      >
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {displayBlueprint?.full_synopsis || "蓝图未包含完整梗概字段"}
        </div>
      </Modal>

      <Modal
        isOpen={versionsOpen}
        onClose={() => setVersionsOpen(false)}
        title="蓝图版本"
        subtitle="版本操作从主页面移入弹窗，避免概览信息过载。"
        icon={<GitBranch size={15} />}
        maxWidthClassName="max-w-4xl"
        bodyClassName="px-4 py-4"
      >
        <BlueprintVersionsPanel projectId={project.id} onMutated={load} />
      </Modal>
    </div>
  );
};
