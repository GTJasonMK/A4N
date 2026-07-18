import React, { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronRight, FileText, GitBranch, Users } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import type { NovelProject } from "../../api/project";
import { getBlueprint, refineBlueprint, type Blueprint } from "../../api/blueprint";
import { BlueprintVersionsPanel } from "./overview/BlueprintVersionsPanel";
import { ProjectCollaborationPanel } from "./overview/ProjectCollaborationPanel";
import { ProjectOverviewCard } from "./overview/ProjectOverviewCard";
import { SynopsisCard } from "./overview/SynopsisCard";
import { getProjectAccessRoleLabel } from "../../utils/projectRouting";

interface OverviewTabProps {
  project: NovelProject;
  onRefresh: () => Promise<void>;
}

/**
 * 项目概览：项目进度与故事摘要是主信息，版本和协作是次级入口。
 */
export const OverviewTab: React.FC<OverviewTabProps> = ({ project, onRefresh }) => {
  const { addToast } = useToast();
  const navigate = useNavigate();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [refining, setRefining] = useState(false);
  const [synopsisOpen, setSynopsisOpen] = useState(false);
  const [versionsOpen, setVersionsOpen] = useState(false);
  const [collabOpen, setCollabOpen] = useState(false);
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

  const isCurrentLoad = useCallback(
    (projectId: number, seq: number) => {
      return isCurrentProject(projectId) && loadSeqRef.current === seq;
    },
    [isCurrentProject],
  );

  const isCurrentRefine = useCallback(
    (projectId: number, seq: number) => {
      return isCurrentProject(projectId) && refineSeqRef.current === seq;
    },
    [isCurrentProject],
  );

  const canEditProject =
    project.my_role === "owner" ||
    project.my_role === "manager" ||
    project.my_role === "editor";

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
    setCollabOpen(false);
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

  const blueprintVersion = displayBlueprint?.current_version || "—";
  const memberCount = project.member_count ?? 1;

  return (
    <div className="content-main flex min-h-0 flex-col gap-3">
      <div className="grid min-h-0 grid-cols-1 content-start items-start gap-3 lg:grid-cols-[minmax(0,1.05fr)_minmax(0,0.95fr)]">
        <section className="a4n-flat-surface flex min-h-0 flex-col p-4">
          <header className="mb-3 border-b border-book-border/45 pb-3">
            <h2 className="text-sm font-semibold text-book-text-main">项目概况</h2>
          </header>
          <div className="min-h-0 flex-1">
            <ProjectOverviewCard
              project={project}
              blueprint={displayBlueprint}
              canEditProject={canEditProject}
              onRefresh={onRefresh}
              embedded
            />
          </div>
        </section>

        <SynopsisCard
          blueprint={displayBlueprint}
          loading={displayLoading}
          refining={refining}
          canEditBlueprint={canEditProject}
          onOpenSynopsis={() => setSynopsisOpen(true)}
          onRefine={() => void handleRefine()}
          onCreateBlueprint={handleCreateBlueprint}
        />

        <nav
          aria-label="项目管理"
          className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:col-span-2"
        >
          {displayBlueprint ? (
            <OverviewActionCard
              icon={<GitBranch size={14} />}
              title="蓝图版本"
              description={`当前 ${blueprintVersion}${displayBlueprint.genre ? ` · ${displayBlueprint.genre}` : ""}`}
              onClick={() => setVersionsOpen(true)}
            />
          ) : null}
          <OverviewActionCard
            icon={<Users size={14} />}
            title="协作成员"
            description={`${memberCount} 位成员 · ${getProjectAccessRoleLabel(project.my_role)}${
              project.pending_invite_count
                ? ` · ${project.pending_invite_count} 个邀请待处理`
                : ""
            }`}
            onClick={() => setCollabOpen(true)}
          />
        </nav>
      </div>

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
        subtitle={`${project.title} · 当前 ${blueprintVersion}`}
        icon={<GitBranch size={15} />}
        maxWidthClassName="max-w-4xl"
        bodyClassName="px-4 py-4"
      >
        <BlueprintVersionsPanel projectId={project.id} onMutated={load} />
      </Modal>

      <Modal
        isOpen={collabOpen}
        onClose={() => setCollabOpen(false)}
        title="协作成员"
        subtitle={`${project.title} · 协作根项目`}
        icon={<Users size={15} />}
        maxWidthClassName="max-w-lg"
        bodyClassName="px-4 py-4"
      >
        <ProjectCollaborationPanel project={project} onRefresh={onRefresh} />
      </Modal>
    </div>
  );
};

const OverviewActionCard: React.FC<{
  icon: React.ReactNode;
  title: string;
  description: string;
  onClick: () => void;
}> = ({ icon, title, description, onClick }) => (
  <button
    type="button"
    onClick={onClick}
    className="a4n-flat-surface flex w-full min-w-0 items-center gap-3 px-4 py-3 text-left transition-colors hover:border-book-primary/30 hover:bg-book-bg-tertiary/45 focus-visible:outline-2 focus-visible:outline-book-primary/35"
  >
    <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-book-primary/10 text-book-primary">
      {icon}
    </span>
    <div className="min-w-0 flex-1">
      <h2 className="text-xs font-medium text-book-text-main">{title}</h2>
      <p className="mt-0.5 truncate text-[11px] text-book-text-muted">{description}</p>
    </div>
    <ChevronRight size={14} className="shrink-0 text-book-text-muted" />
  </button>
);
