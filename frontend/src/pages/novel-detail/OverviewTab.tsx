import React, { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronRight, FileText, GitBranch, Pencil, Users } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { BookButton } from "../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../components/ui/BookInput";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { promptDialog } from "../../components/feedback/PromptDialog";
import { extractErrorMessage } from "../../api";
import { canEditProjectMaterials, type NovelProject } from "../../api/project";
import {
  getBlueprint,
  refineBlueprint,
  saveBlueprint,
  type Blueprint,
} from "../../api/blueprint";
import { BlueprintVersionsPanel } from "./overview/BlueprintVersionsPanel";
import { ProjectOverviewCard } from "./overview/ProjectOverviewCard";
import { SynopsisCard } from "./overview/SynopsisCard";
import { FactFeedbackPanel } from "./FactFeedbackPanel";
import { getProjectAccessRoleLabel } from "../../utils/projectRouting";
import { isCancelledOperation } from "../../utils/cancellation";
import { nonEmptyText } from "../../utils/text";
import { useProgressTask } from "../../hooks/useProgressTask";
import { runBlueprintRefinementWithProgress } from "./blueprintRefinement";
import { useI18n } from "../../i18n";

const ProjectCollaborationPanel = lazy(() =>
  import("./overview/ProjectCollaborationPanel").then((module) => ({
    default: module.ProjectCollaborationPanel,
  })),
);

