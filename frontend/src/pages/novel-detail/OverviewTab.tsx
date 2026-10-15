import React, { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronRight, FileText, GitBranch, Pencil, Users } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { BookButton } from "../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../components/ui/BookInput";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
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
import { isPositiveSafeInteger } from "../../utils/numberRange";
import { nonEmptyText } from "../../utils/text";
import { useProgressTask } from "../../hooks/useProgressTask";
import { runBlueprintRefinementWithProgress } from "./blueprintRefinement";
import { useI18n } from "../../i18n";
import { useNovelDetailDirty, useNovelDetailDirtyRegistry } from "./novelDetailDirty";

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
  const tRef = useRef(t);
  tRef.current = t;
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const { runProgressTask } = useProgressTask();
  const navigate = useNavigate();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loadError, setLoadError] = useState<unknown | null>(null);
  const [loading, setLoading] = useState(true);
  const [reading, setReading] = useState(false);
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
  const blueprintMutationVersionRef = useRef(0);
  const blueprintVersionRef = useRef<string | null>(null);
  const editorExpectedVersionRef = useRef<string | null>(null);
  const refineSeqRef = useRef(0);
  const refiningRef = useRef(false);
  const blueprintSaveBusyRef = useRef(false);
  const mountedRef = useRef(false);
  const readReadyRef = useRef(false);
  const readingRef = useRef(false);
  const loadErrorRef = useRef<unknown | null>(loadError);

  activeProjectIdRef.current = project.id;
  readingRef.current = reading;
  loadErrorRef.current = loadError;

  const ensureBlueprintReadCurrent = () => {
    if (readReadyRef.current && !readingRef.current && loadErrorRef.current == null) return true;
    addToast(loadErrorRef.current ? extractErrorMessage(loadErrorRef.current, t("novel.refreshFailed")) : t("common.operationInProgress"), "error");
    return false;
  };

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

  const load = useCallback(async (options: { silent?: boolean } = {}): Promise<boolean> => {
    const requestProjectId = project.id;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    const mutationVersion = blueprintMutationVersionRef.current;
    readReadyRef.current = false;
    readingRef.current = true;
    setReading(true);
    setLoadError(null);
    if (!options.silent) setLoading(true);
    try {
      const bp = await getBlueprint(requestProjectId);
      if (!isCurrentLoad(requestProjectId, requestSeq)) return false;
      if (blueprintMutationVersionRef.current !== mutationVersion) return true;
      setBlueprint(bp);
      blueprintVersionRef.current = bp?.current_version ?? null;
      setLoadError(null);
      readReadyRef.current = true;
      return true;
    } catch (err) {
      if (!isCurrentLoad(requestProjectId, requestSeq)) return false;
      setLoadError(err);
      readReadyRef.current = false;
      return false;
    } finally {
      if (isCurrentLoad(requestProjectId, requestSeq)) {
        setLoadedProjectId(requestProjectId);
        setLoading(false);
        readingRef.current = false;
        setReading(false);
      }
    }
  }, [isCurrentLoad, project.id]);

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
    blueprintMutationVersionRef.current = 0;
    readReadyRef.current = false;
    blueprintVersionRef.current = null;
    editorExpectedVersionRef.current = null;
    setRefining(false);
    setBlueprint(null);
    setLoadError(null);
    setLoadedProjectId(null);
  }, [project.id]);

  const handleRefine = async () => {
    if (!canEditProject) {
      addToast(t("novel.noEditPermission"), "error");
      return;
    }
    if (refiningRef.current) return;
    if (!ensureBlueprintReadCurrent()) return;
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
      if (focus === null || !isCurrentProject(requestProjectId) || !ensureBlueprintReadCurrent()) return;
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
        title: tRef.current("novel.refineBlueprint"),
        startMessage: tRef.current("novel.refiningBlueprint"),
        successMessage: tRef.current("novel.blueprintRefined"),
        errorMessage: tRef.current("novel.refineBlueprintFailed"),
        isCurrent: () => isCurrentRefine(requestProjectId, requestSeq),
        refresh: async () => {
          if (!await load({ silent: true })) return false;
          return onRefresh();
        },
        onRefreshFailure: () => {
          if (isCurrentRefine(requestProjectId, requestSeq)) addToast(tRef.current("novel.refreshFailed"), "error");
        },
        t: (key, params) => tRef.current(key, params),
      });
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      addToast(
        syncWarnings.length > 0
          ? tRef.current("novel.blueprintRefinedWithWarnings", { warnings: syncWarnings.join(tRef.current("common.summarySeparator")) })
          : tRef.current("novel.blueprintRefined"),
        syncWarnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, tRef.current("novel.refineBlueprintFailed"));
      const cancelled = isCancelledOperation(message);
      addToast(
        cancelled ? tRef.current("feedback.progressCancelled") : message,
        cancelled ? "info" : "error",
      );
      if (!cancelled) {
        void onRefresh().then((refreshed) => {
          if (!refreshed && isCurrentProject(requestProjectId)) {
            addToast(tRef.current("novel.refreshFailed"), "error");
          }
        });
      }
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
  const blueprintDraftDirty = blueprintEditorOpen && Boolean(
    blueprintDraft.genre.trim() !== (displayBlueprint?.genre ?? "").trim() ||
    blueprintDraft.style.trim() !== (displayBlueprint?.style ?? "").trim() ||
    blueprintDraft.tone.trim() !== (displayBlueprint?.tone ?? "").trim() ||
    blueprintDraft.targetAudience.trim() !== (displayBlueprint?.target_audience ?? "").trim() ||
    blueprintDraft.oneSentenceSummary.trim() !== (displayBlueprint?.one_sentence_summary ?? "").trim() ||
    blueprintDraft.fullSynopsis.trim() !== (displayBlueprint?.full_synopsis ?? "").trim() ||
    blueprintDraft.totalChapters.trim() !== String(displayBlueprint?.total_chapters ?? project.total_chapters ?? ""),
  );
  const discardBlueprintDraft = useCallback(() => {
    setBlueprintEditorOpen(false);
    editorExpectedVersionRef.current = null;
    setBlueprintDraft({
      genre: displayBlueprint?.genre ?? "",
      style: displayBlueprint?.style ?? "",
      tone: displayBlueprint?.tone ?? "",
      targetAudience: displayBlueprint?.target_audience ?? "",
      oneSentenceSummary: displayBlueprint?.one_sentence_summary ?? "",
      fullSynopsis: displayBlueprint?.full_synopsis ?? "",
      totalChapters: String(displayBlueprint?.total_chapters ?? project.total_chapters ?? ""),
    });
  }, [displayBlueprint, project.total_chapters]);
  useNovelDetailDirty(`overview:${project.id}`, blueprintDraftDirty, discardBlueprintDraft, () => !blueprintSaving);
  const closeBlueprintEditor = async () => {
    if (blueprintSaving) return;
    if (blueprintDraftDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.blueprintVersion"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setBlueprintEditorOpen(false);
    editorExpectedVersionRef.current = null;
  };
  const closeCollaboration = async () => {
    const key = `collaboration:${project.id}`;
    if (dirtyRegistry.hasDirty(key) && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.collaborationTitle"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    if (!await dirtyRegistry.discardDirty(key)) return;
    setCollabOpen(false);
  };
  const displayLoading = loading || loadedProjectId !== project.id;

  const handleOpenBlueprintEditor = () => {
    if (!canEditProject) {
      addToast(t("novel.noEditPermission"), "error");
      return;
    }
    if (!ensureBlueprintReadCurrent()) return;
    if (loadedProjectId !== project.id || loadError != null) {
      addToast(
        loadError
          ? extractErrorMessage(loadError, t("novel.refreshFailed"))
          : t("novel.refreshFailed"),
        "error",
      );
      return;
    }
    editorExpectedVersionRef.current = blueprintVersionRef.current;
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
    if (!ensureBlueprintReadCurrent()) return;
    const genre = blueprintDraft.genre.trim();
    const totalChapters = Number(blueprintDraft.totalChapters);
    if (!genre) {
      addToast(t("novel.blueprintGenreRequired"), "error");
      return;
    }
    if (!isPositiveSafeInteger(totalChapters) || totalChapters > 10_000) {
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
        expected_current_version: editorExpectedVersionRef.current,
        genre,
        style: optional(blueprintDraft.style),
        tone: optional(blueprintDraft.tone),
        target_audience: optional(blueprintDraft.targetAudience),
        one_sentence_summary: optional(blueprintDraft.oneSentenceSummary),
        full_synopsis: optional(blueprintDraft.fullSynopsis),
        total_chapters: totalChapters,
      });
      if (!isCurrentProject(requestProjectId)) return;
      blueprintVersionRef.current = result.current_version;
      editorExpectedVersionRef.current = result.current_version;
      blueprintMutationVersionRef.current += 1;
      const refreshPromise = load({ silent: true });
      setBlueprint((current) => current ? {
        ...current,
        genre,
        style: optional(blueprintDraft.style),
        tone: optional(blueprintDraft.tone),
        target_audience: optional(blueprintDraft.targetAudience),
        one_sentence_summary: optional(blueprintDraft.oneSentenceSummary),
        full_synopsis: optional(blueprintDraft.fullSynopsis),
        total_chapters: totalChapters,
        current_version: result.current_version,
      } : current);
      setBlueprintEditorOpen(false);
      addToast(
        result.warnings.length > 0
          ? tRef.current("novel.blueprintSavedWithWarnings", { warnings: result.warnings.join(tRef.current("common.summarySeparator")) })
          : tRef.current("novel.blueprintSaved"),
        result.warnings.length > 0 ? "info" : "success",
      );
      void (async () => {
        let loaded = false;
        let refreshed = false;
        try {
          loaded = await refreshPromise;
        } catch {
          loaded = false;
        }
        try {
          refreshed = await onRefresh();
        } catch {
          refreshed = false;
        }
        if (isCurrentProject(requestProjectId) && (!loaded || !refreshed)) {
          addToast(tRef.current("novel.refreshFailed"), "error");
        }
      })();
    } catch (err) {
      if (isCurrentProject(requestProjectId)) {
        addToast(extractErrorMessage(err, tRef.current("novel.blueprintSaveFailed")), "error");
        void load({ silent: true });
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
          loadError={loadError == null ? null : extractErrorMessage(loadError, tRef.current("writing.blueprintLoadFailed"))}
          refining={refining}
          canEditBlueprint={canEditProject}
          onRetry={() => void load()}
          onOpenSynopsis={() => setSynopsisOpen(true)}
          onRefine={() => void handleRefine()}
          onEditBlueprint={handleOpenBlueprintEditor}
          onOpenImports={handleOpenImports}
          onCreateBlueprint={handleCreateBlueprint}
        />

        <div className="min-w-0 lg:col-span-2">
          <FactFeedbackPanel
            key={project.id}
            projectId={project.id}
            canEdit={canEditProject}
            refreshKey={project.updated_at}
            onChanged={async () => {
              const loaded = await load();
              const refreshed = await onRefresh();
              return loaded && refreshed;
            }}
          />
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
        onClose={() => void closeBlueprintEditor()}
        title={displayBlueprint ? t("novel.editFormalBlueprint") : t("novel.enterFormalBlueprint")}
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
              onClick={() => void closeBlueprintEditor()}
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
        onClose={() => void closeCollaboration()}
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
