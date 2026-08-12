import React, { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronRight, FileText, GitBranch, Pencil, Users } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { BookButton } from "../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../components/ui/BookInput";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { canEditProjectMaterials, type NovelProject } from "../../api/project";
import {
  getBlueprint,
  loadBlueprintTaskResult,
  refineBlueprint,
  saveBlueprint,
  syncBlueprintMaterials,
  type Blueprint,
} from "../../api/blueprint";
import { generationClient } from "../../api/generationTasks";
import { BlueprintVersionsPanel } from "./overview/BlueprintVersionsPanel";
import { ProjectCollaborationPanel } from "./overview/ProjectCollaborationPanel";
import { ProjectOverviewCard } from "./overview/ProjectOverviewCard";
import { SynopsisCard } from "./overview/SynopsisCard";
import { reverseEngineerBlueprint } from "../../api/import";
import { cancelWorkflowTask } from "../../api/streaming";
import { allocateHostTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { PROJECT_ACCESS_ROLE_LABELS } from "../../utils/projectRouting";

interface OverviewTabProps {
  project: NovelProject;
  onRefresh: () => Promise<void>;
}

interface BlueprintEditorDraft {
  genre: string;
  style: string;
  tone: string;
  targetAudience: string;
  oneSentenceSummary: string;
  fullSynopsis: string;
  totalChapters: string;
}

const EMPTY_BLUEPRINT_DRAFT: BlueprintEditorDraft = {
  genre: "",
  style: "",
  tone: "",
  targetAudience: "",
  oneSentenceSummary: "",
  fullSynopsis: "",
  totalChapters: "",
};

/**
 * 项目概览：项目进度与故事摘要是主信息，版本和协作是次级入口。
 */
export const OverviewTab: React.FC<OverviewTabProps> = ({ project, onRefresh }) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const navigate = useNavigate();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [refining, setRefining] = useState(false);
  const [aggregatingBlueprint, setAggregatingBlueprint] = useState(false);
  const [synopsisOpen, setSynopsisOpen] = useState(false);
  const [versionsOpen, setVersionsOpen] = useState(false);
  const [collabOpen, setCollabOpen] = useState(false);
  const [blueprintEditorOpen, setBlueprintEditorOpen] = useState(false);
  const [blueprintSaving, setBlueprintSaving] = useState(false);
  const [blueprintDraft, setBlueprintDraft] = useState(EMPTY_BLUEPRINT_DRAFT);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(project.id);
  const loadSeqRef = useRef(0);
  const refineSeqRef = useRef(0);
  const aggregateSeqRef = useRef(0);
  const refiningRef = useRef(false);
  const aggregatingBlueprintRef = useRef(false);
  const blueprintSaveBusyRef = useRef(false);
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

  const canEditProject = canEditProjectMaterials(project);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      refineSeqRef.current += 1;
      aggregateSeqRef.current += 1;
      refiningRef.current = false;
      aggregatingBlueprintRef.current = false;
      blueprintSaveBusyRef.current = false;
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
    aggregateSeqRef.current += 1;
    refiningRef.current = false;
    aggregatingBlueprintRef.current = false;
    setSynopsisOpen(false);
    setVersionsOpen(false);
    setCollabOpen(false);
    setBlueprintEditorOpen(false);
    setBlueprintSaving(false);
    blueprintSaveBusyRef.current = false;
    setRefining(false);
    setAggregatingBlueprint(false);
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
      const handle = await refineBlueprint({
        project_id: requestProjectId,
        focus: null,
        variables: null,
      });
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      const task = await generationClient.waitForTerminal(handle);
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      await loadBlueprintTaskResult(requestProjectId, task.resultRef);
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      const syncWarnings = await syncBlueprintMaterials(requestProjectId);
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      await load();
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      addToast(
        syncWarnings.length > 0
          ? `蓝图已优化；${syncWarnings.join("；")}`
          : "蓝图已优化",
        syncWarnings.length > 0 ? "info" : "success",
      );
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

  const handleAggregateBlueprint = async () => {
    if (!canEditProject) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    if (aggregatingBlueprintRef.current) return;
    const requestProjectId = project.id;
    const requestSeq = aggregateSeqRef.current + 1;
    aggregateSeqRef.current = requestSeq;
    aggregatingBlueprintRef.current = true;
    setAggregatingBlueprint(true);
    const taskId = allocateHostTaskId("reverse-blueprint", requestProjectId);
    try {
      await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: displayBlueprint ? "更新正式蓝图" : "归纳正式蓝图",
          startMessage: "正在整理项目正文、简介、大纲和正式资料",
          successMessage: "蓝图已归纳并写入正式资料",
          errorMessage: "归纳蓝图失败",
          total: 3,
          cancelLabel: "停止归纳",
          runningHint: "只有成功完成才会更新正式蓝图；失败或停止时保留原蓝图",
        },
        () =>
          reverseEngineerBlueprint(
            {
              project_id: requestProjectId,
              content_sample: "",
              auto_context: true,
              context_limit: 50,
              variables: null,
            },
            taskId,
          ),
      );
      if (
        !isCurrentProject(requestProjectId) ||
        aggregateSeqRef.current !== requestSeq
      )
        return;
      await load();
      if (
        !isCurrentProject(requestProjectId) ||
        aggregateSeqRef.current !== requestSeq
      )
        return;
      await onRefresh();
      if (
        !isCurrentProject(requestProjectId) ||
        aggregateSeqRef.current !== requestSeq
      )
        return;
      addToast("蓝图已依据当前正式资料完成归纳", "success");
    } catch (err) {
      if (
        !isCurrentProject(requestProjectId) ||
        aggregateSeqRef.current !== requestSeq
      )
        return;
      const message = extractErrorMessage(err, "归纳蓝图失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (aggregateSeqRef.current === requestSeq)
        aggregatingBlueprintRef.current = false;
      if (isCurrentProject(requestProjectId) && aggregateSeqRef.current === requestSeq) {
        setAggregatingBlueprint(false);
      }
    }
  };

  const displayBlueprint = loadedProjectId === project.id ? blueprint : null;
  const displayLoading = loading || loadedProjectId !== project.id;

  const handleOpenBlueprintEditor = () => {
    if (!canEditProject) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    setBlueprintDraft({
      genre: displayBlueprint?.genre ?? "",
      style: displayBlueprint?.style ?? "",
      tone: displayBlueprint?.tone ?? "",
      targetAudience: displayBlueprint?.target_audience ?? "",
      oneSentenceSummary: displayBlueprint?.one_sentence_summary ?? "",
      fullSynopsis: displayBlueprint?.full_synopsis ?? "",
      totalChapters:
        displayBlueprint?.total_chapters != null
          ? String(displayBlueprint.total_chapters)
          : project.total_chapters != null
            ? String(project.total_chapters)
            : project.current_chapter > 0
              ? String(project.current_chapter)
              : "",
    });
    setBlueprintEditorOpen(true);
  };

  const handleSaveBlueprint = async () => {
    const genre = blueprintDraft.genre.trim();
    const totalChapters = Number(blueprintDraft.totalChapters);
    if (!genre) {
      addToast("请填写蓝图题材", "error");
      return;
    }
    if (!Number.isInteger(totalChapters) || totalChapters < 1 || totalChapters > 10_000) {
      addToast("总章节数必须是 1 到 10000 之间的整数", "error");
      return;
    }
    if (blueprintSaveBusyRef.current) return;
    const requestProjectId = project.id;
    blueprintSaveBusyRef.current = true;
    setBlueprintSaving(true);
    try {
      const optional = (value: string): string | null => value.trim() || null;
      const result = await saveBlueprint({
        project_id: requestProjectId,
        genre,
        style: optional(blueprintDraft.style),
        tone: optional(blueprintDraft.tone),
        target_audience: optional(blueprintDraft.targetAudience),
        one_sentence_summary: optional(blueprintDraft.oneSentenceSummary),
        full_synopsis: optional(blueprintDraft.fullSynopsis),
        total_chapters: totalChapters,
      });
      if (!isCurrentProject(requestProjectId)) return;
      setBlueprintEditorOpen(false);
      await load();
      if (!isCurrentProject(requestProjectId)) return;
      await onRefresh();
      if (!isCurrentProject(requestProjectId)) return;
      addToast(
        result.warnings.length > 0
          ? `蓝图已保存；${result.warnings.join("；")}`
          : "蓝图已保存",
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentProject(requestProjectId)) {
        addToast(extractErrorMessage(err, "保存蓝图失败"), "error");
      }
    } finally {
      if (isCurrentProject(requestProjectId)) {
        blueprintSaveBusyRef.current = false;
        setBlueprintSaving(false);
      }
    }
  };

  const blueprintVersion = displayBlueprint?.current_version || "—";
  const memberCount = project.member_count;

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
              embedded
            />
          </div>
        </section>

        <SynopsisCard
          blueprint={displayBlueprint}
          loading={displayLoading}
          refining={refining}
          aggregating={aggregatingBlueprint}
          canEditBlueprint={canEditProject}
          onOpenSynopsis={() => setSynopsisOpen(true)}
          onRefine={() => void handleRefine()}
          onEditBlueprint={handleOpenBlueprintEditor}
          onAggregateBlueprint={() => void handleAggregateBlueprint()}
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
            description={`${memberCount} 位成员 · ${PROJECT_ACCESS_ROLE_LABELS[project.my_role]}${
              project.pending_invite_count
                ? ` · ${project.pending_invite_count} 个邀请待处理`
                : ""
            }`}
            onClick={() => setCollabOpen(true)}
          />
        </nav>
      </div>

      <Modal
        isOpen={blueprintEditorOpen}
        onClose={() => {
          if (!blueprintSaving) setBlueprintEditorOpen(false);
        }}
        title={displayBlueprint ? "编辑正式蓝图" : "直接录入正式蓝图"}
        subtitle="保存后立即成为当前正式版本；世界观、角色和关系可在各自资料页继续补充"
        icon={<Pencil size={15} />}
        maxWidthClassName="max-w-3xl"
        bodyClassName="px-5 py-5"
      >
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <BookInput
              label="题材 *"
              value={blueprintDraft.genre}
              maxLength={256}
              disabled={blueprintSaving}
              onChange={(event) =>
                setBlueprintDraft((current) => ({ ...current, genre: event.target.value }))
              }
            />
            <BookInput
              label="总章节数 *"
              type="number"
              min={1}
              max={10000}
              value={blueprintDraft.totalChapters}
              disabled={blueprintSaving}
              onChange={(event) =>
                setBlueprintDraft((current) => ({
                  ...current,
                  totalChapters: event.target.value,
                }))
              }
            />
            <BookInput
              label="写作风格"
              value={blueprintDraft.style}
              maxLength={1024}
              disabled={blueprintSaving}
              onChange={(event) =>
                setBlueprintDraft((current) => ({ ...current, style: event.target.value }))
              }
            />
            <BookInput
              label="情感基调"
              value={blueprintDraft.tone}
              maxLength={1024}
              disabled={blueprintSaving}
              onChange={(event) =>
                setBlueprintDraft((current) => ({ ...current, tone: event.target.value }))
              }
            />
          </div>
          <BookInput
            label="目标读者"
            value={blueprintDraft.targetAudience}
            maxLength={1024}
            disabled={blueprintSaving}
            onChange={(event) =>
              setBlueprintDraft((current) => ({
                ...current,
                targetAudience: event.target.value,
              }))
            }
          />
          <BookTextarea
            label="一句话概要"
            rows={2}
            value={blueprintDraft.oneSentenceSummary}
            maxLength={4096}
            disabled={blueprintSaving}
            onChange={(event) =>
              setBlueprintDraft((current) => ({
                ...current,
                oneSentenceSummary: event.target.value,
              }))
            }
          />
          <BookTextarea
            label="完整梗概"
            rows={8}
            value={blueprintDraft.fullSynopsis}
            maxLength={131072}
            disabled={blueprintSaving}
            onChange={(event) =>
              setBlueprintDraft((current) => ({
                ...current,
                fullSynopsis: event.target.value,
              }))
            }
          />
          <div className="flex justify-end gap-2 border-t border-book-border/40 pt-4">
            <BookButton
              variant="secondary"
              disabled={blueprintSaving}
              onClick={() => setBlueprintEditorOpen(false)}
            >
              取消
            </BookButton>
            <BookButton disabled={blueprintSaving} onClick={() => void handleSaveBlueprint()}>
              {blueprintSaving ? "保存中…" : "保存为正式蓝图"}
            </BookButton>
          </div>
        </div>
      </Modal>

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
      <p className="mt-0.5 truncate text-small text-book-text-muted">{description}</p>
    </div>
    <ChevronRight size={14} className="shrink-0 text-book-text-muted" />
  </button>
);