interface OverviewTabProps {
  project: NovelProject;
  onRefresh: () => Promise<boolean>;
  onOpenImports: () => void;
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
export const OverviewTab: React.FC<OverviewTabProps> = ({
  project,
  onRefresh,
  onOpenImports,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const navigate = useNavigate();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [refining, setRefining] = useState(false);
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
  const refiningRef = useRef(false);
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
      refiningRef.current = false;
      blueprintSaveBusyRef.current = false;
    };
  }, []);

  const load = useCallback(async (): Promise<boolean> => {
    const requestProjectId = project.id;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const bp = await getBlueprint(requestProjectId);
      if (!isCurrentLoad(requestProjectId, requestSeq)) return false;
      setBlueprint(bp);
      return true;
    } catch (err) {
      if (!isCurrentLoad(requestProjectId, requestSeq)) return false;
      addToast(extractErrorMessage(err, t("writing.blueprintLoadFailed")), "error");
      return false;
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
    setBlueprintEditorOpen(false);
    setBlueprintSaving(false);
    blueprintSaveBusyRef.current = false;
    setRefining(false);
  }, [project.id]);

  const handleRefine = async () => {
    if (!canEditProject) {
      addToast(t("novel.noEditPermission"), "error");
      return;
    }
    if (refiningRef.current) return;
    const requestProjectId = project.id;
    const requestSeq = refineSeqRef.current + 1;
    refineSeqRef.current = requestSeq;
    refiningRef.current = true;
    setRefining(true);
    try {
      const focus = await promptDialog({
        title: t("novel.refineBlueprint"),
        message: t("novel.refineBlueprintMessage"),
        placeholder: t("novel.refineBlueprintPlaceholder"),
        multiline: true,
        confirmText: t("novel.startRefine"),
      });
      if (focus === null || !isCurrentProject(requestProjectId)) return;
      const handle = await refineBlueprint({
        project_id: requestProjectId,
        focus: nonEmptyText(focus),
        variables: null,
      });
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      const syncWarnings = await runBlueprintRefinementWithProgress({
        runProgressTask,
        handle,
        projectId: requestProjectId,
        title: t("novel.refineBlueprint"),
        startMessage: t("novel.refiningBlueprint"),
        successMessage: t("novel.blueprintRefined"),
        errorMessage: t("novel.refineBlueprintFailed"),
        isCurrent: () => isCurrentRefine(requestProjectId, requestSeq),
        refresh: async () => {
          if (!await load()) return false;
          return onRefresh();
        },
        t,
      });
      addToast(
        syncWarnings.length > 0
          ? t("novel.blueprintRefinedWithWarnings", { warnings: syncWarnings.join(t("common.summarySeparator")) })
          : t("novel.blueprintRefined"),
        syncWarnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, t("novel.refineBlueprintFailed"));
      const cancelled = isCancelledOperation(message);
      addToast(
        cancelled ? t("feedback.progressCancelled") : message,
        cancelled ? "info" : "error",
      );
    } finally {
      if (refineSeqRef.current === requestSeq) refiningRef.current = false;
      if (isCurrentRefine(requestProjectId, requestSeq)) setRefining(false);
    }
  };

  const handleCreateBlueprint = () => {
    if (!canEditProject) {
      addToast(t("novel.noEditPermission"), "error");
      return;
    }
    navigate(`/inspiration/${project.id}`);
  };

  const handleOpenImports = () => {
    if (!canEditProject) {
      addToast(t("novel.noEditPermission"), "error");
      return;
    }
    onOpenImports();
  };

  const displayBlueprint = loadedProjectId === project.id ? blueprint : null;
  const displayLoading = loading || loadedProjectId !== project.id;

  const handleOpenBlueprintEditor = () => {
    if (!canEditProject) {
      addToast(t("novel.noEditPermission"), "error");
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
      addToast(t("novel.blueprintGenreRequired"), "error");
      return;
    }
    if (!Number.isInteger(totalChapters) || totalChapters < 1 || totalChapters > 10_000) {
      addToast(t("novel.totalChaptersRange"), "error");
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
      if (!await load()) return;
      if (!isCurrentProject(requestProjectId)) return;
      if (!await onRefresh()) return;
      if (!isCurrentProject(requestProjectId)) return;
      setBlueprintEditorOpen(false);
      addToast(
        result.warnings.length > 0
          ? t("novel.blueprintSavedWithWarnings", { warnings: result.warnings.join(t("common.summarySeparator")) })
          : t("novel.blueprintSaved"),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentProject(requestProjectId)) {
        addToast(extractErrorMessage(err, t("novel.blueprintSaveFailed")), "error");
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
            <h2 className="text-sm font-semibold text-book-text-main">{t("novel.overviewTitle")}</h2>
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
          canEditBlueprint={canEditProject}
          onOpenSynopsis={() => setSynopsisOpen(true)}
          onRefine={() => void handleRefine()}
          onEditBlueprint={handleOpenBlueprintEditor}
          onOpenImports={handleOpenImports}
          onCreateBlueprint={handleCreateBlueprint}
        />

        <div className="min-w-0 lg:col-span-2">
          <FactFeedbackPanel key={project.id} projectId={project.id} canEdit={canEditProject} refreshKey={project.updated_at} onChanged={async () => { if (!await load()) return false; return onRefresh(); }} />
        </div>

        <nav
          aria-label={t("novel.projectManagement")}
          className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:col-span-2"
        >
          {displayBlueprint ? (
            <OverviewActionCard
              icon={<GitBranch size={14} />}
              title={t("novel.blueprintVersion")}
              description={`${t("novel.currentVersion", { version: blueprintVersion })}${displayBlueprint.genre ? ` · ${displayBlueprint.genre}` : ""}`}
              onClick={() => setVersionsOpen(true)}
            />
          ) : null}
          <OverviewActionCard
            icon={<Users size={14} />}
            title={t("novel.collaborators")}
            description={project.pending_invite_count
              ? t("novel.memberSummaryWithInvites", {
                  count: memberCount,
                  role: getProjectAccessRoleLabel(project.my_role, t),
                  invites: project.pending_invite_count,
                })
              : t("novel.memberSummary", {
                  count: memberCount,
                  role: getProjectAccessRoleLabel(project.my_role, t),
                })}
            onClick={() => setCollabOpen(true)}
          />
        </nav>
      </div>

      <Modal
        isOpen={blueprintEditorOpen}
        onClose={() => {
          if (!blueprintSaving) setBlueprintEditorOpen(false);
        }}
        title={displayBlueprint ? t("novel.editFormalBlueprint") : t("novel.enterFormalBlueprint")}
        subtitle={t("novel.formalBlueprintSubtitle")}
        icon={<Pencil size={15} />}
        maxWidthClassName="max-w-3xl"
        bodyClassName="px-5 py-5"
      >
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <BookInput
              label={t("novel.genreRequired")}
              value={blueprintDraft.genre}
              maxLength={256}
              disabled={blueprintSaving}
              onChange={(event) =>
                setBlueprintDraft((current) => ({ ...current, genre: event.target.value }))
              }
            />
            <BookInput
              label={t("novel.totalChaptersRequired")}
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
              label={t("novel.writingStyle")}
              value={blueprintDraft.style}
              maxLength={1024}
              disabled={blueprintSaving}
              onChange={(event) =>
                setBlueprintDraft((current) => ({ ...current, style: event.target.value }))
              }
            />
            <BookInput
              label={t("novel.tone")}
              value={blueprintDraft.tone}
              maxLength={1024}
              disabled={blueprintSaving}
              onChange={(event) =>
                setBlueprintDraft((current) => ({ ...current, tone: event.target.value }))
              }
            />
          </div>
          <BookInput
            label={t("writing.blueprintFieldAudience")}
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
            label={t("novel.oneLineSummary")}
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
            label={t("novel.fullSynopsis")}
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
              {t("common.cancel")}
            </BookButton>
            <BookButton disabled={blueprintSaving} onClick={() => void handleSaveBlueprint()}>
              {blueprintSaving ? t("settings.saving") : t("novel.saveFormalBlueprint")}
            </BookButton>
          </div>
        </div>
      </Modal>

      <Modal
        isOpen={synopsisOpen}
        onClose={() => setSynopsisOpen(false)}
        title={t("novel.fullPlotSynopsis")}
        icon={<FileText size={15} />}
        maxWidthClassName="max-w-3xl"
        bodyClassName="px-5 py-5"
      >
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {displayBlueprint?.full_synopsis || t("novel.blueprintSynopsisMissing")}
        </div>
      </Modal>

      <Modal
        isOpen={versionsOpen}
        onClose={() => setVersionsOpen(false)}
        title={t("novel.blueprintVersion")}
        subtitle={`${project.title} · ${t("novel.currentVersion", { version: blueprintVersion })}`}
        icon={<GitBranch size={15} />}
        maxWidthClassName="max-w-4xl"
        bodyClassName="px-4 py-4"
      >
        <BlueprintVersionsPanel projectId={project.id} onMutated={load} />
      </Modal>

      <Modal
        isOpen={collabOpen}
        onClose={() => setCollabOpen(false)}
        title={t("novel.collaborators")}
        subtitle={`${project.title} · ${t("novel.collaborationRootProject")}`}
        icon={<Users size={15} />}
        maxWidthClassName="max-w-lg"
        bodyClassName="px-4 py-4"
      >
        {collabOpen ? (
          <Suspense fallback={<LoadingSpinner size="md" text={t("novel.loadingCollaborators")} />}>
            <ProjectCollaborationPanel project={project} onRefresh={onRefresh} />
          </Suspense>
        ) : null}
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
