import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Eye,
  FileText,
  ImagePlus,
  Link2,
  MessageSquareText,
  Pencil,
  RefreshCw,
  Save,
  Trash2,
  Users,
  Wand2,
  X,
} from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { MangaPipelineArtifactStage } from "../../../api/generated/host";
import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type { Portrait } from "../../../api/portrait";
import type {
  BindMangaReferenceImageInput,
  MangaReferenceAssetKind,
  MangaPageReferenceTargetKind,
  MangaPipelineStatusSummary,
  MangaPagePromptSummary,
  SaveMangaPipelineArtifactInput,
} from "../../../api/manga";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { EmptyHint } from "../shared";
import { MangaPageImagePreview } from "./MangaPageImagePreview";
import {
  MangaPageStageArtifactEditor,
  validateMangaPageArtifactDraft,
  type MangaPageEditableArtifactStage,
} from "./MangaPipelineArtifactEditor";
import {
  LayoutSummaryGrid,
  statusLabel,
  type MangaPageDetailView,
} from "./MangaPagePromptListParts";
import { ReferenceBindingsPanel } from "./ReferenceBindingsPanel";
import {
  buildPagePrompts,
  findGeneratedPageImage,
  sumPipelinePrompts,
  type MangaPagePromptLabels,
  type MangaPagePrompt,
} from "./mangaPagePromptModel";
import {
  mangaPageImageBlockReason,
  mangaPageStageLabel,
  mangaImageStatusMatchesWorkspaceStage,
  type MangaPageWorkspaceStage,
} from "./mangaTabUtils";
import { useI18n } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";
import { getScrollRailCue, scrollRailItemIntoView } from "../../../utils/scrollRail";

interface MangaPagePromptListProps {
  stageMode?: MangaPageWorkspaceStage;
  onDirtyChange?: (dirty: boolean) => void;
  prompts: MangaPagePromptSummary[];
  paginationActions?: React.ReactNode;
  selectedPageNumber: number | null;
  onSelectedPageChange: (pageNumber: number | null) => void;
  promptsLoading: boolean;
  pagePromptsError: string | null;
  currentContentHash: string | null;
  chapterLocked?: boolean;
  generatingPageNumber?: number | null;
  profiles: CharacterProfile[];
  portraits: Portrait[];
  projectImages: GeneratedImage[];
  assetsLoading?: boolean;
  pipelineStatus?: MangaPipelineStatusSummary | null;
  optimizingPromptId?: number | null;
  actionsDisabled?: boolean;
  canEditManga: boolean;
  onPreviewPageImage?: (pageNumber: number) => void | Promise<void>;
  onGeneratePageImage?: (pageNumber: number) => void | Promise<void>;
  onBindReferenceImage?: (
    input: BindMangaReferenceImageInput,
    sourceImage?: GeneratedImage,
  ) => void | Promise<void>;
  onBindDiscoveredReference?: (
    sourceImage: GeneratedImage | File,
    target: { targetKind: MangaPageReferenceTargetKind; targetName: string },
  ) => void | Promise<void>;
  onUnbindDiscoveredReference?: (
    assetKind: MangaReferenceAssetKind,
    assetId: number,
  ) => void | Promise<void>;
  onGenerateDiscoveredReference?: (
    promptId: number,
    target: { targetKind: MangaPageReferenceTargetKind; targetName: string },
  ) => Promise<void>;
  onUploadReferenceImage?: (
    promptId: number,
    characterName: string,
    file: File,
  ) => Promise<void>;
  uploadingReferenceBindingKey?: string | null;
  onUpdatePrompt?: (promptId: number, imagePrompt: string) => void | Promise<void>;
  onSaveStageArtifact?: (input: SaveMangaPipelineArtifactInput) => Promise<void>;
  onGenerateStageArtifact?: (
    stage: MangaPipelineArtifactStage,
    pageNumber: number,
  ) => void | Promise<void>;
  onDeleteStageArtifact?: (
    stage: MangaPipelineArtifactStage,
    pageNumber: number,
  ) => void | Promise<void>;
  onDeleteOrphanedPage?: (pageNumber: number) => void | Promise<void>;
  onOptimizePrompt?: (prompt: MangaPagePromptSummary) => void | Promise<void>;
  onDeletePrompt?: (prompt: MangaPagePromptSummary) => void | Promise<void>;
  onRetryPagePrompts: () => void;
}

const pageDetailItems = (
  rows: Array<[string, string, string | null | undefined]>,
) =>
  rows
    .filter(([, , value]) => value != null && value.trim() !== "")
    .map(([key, label, value]) => ({ key, label, value: value ?? "" }));

const MangaPageContentDetails: React.FC<{ page: MangaPagePrompt }> = ({ page }) => {
  const { t } = useI18n();
  const content = page.pageContent;
  return content ? (
    <LayoutSummaryGrid
      items={pageDetailItems([
        ["summary", t("comic.prompt.contentSummary"), content.summary],
        ["action", t("comic.prompt.pageAction"), content.action],
        ["reaction", t("comic.prompt.pageReaction"), content.reaction],
        ["dialogue", t("comic.prompt.dialogueIntent"), content.dialogue_intent],
        ["entry", t("comic.prompt.entryState"), content.entry_state],
        ["exit", t("comic.prompt.exitState"), content.exit_state],
      ])}
    />
  ) : (
    <p className="text-xs text-book-text-muted">
      {t("comic.prompt.awaitingPageContent")}
    </p>
  );
};

const MangaPageAdaptationDetails: React.FC<{ page: MangaPagePrompt }> = ({ page }) => {
  const { t } = useI18n();
  const adaptation = page.pageAdaptation;
  return adaptation ? (
    <LayoutSummaryGrid
      items={pageDetailItems([
        ["intent", t("comic.prompt.visualIntent"), adaptation.visual_intent],
        ["continuity", t("comic.prompt.continuityRequirements"), adaptation.continuity_requirements.join("\n")],
        ["retained", t("comic.prompt.retainedDetails"), adaptation.retained_details.join("\n")],
        ["omitted", t("comic.prompt.omittedDetails"), adaptation.omitted_details.join("\n")],
      ])}
    />
  ) : (
    <p className="text-xs text-book-text-muted">
      {t("comic.prompt.awaitingPageAdaptation")}
    </p>
  );
};

const MangaPageStoryboardDetails: React.FC<{ page: MangaPagePrompt }> = ({ page }) => {
  const { t } = useI18n();
  const storyboard = page.prompt.page_plan;
  return (
    <section className="space-y-3">
      {storyboard ? (
        <>
          <LayoutSummaryGrid items={page.layoutItems} />
          <ol className="space-y-2">
            {storyboard.panels.map((panel) => (
              <li
                key={panel.panel_number}
                className="rounded-lg border border-book-border/45 bg-book-bg-paper/35 px-3 py-2"
              >
                <p className="text-caption font-semibold text-book-text-muted">
                  {t("comic.prompt.panelHeading", { panel: panel.panel_number })}
                  {panel.participants.length > 0
                    ? ` · ${panel.participants.join(t("common.listSeparator"))}`
                    : ""}
                </p>
                <p className="mt-1 text-xs leading-relaxed text-book-text-main">
                  {panel.observable_action}
                </p>
                {panel.reaction && (
                  <p className="mt-1 text-small text-book-text-sub">
                    {t("comic.prompt.pageReaction")}: {panel.reaction}
                  </p>
                )}
                <p className="mt-1 text-small text-book-text-muted">
                  {panel.spatial_relationship}
                </p>
              </li>
            ))}
          </ol>
        </>
      ) : (
        <p className="text-xs text-book-text-muted">
          {t("comic.prompt.awaitingStoryboard")}
        </p>
      )}
    </section>
  );
};

export const MangaPagePromptList: React.FC<MangaPagePromptListProps> = ({
  stageMode,
  onDirtyChange,
  prompts,
  paginationActions,
  selectedPageNumber,
  onSelectedPageChange,
  promptsLoading,
  pagePromptsError,
  currentContentHash,
  chapterLocked = false,
  generatingPageNumber = null,
  profiles,
  portraits,
  projectImages,
  assetsLoading = false,
  pipelineStatus,
  optimizingPromptId = null,
  actionsDisabled = false,
  canEditManga,
  onPreviewPageImage,
  onGeneratePageImage,
  onBindReferenceImage,
  onBindDiscoveredReference,
  onGenerateDiscoveredReference,
  onUnbindDiscoveredReference,
  onUploadReferenceImage,
  uploadingReferenceBindingKey,
  onUpdatePrompt,
  onSaveStageArtifact,
  onGenerateStageArtifact,
  onDeleteStageArtifact,
  onDeleteOrphanedPage,
  onOptimizePrompt,
  onDeletePrompt,
  onRetryPagePrompts,
}) => {
  const { t } = useI18n();
  const pagePromptLabels = useMemo<MangaPagePromptLabels>(() => ({
    role: {
      opening: t("comic.prompt.roleOpening"),
      setup: t("comic.prompt.roleSetup"),
      rising: t("comic.prompt.roleRising"),
      climax: t("comic.prompt.roleClimax"),
      falling: t("comic.prompt.roleFalling"),
      resolution: t("comic.prompt.roleResolution"),
      transition: t("comic.prompt.roleTransition"),
    },
    pacing: {
      slow: t("comic.prompt.pacingSlow"),
      medium: t("comic.prompt.pacingMedium"),
      fast: t("comic.prompt.pacingFast"),
      explosive: t("comic.prompt.pacingExplosive"),
    },
    importance: {
      critical: t("comic.prompt.importanceCritical"),
      high: t("comic.prompt.importanceHigh"),
      normal: t("comic.prompt.importanceNormal"),
      low: t("comic.prompt.importanceLow"),
    },
    readingFlow: {
      top_to_bottom_left_to_right: t("comic.prompt.flowTopToBottom"),
      z_path: t("comic.prompt.flowZPath"),
      s_path: t("comic.prompt.flowSPath"),
      diagonal: t("comic.prompt.flowDiagonal"),
    },
    pageFunction: t("comic.prompt.pageFunction"),
    pagePacing: t("comic.prompt.pagePacing"),
    importanceLabel: t("comic.prompt.importance"),
    readingFlowLabel: t("comic.prompt.readingFlow"),
    pageGoal: t("comic.prompt.pageGoal"),
    visualThesis: t("comic.prompt.visualThesis"),
    entryState: t("comic.prompt.entryState"),
    exitState: t("comic.prompt.exitState"),
    dominantPanel: t("comic.prompt.dominantPanel"),
    panelCount: t("comic.prompt.panelCount"),
    primaryPortrait: t("comic.prompt.primaryPortrait"),
    primaryImage: t("comic.prompt.primaryImage"),
    projectImage: t("comic.prompt.projectImage"),
    otherPortrait: t("comic.prompt.otherPortrait"),
  }), [t]);
  const pagePrompts = useMemo(
    () =>
      buildPagePrompts(
        prompts,
        currentContentHash,
        pagePromptLabels,
        stageMode,
      ),
    [currentContentHash, pagePromptLabels, prompts, stageMode],
  );
  const imageStatusVisible = mangaImageStatusMatchesWorkspaceStage(stageMode);
  const pageRailRef = useRef<HTMLElement | null>(null);
  const activePageButtonRef = useRef<HTMLButtonElement | null>(null);
  const [pageRailHasBefore, setPageRailHasBefore] = useState(false);
  const [pageRailHasMore, setPageRailHasMore] = useState(false);
  const [detailView, setDetailView] = useState<MangaPageDetailView | null>(null);
  const [promptDrafts, setPromptDrafts] = useState<Record<number, string>>({});
  const [promptSaving, setPromptSaving] = useState(false);
  const [stageDraft, setStageDraft] = useState<{ key: string; input: SaveMangaPipelineArtifactInput } | null>(null);
  const [stageSaving, setStageSaving] = useState(false);
  const [selectedCandidateByCharacter, setSelectedCandidateByCharacter] = useState<
    Record<string, string>
  >({});
  const promptScopeKey = useMemo(
    () => prompts.map((prompt) => `${prompt.id}:${prompt.page_number}`).join("|"),
    [prompts],
  );

  useEffect(() => {
    if (pagePrompts.length === 0) {
      if (selectedPageNumber !== null) onSelectedPageChange(null);
      return;
    }
    if (
      selectedPageNumber == null ||
      !pagePrompts.some((page) => page.pageNumber === selectedPageNumber)
    ) {
      onSelectedPageChange(pagePrompts[0].pageNumber);
    }
  }, [onSelectedPageChange, pagePrompts, selectedPageNumber]);

  const activePage =
    pagePrompts.find((page) => page.pageNumber === selectedPageNumber) ?? pagePrompts[0] ?? null;
  const activePageStage = activePage?.pipelineStage ??
    (imageStatusVisible && activePage?.prompt.status !== "pending" ? "image" : null);
  const activePageStatus = activePage?.pipelineStatus ??
    (imageStatusVisible ? activePage?.prompt.status ?? null : null);
  const activePageSummary = activePage
    ? [
        activePageStage ? mangaPageStageLabel(activePageStage, t) : "",
        activePageStatus ? statusLabel(activePageStatus, t) : "",
        activePage.prompt.generated_image_id != null ? t("comic.prompt.hasImage") : t("comic.prompt.noImage"),
        activePage.sourceStale ? t("comic.prompt.sourceStale") : "",
        activePage.promptCount > 1 ? t("comic.prompt.recordCount", { count: activePage.promptCount }) : "",
      ].filter(Boolean).join(" · ")
    : "";
  const stageArtifactPayload = (stage: MangaPipelineArtifactStage): unknown | null => {
    if (!activePage) return null;
    switch (stage) {
      case "page_content":
        return activePage.prompt.page_content;
      case "page_adaptation":
        return activePage.prompt.page_adaptation;
      case "storyboard":
        return activePage.prompt.page_plan;
      case "reference_discovery":
        return activePage.prompt.reference_discovery;
      default:
        return null;
    }
  };

  useEffect(() => {
    const rail = pageRailRef.current;
    if (!rail) return;
    const updateCue = () => {
      const cue = getScrollRailCue(rail);
      setPageRailHasBefore(cue.hasBefore);
      setPageRailHasMore(cue.hasMore);
    };
    const frame = window.requestAnimationFrame(() => {
      scrollRailItemIntoView(rail, activePageButtonRef.current, "smooth");
      updateCue();
    });
    rail.addEventListener("scroll", updateCue, { passive: true });
    window.addEventListener("resize", updateCue);
    const resizeObserver =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(updateCue);
    resizeObserver?.observe(rail);
    return () => {
      window.cancelAnimationFrame(frame);
      rail.removeEventListener("scroll", updateCue);
      window.removeEventListener("resize", updateCue);
      resizeObserver?.disconnect();
    };
  }, [activePage?.pageNumber, pagePrompts.length]);
  const activeDetailView: MangaPageDetailView = stageMode != null
    ? "prompt"
    : detailView ?? (activePage?.prompt.generated_image_id != null ? "image" : "prompt");
  const editingPromptId = activePage && promptDrafts[activePage.prompt.id] !== undefined ? activePage.prompt.id : null;
  const promptDraft = editingPromptId == null ? "" : promptDrafts[editingPromptId];
  const promptDraftDirty = Object.entries(promptDrafts).some(([id, value]) => {
    const prompt = prompts.find((item) => item.id === Number(id));
    return prompt != null && value !== prompt.image_prompt;
  });
  const stageDraftDirty = stageDraft != null;
  useEffect(() => { onDirtyChange?.(promptDraftDirty || stageDraftDirty); }, [onDirtyChange, promptDraftDirty, stageDraftDirty]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  useNovelDetailDirty(`manga-prompts:${prompts.map((prompt) => prompt.id).join(",")}`, promptDraftDirty || stageDraftDirty);
  const setPromptDraft = (value: string) => {
    if (activePage) setPromptDrafts((current) => ({ ...current, [activePage.prompt.id]: value }));
  };
  const discardPromptDraft = (id: number) => setPromptDrafts((current) => {
    const next = { ...current };
    delete next[id];
    return next;
  });
  const hasMultiplePages = pagePrompts.length > 1;
  const activePageImage = useMemo(
    () => findGeneratedPageImage(activePage, projectImages),
    [activePage, projectImages],
  );

  useEffect(() => {
    setSelectedCandidateByCharacter({});
  }, [promptScopeKey]);

  useEffect(() => {
    if (stageDraft && (!activePage || !stageDraft.key.startsWith(`${activePage.prompt.id}:`))) {
      setStageDraft(null);
    }
  }, [activePage, stageDraft]);

  useEffect(() => {
    if (!canEditManga) {
      setPromptDrafts({});
      setPromptSaving(false);
    }
  }, [canEditManga]);

  const linkedImageCount = pagePrompts.filter(
    (page) => page.prompt.generated_image_id != null,
  ).length;
  const staleCount = pagePrompts.filter((page) => page.sourceStale).length;
  const bindingCount = pagePrompts.reduce(
    (sum, page) => sum + page.referenceBindings.length,
    0,
  );
  const boundBindingCount = pagePrompts.reduce(
    (sum, page) =>
      sum +
      page.referenceBindings.filter(
        (binding) =>
          binding.systemAsset != null || binding.userAddedAssets.length > 0,
      ).length,
    0,
  );
  const pipelinePromptCount = imageStatusVisible ? sumPipelinePrompts(pipelineStatus) : 0;
  const pipelineCompleted = imageStatusVisible ? pipelineStatus?.completed ?? 0 : 0;
  const pipelineFailed = pagePrompts.filter(
    (page) =>
      page.pipelineStatus === "failed" ||
      (imageStatusVisible && page.prompt.status === "failed"),
  ).length;
  const pipelineCheckpointLabel = pipelineStatus?.has_checkpoint
    ? pipelineStatus.checkpoint_valid
      ? t("comic.prompt.available")
      : t("comic.prompt.expired")
    : t("comic.prompt.none");
  if (promptsLoading && prompts.length === 0) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> {t("comic.prompt.loading")}
      </div>
    );
  }

  if (pagePrompts.length === 0) {
    return (
      <div className="flex h-full min-h-0 flex-col gap-3">
        {paginationActions ? (
          <div className="flex shrink-0 justify-end">{paginationActions}</div>
        ) : null}
        {pagePromptsError ? (
          <div role="alert" className="flex flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
            <span className="min-w-0 flex-1">{pagePromptsError}</span>
            <BookButton size="sm" variant="secondary" onClick={onRetryPagePrompts}>
              <RefreshCw size={12} /> {t("common.retry")}
            </BookButton>
          </div>
        ) : null}
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyHint text={t(stageMode === "page_planning"
            ? "comic.prompt.awaitingPageContent"
            : stageMode === "visual_design"
              ? "comic.prompt.awaitingStoryboard"
              : "comic.prompt.empty")} />
        </div>
      </div>
    );
  }

  const renderGenerateButton = (page: MangaPagePrompt) => {
    if (
      !canEditManga ||
      !onGeneratePageImage ||
      !page.prompt.drawing_prompt ||
      page.prompt.generated_image_id != null
    ) return null;
    const generating = generatingPageNumber === page.pageNumber;
    const imageBlockReason = mangaPageImageBlockReason(page.prompt, t);
    return (
      <BookButton
        size="sm"
        variant="primary"
        onClick={() => void onGeneratePageImage(page.pageNumber)}
        disabled={
          actionsDisabled ||
          chapterLocked ||
          generating ||
          generatingPageNumber != null
        }
        title={imageBlockReason ?? undefined}
      >
        {generating ? (
          <LoadingSpinner size="sm" />
        ) : (
          <ImagePlus size={13} />
        )}
        {t("comic.prompt.generateImage")}
      </BookButton>
    );
  };

  const beginPromptEdit = (page: MangaPagePrompt) => {
    if (!canEditManga) return;
    setDetailView("prompt");
    setPromptDrafts((current) => ({ ...current, [page.prompt.id]: page.prompt.image_prompt }));
  };

  const renderEditPromptButton = () => {
    if (
      !activePage?.prompt.drawing_prompt ||
      !canEditManga ||
      !onUpdatePrompt ||
      editingPromptId === activePage.prompt.id
    ) return null;
    return (
      <BookButton
        size="sm"
        variant="secondary"
        onClick={() => beginPromptEdit(activePage)}
        disabled={actionsDisabled || chapterLocked}
      >
        <Pencil size={13} /> {t("comic.prompt.edit")}
      </BookButton>
    );
  };

  const cancelPromptEdit = () => {
    if (editingPromptId != null) discardPromptDraft(editingPromptId);
  };

  const savePromptEdit = async (page: MangaPagePrompt) => {
    if (!canEditManga) {
      setPromptDrafts({});
      setPromptSaving(false);
      return;
    }
    const nextPrompt = promptDraft.trim();
    if (!nextPrompt || !onUpdatePrompt) return;
    setPromptSaving(true);
    try {
      await onUpdatePrompt(page.prompt.id, nextPrompt);
      discardPromptDraft(page.prompt.id);
    } catch {
      // 调用方负责 toast；这里保留编辑状态，方便继续修改后重试。
    } finally {
      setPromptSaving(false);
    }
  };

  const stageArtifactKey = (stage: MangaPipelineArtifactStage) =>
    activePage ? `${activePage.prompt.id}:${stage}` : null;

  const beginStageEdit = (stage: MangaPageEditableArtifactStage) => {
    const key = stageArtifactKey(stage);
    const payload = stageArtifactPayload(stage);
    if (!canEditManga || !activePage || !key || payload == null) return;
    setStageDraft({
      key,
      input: {
        project_id: activePage.prompt.project_id,
        chapter_id: activePage.prompt.chapter_id,
        page_number: activePage.pageNumber,
        stage,
        expected_prompt_updated_at: activePage.prompt.updated_at,
        payload,
      },
    });
  };

  const saveStageEdit = async (stage: MangaPageEditableArtifactStage) => {
    const key = stageArtifactKey(stage);
    if (!activePage || !key || stageDraft?.key !== key || !onSaveStageArtifact) return;
    if (
      validateMangaPageArtifactDraft(
        stage,
        stageDraft.input.payload,
        activePage.pageNumber,
        t,
      ) != null
    ) return;
    setStageSaving(true);
    try {
      await onSaveStageArtifact(stageDraft.input);
      setStageDraft(null);
    } catch {
      // 调用方负责 toast；这里保留编辑状态，方便修正后重试。
    } finally {
      setStageSaving(false);
    }
  };

  const deleteStage = async (stage: MangaPipelineArtifactStage) => {
    if (!activePage || !stageArtifactPayload(stage) || !onDeleteStageArtifact) return;
    try {
      await onDeleteStageArtifact(stage, activePage.pageNumber);
    } catch {
      // 调用方负责 toast。
    }
  };

  const deleteStageOutput = async (
    stage: MangaPipelineArtifactStage,
    pageNumber: number,
  ) => {
    if (!onDeleteStageArtifact) return;
    await onDeleteStageArtifact(stage, pageNumber);
  };

  const renderStageArtifact = (
    stage: MangaPageEditableArtifactStage | "reference_discovery",
    label: string,
    content: React.ReactNode,
  ) => {
    const key = stageArtifactKey(stage);
    const editing = key != null && stageDraft?.key === key;
    const payload = stageArtifactPayload(stage);
    const editableStage = stage === "reference_discovery" ? null : stage;
    const generatableStage =
      stage === "page_adaptation" ||
      stage === "storyboard" ||
      stage === "reference_discovery";
    const generateLabel = stage === "page_adaptation"
      ? t("comic.prompt.generatePageAdaptation")
      : stage === "storyboard"
        ? t("comic.prompt.generateStoryboard")
        : t("comic.prompt.discoverReferences");
    const draftError =
      editing && stageDraft && editableStage && activePage
        ? validateMangaPageArtifactDraft(
            editableStage,
            stageDraft.input.payload,
            activePage.pageNumber,
            t,
          )
        : null;
    return (
      <section className="min-w-0 space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h3 className="text-xs font-semibold text-book-text-main">{label}</h3>
          {editing ? (
            <div className="flex flex-wrap items-center gap-1.5">
              <BookButton size="sm" variant="secondary" onClick={() => setStageDraft(null)} disabled={actionsDisabled || stageSaving}>
                <X size={12} /> {t("comic.prompt.cancel")}
              </BookButton>
              {editableStage ? <BookButton size="sm" variant="primary" onClick={() => void saveStageEdit(editableStage)} disabled={actionsDisabled || stageSaving || draftError != null}>
                {stageSaving ? <LoadingSpinner size="sm" /> : <Save size={12} />} {t("comic.prompt.save")}
              </BookButton> : null}
            </div>
          ) : canEditManga ? (
            <div className="flex flex-wrap items-center gap-1.5">
              {payload != null && onDeleteStageArtifact ? (
                <BookButton size="sm" variant="secondary" onClick={() => void deleteStage(stage)} disabled={actionsDisabled || chapterLocked}>
                  <Trash2 size={12} /> {t("common.delete")}
                </BookButton>
              ) : null}
              {onSaveStageArtifact && editableStage && payload != null ? (
                <BookButton size="sm" variant="secondary" onClick={() => beginStageEdit(editableStage)} disabled={actionsDisabled || chapterLocked}>
                  <Pencil size={12} /> {t("comic.prompt.editArtifact")}
                </BookButton>
              ) : null}
              {generatableStage && activePage && onGenerateStageArtifact && payload == null ? (
                <BookButton
                  size="sm"
                  variant="primary"
                  onClick={() => void onGenerateStageArtifact(stage, activePage.pageNumber)}
                  disabled={actionsDisabled || chapterLocked}
                >
                  <RefreshCw size={12} /> {generateLabel}
                </BookButton>
              ) : null}
            </div>
          ) : null}
        </div>
        {editing && stageDraft && editableStage ? (
          <>
            <MangaPageStageArtifactEditor
              stage={editableStage}
              value={stageDraft.input.payload}
              labels={pagePromptLabels}
              disabled={actionsDisabled || stageSaving}
              onChange={(value) => setStageDraft({ ...stageDraft, input: { ...stageDraft.input, payload: value } })}
            />
            {draftError ? (
              <p role="alert" className="text-small text-error">
                {draftError}
              </p>
            ) : null}
          </>
        ) : content}
      </section>
    );
  };

  const activePromptImageRetentionHint =
    activePage?.prompt.generated_image_id != null
      ? t("comic.prompt.imageRetained")
      : undefined;
  const optimizingActivePrompt =
    activePage != null && optimizingPromptId === activePage.prompt.id;
  const pageSummaryParts = [
    pipelineFailed > 0 ? t("comic.prompt.failedCount", { count: pipelineFailed }) : "",
    staleCount > 0 ? t("comic.prompt.staleCount", { count: staleCount }) : "",
    imageStatusVisible && pipelineStatus?.has_checkpoint
      ? t("comic.prompt.checkpoint", { status: pipelineCheckpointLabel })
      : "",
    t("comic.prompt.pageCount", { count: pagePrompts.length }),
    t("comic.prompt.generatedCount", { linked: linkedImageCount, total: pagePrompts.length }),
    bindingCount > 0 ? t("comic.prompt.referenceCount", { bound: boundBindingCount, total: bindingCount }) : "",
    imageStatusVisible && pipelineStatus
      ? t("comic.prompt.pipelineCount", {
          completed: pipelineCompleted,
          total: pipelinePromptCount || pagePrompts.length,
        })
      : "",
  ].filter(Boolean);
  const activePromptMenuItems: DropdownItem[] = [];
  const activePageMenuItems: DropdownItem[] = [];
  if (activePage?.prompt.orphaned && canEditManga && onDeleteOrphanedPage) {
    activePageMenuItems.push({
      label: t("comic.prompt.deleteOrphanedPage"),
      icon: <Trash2 size={12} />,
      danger: true,
      disabled: actionsDisabled || chapterLocked,
      onClick: () => void onDeleteOrphanedPage(activePage.pageNumber),
    });
  }
  if (
    stageMode === "image_creation" &&
    activePage?.prompt.drawing_prompt &&
    canEditManga &&
    onDeletePrompt
  ) {
    activePageMenuItems.push({ type: "divider" });
    activePageMenuItems.push({
      label: t("comic.prompt.deleteCurrent"),
      icon: <Trash2 size={12} />,
      danger: true,
      disabled: actionsDisabled || chapterLocked || editingPromptId === activePage.prompt.id,
      onClick: () => void onDeletePrompt(activePage.prompt),
    });
  }
  if (
    activePage &&
    activePage.prompt.drawing_prompt &&
    canEditManga &&
    (stageMode == null || stageMode === "image_creation")
  ) {
    if (onPreviewPageImage) {
      activePageMenuItems.push({
        label: t("comic.prompt.viewRequest"),
        icon: <Eye size={12} />,
        disabled:
          actionsDisabled ||
          chapterLocked ||
          generatingPageNumber === activePage.pageNumber ||
          generatingPageNumber != null,
        onClick: () => void onPreviewPageImage(activePage.pageNumber),
      });
    }
    if (
      (stageMode == null || stageMode === "image_creation") &&
      activePage.prompt.generated_image_id != null &&
      onDeleteStageArtifact &&
      !actionsDisabled &&
      !chapterLocked
    ) {
      if (activePageMenuItems.length > 0) activePageMenuItems.push({ type: "divider" });
      activePageMenuItems.push({
        label: t("comic.prompt.deleteGeneratedImage"),
        icon: <Trash2 size={12} />,
        danger: true,
        onClick: () => void deleteStageOutput("image", activePage.pageNumber),
      });
    }
  }
  if (
    activePage &&
    activePage.prompt.drawing_prompt &&
    canEditManga &&
    (stageMode == null || stageMode === "image_creation") &&
    editingPromptId !== activePage.prompt.id
  ) {
    if (onDeleteStageArtifact) {
      activePromptMenuItems.push({
        label: `${t("common.delete")} · ${mangaPageStageLabel("drawing_prompt", t)}`,
        icon: <Trash2 size={12} />,
        danger: true,
        disabled: actionsDisabled || chapterLocked || optimizingActivePrompt,
        onClick: () => void deleteStageOutput("drawing_prompt", activePage.pageNumber),
      });
    }
    if (onOptimizePrompt) {
      if (activePromptMenuItems.length > 0) activePromptMenuItems.push({ type: "divider" });
      activePromptMenuItems.push({
        label: t("comic.prompt.optimize"),
        icon: <Wand2 size={12} />,
        disabled: chapterLocked || actionsDisabled || optimizingActivePrompt,
        onClick: () => void onOptimizePrompt(activePage.prompt),
      });
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      {pagePromptsError ? (
        <div role="alert" className="mb-3 flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{pagePromptsError}</span>
          <BookButton size="sm" variant="secondary" onClick={onRetryPagePrompts}>
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      <div className={`flex shrink-0 flex-wrap items-center justify-between gap-2 ${stageMode === "image_creation" ? "mb-1" : "mb-3"}`}>
        <p
          title={pageSummaryParts.join(" · ")}
          className={`min-w-0 flex-1 text-small leading-relaxed ${stageMode === "image_creation" ? "mt-0 truncate xl:whitespace-normal xl:overflow-visible xl:text-clip" : "mt-1"} ${
            pipelineFailed > 0 || staleCount > 0
              ? "text-warning"
              : pipelineCompleted > 0
                ? "text-book-text-sub"
                : "text-book-text-muted"
          }`}
        >
          {pageSummaryParts.join(" · ")}
        </p>
        {paginationActions}
      </div>

      <div
        className={`flex min-h-0 min-w-0 flex-1 flex-col gap-3 xl:grid xl:grid-rows-1 xl:content-stretch ${
          hasMultiplePages ? "xl:grid-cols-[180px_minmax(0,1fr)]" : "xl:grid-cols-1"
        }`}
      >
        {hasMultiplePages ? (
          <nav
            ref={pageRailRef}
            aria-label={t("comic.prompt.listLabel")}
            aria-describedby="comic-page-list-scroll-hint"
            data-has-before={pageRailHasBefore ? "true" : undefined}
            data-has-more={pageRailHasMore ? "true" : undefined}
            className="a4n-tab-rail-wrap a4n-tab-rail-wrap--canvas a4n-media-page-rail-surface a4n-mobile-scroll-rail flex min-w-0 w-full gap-1.5 overflow-x-auto pb-1 xl:min-h-0 xl:flex-col xl:gap-1 xl:overflow-x-hidden xl:overflow-y-auto xl:overscroll-contain xl:p-1 custom-scrollbar"
          >
            {pagePrompts.map((page) => {
              const selected = page.pageNumber === activePage?.pageNumber;
              const imageLabel = imageStatusVisible
                ? page.prompt.generated_image_id != null
                  ? t("comic.prompt.generatedStatus")
                  : page.prompt.status === "pending"
                    ? t("comic.prompt.pendingStatus")
                    : ""
                : "";
              const pageStage = page.pipelineStage ??
                (imageStatusVisible && page.prompt.status !== "pending" ? "image" : null);
              const pagePipelineStatus = page.pipelineStatus;
              const pageMeta = [
                pageStage ? mangaPageStageLabel(pageStage, t) : "",
                pagePipelineStatus ? statusLabel(pagePipelineStatus, t) : "",
                imageLabel,
                imageStatusVisible && page.prompt.status !== "completed" && page.prompt.status !== "pending"
                  ? statusLabel(page.prompt.status, t)
                  : "",
                page.promptCount > 1 ? t("comic.prompt.recordCount", { count: page.promptCount }) : "",
                page.prompt.orphaned ? t("comic.prompt.orphanedPage") : "",
                page.sourceStale ? t("comic.prompt.staleStatus") : "",
                promptDrafts[page.prompt.id] !== undefined && promptDrafts[page.prompt.id] !== page.prompt.image_prompt
                  ? t("common.unsavedChangesTitle") : "",
              ].filter(Boolean);
              return (
                <button
                  ref={selected ? activePageButtonRef : undefined}
                  key={page.pageNumber}
                  type="button"
                  onClick={() => onSelectedPageChange(page.pageNumber)}
                  aria-current={selected ? "true" : undefined}
                  data-active={selected ? "1" : undefined}
                  className={`a4n-sidebar-item min-w-[8.75rem] shrink-0 text-xs xl:min-w-0 ${stageMode === "image_creation" ? "max-xl:!py-1.5" : ""}`}
                >
                  <span className="block text-xs font-semibold">{t("comic.prompt.pageHeading", { page: page.pageNumber })}</span>
                  <div
                    className={`mt-0.5 truncate text-caption ${
                      selected ? "text-book-primary" : "text-book-text-muted"
                    }`}
                  >
                    {pageMeta.join(" · ")}
                  </div>
                </button>
              );
            })}
            <span id="comic-page-list-scroll-hint" className="sr-only">
              {t("comic.prompt.pageRailScrollHint")}
            </span>
          </nav>
        ) : null}

        <section className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
            {activePage ? (
              <article className="a4n-flat-surface flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
                <header className={`shrink-0 border-b border-book-border/40 ${stageMode === "image_creation" ? "px-1 py-0 sm:px-1.5 sm:py-0.5 xl:p-3" : "p-3"}`}>
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="min-w-0 flex-1 basis-32">
                      <h3 className="text-sm font-semibold text-book-text-main">
                        {t("comic.prompt.pageHeading", { page: activePage.pageNumber })}
                      </h3>
                      <p title={activePageSummary} className={`mt-1 text-small text-book-text-muted ${stageMode === "image_creation" ? "max-xl:hidden xl:truncate" : ""}`}>
                        {activePageSummary}
                      </p>
                    </div>
                    <div className="flex flex-wrap items-center justify-end gap-2">
                      {generatingPageNumber === activePage.pageNumber &&
                      activePage.prompt.generated_image_id != null &&
                      !activePage.sourceStale ? (
                        <span
                          role="status"
                          className="inline-flex min-h-8 items-center gap-1.5 px-1 text-small text-book-text-muted"
                        >
                          <LoadingSpinner size="sm" /> {t("comic.prompt.regenerating")}
                        </span>
                      ) : null}
                      {stageMode == null &&
                      activePage.prompt.drawing_prompt &&
                      canEditManga &&
                      onDeletePrompt ? (
                        <BookButton
                          size="sm"
                          variant="danger"
                          onClick={() => void onDeletePrompt(activePage.prompt)}
                          disabled={actionsDisabled || chapterLocked || editingPromptId === activePage.prompt.id}
                        >
                          <Trash2 size={13} /> {t("comic.prompt.deleteCurrent")}
                        </BookButton>
                      ) : null}
                      {stageMode === "image_creation" ? (
                        <div className="hidden xl:block">{renderEditPromptButton()}</div>
                      ) : stageMode == null ? renderEditPromptButton() : null}
                      {stageMode === "image_creation" ? (
                        <div className="hidden xl:block">{renderGenerateButton(activePage)}</div>
                      ) : renderGenerateButton(activePage)}
                      {activePageMenuItems.length > 0 ? <Dropdown items={activePageMenuItems} /> : null}
                    </div>
                  </div>
                </header>

                {!stageMode && <div className="shrink-0 border-b border-book-border/40 px-3 py-2">
                  <SegmentedControl
                    ariaLabel={t("comic.prompt.pageTitle")}
                    value={activeDetailView}
                    onChange={setDetailView}
                    options={[
                      { id: "prompt", label: t("comic.prompt.promptAndReferences") },
                      { id: "image", label: t("comic.prompt.imageView") },
                    ]}
                  />
                </div>}

                <div className={`min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-contain ${stageMode === "image_creation" ? "px-3 pb-3 pt-1 xl:pt-3" : "p-3"} custom-scrollbar`}>
                  {stageMode === "page_planning" ? (
                    <div className="grid min-w-0 grid-cols-1 gap-4 xl:grid-cols-2">
                      {renderStageArtifact(
                        "page_content",
                        t("comic.prompt.pageContent"),
                        <MangaPageContentDetails page={activePage} />,
                      )}
                      {renderStageArtifact(
                        "page_adaptation",
                        t("comic.prompt.pageAdaptation"),
                        <MangaPageAdaptationDetails page={activePage} />,
                      )}
                    </div>
                  ) : stageMode === "visual_design" ? (
                    <div className="grid min-w-0 grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1.08fr)_minmax(280px,0.92fr)]">
                      {renderStageArtifact(
                        "storyboard",
                        t("comic.pipelineStages.storyboard"),
                        <MangaPageStoryboardDetails page={activePage} />,
                      )}
                      {renderStageArtifact(
                        "reference_discovery",
                        t("comic.pipelineStages.referenceDiscovery"),
                        <ReferenceBindingsPanel
                          key={activePage.prompt.id}
                          page={activePage}
                          profiles={profiles}
                          portraits={portraits}
                          projectImages={projectImages}
                          assetsLoading={assetsLoading}
                          chapterLocked={chapterLocked || actionsDisabled || !canEditManga}
                          selectedCandidateByCharacter={selectedCandidateByCharacter}
                          onSelectCandidate={(bindingKey, value) =>
                            setSelectedCandidateByCharacter((prev) => ({
                              ...prev,
                              [bindingKey]: value,
                            }))
                          }
                          onBindReferenceImage={onBindReferenceImage}
                          onBindDiscoveredReference={onBindDiscoveredReference}
                          onGenerateDiscoveredReference={onGenerateDiscoveredReference}
                          onUnbindDiscoveredReference={onUnbindDiscoveredReference}
                          onUploadReferenceImage={onUploadReferenceImage}
                          uploadingReferenceBindingKey={uploadingReferenceBindingKey}
                        />
                      )}
                    </div>
                  ) : activeDetailView === "image" ? (
                    <MangaPageImagePreview
                      page={activePage}
                      image={activePageImage}
                      loading={assetsLoading}
                    />
                  ) : (
                    <>
                    <div className="grid min-w-0 grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1.08fr)_minmax(280px,0.92fr)]">
                    <section className={`min-w-0 ${stageMode === "image_creation" ? "order-2 xl:order-1" : ""}`}>
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <h3 className="flex items-center gap-1.5 text-xs font-semibold text-book-text-main">
                          <FileText size={12} className="text-book-primary" />
                          {t("comic.pipelineStages.pagePrompt")}
                        </h3>
                        {canEditManga && editingPromptId === activePage.prompt.id ? (
                          <div className="flex items-center gap-1.5">
                            <BookButton
                              size="sm"
                              variant="secondary"
                              onClick={cancelPromptEdit}
                              disabled={actionsDisabled || promptSaving}
                            >
                              <X size={12} />
                              {t("comic.prompt.cancel")}
                            </BookButton>
                            <BookButton
                              size="sm"
                              variant="primary"
                              onClick={() => void savePromptEdit(activePage)}
                              disabled={
                                actionsDisabled ||
                                promptSaving ||
                                !promptDraft.trim() ||
                                promptDraft.trim() === activePage.prompt.image_prompt.trim()
                              }
                              title={activePromptImageRetentionHint}
                            >
                              {promptSaving ? (
                                <LoadingSpinner size="sm" />
                              ) : (
                                <Save size={12} />
                              )}
                              {t("comic.prompt.save")}
                            </BookButton>
                          </div>
                        ) : activePromptMenuItems.length > 0 ? (
                          <Dropdown items={activePromptMenuItems} />
                        ) : null}
                      </div>
                      {activePage.prompt.drawing_prompt && canEditManga && editingPromptId === activePage.prompt.id ? (
                        <textarea
                          aria-label={t("comic.prompt.pageLabel", { page: activePage.pageNumber })}
                          value={promptDraft}
                          onChange={(event) => setPromptDraft(event.target.value)}
                          className="pyqt-input min-h-[260px] max-h-[52vh] w-full resize-y font-mono text-body-sm leading-relaxed"
                          disabled={actionsDisabled || promptSaving}
                        />
                      ) : activePage.prompt.drawing_prompt ? (
                        <pre className="min-h-[220px] whitespace-pre-wrap break-words border-l-2 border-book-border/70 bg-book-bg-tertiary/25 py-2 pl-3 pr-2 text-body-sm leading-relaxed text-book-text-main">
                          {activePage.prompt.image_prompt}
                        </pre>
                      ) : (
                        <p className="min-h-[220px] border-l-2 border-book-border/70 bg-book-bg-tertiary/25 py-3 pl-3 pr-2 text-xs text-book-text-muted">
                          {t("comic.prompt.awaitingDrawingPrompt")}
                        </p>
                      )}
                    </section>

                    {stageMode === "image_creation" ? (
                      <section className="order-1 min-w-0 space-y-1 xl:order-2 xl:space-y-2">
                        <h3 className="text-xs font-semibold text-book-text-main">
                          {t("comic.pipelineStages.pageImage")}
                        </h3>
                        <MangaPageImagePreview
                          page={activePage}
                          image={activePageImage}
                          loading={assetsLoading}
                        />
                        <div className="flex flex-wrap items-center justify-end gap-2 xl:hidden">
                          {renderEditPromptButton()}
                          {renderGenerateButton(activePage)}
                        </div>
                      </section>
                    ) : (
                    <div className="min-w-0 divide-y divide-book-border/40 border-y border-book-border/40">
                      <section className="py-3 first:pt-0">
                        <h3 className="mb-2 flex items-center gap-1.5 text-xs font-semibold text-book-text-sub">
                          <Link2 size={12} />
                          {t("comic.prompt.referenceBindings")}
                        </h3>
                        <ReferenceBindingsPanel
                          key={activePage.prompt.id}
                          page={activePage}
                          profiles={profiles}
                          portraits={portraits}
                          projectImages={projectImages}
                          assetsLoading={assetsLoading}
                          chapterLocked={chapterLocked || actionsDisabled || !canEditManga}
                          selectedCandidateByCharacter={selectedCandidateByCharacter}
                          onSelectCandidate={(bindingKey, value) =>
                            setSelectedCandidateByCharacter((prev) => ({
                              ...prev,
                              [bindingKey]: value,
                            }))
                          }
                          onBindReferenceImage={onBindReferenceImage}
                          onBindDiscoveredReference={onBindDiscoveredReference}
                          onGenerateDiscoveredReference={onGenerateDiscoveredReference}
                          onUnbindDiscoveredReference={onUnbindDiscoveredReference}
                          onUploadReferenceImage={onUploadReferenceImage}
                          uploadingReferenceBindingKey={uploadingReferenceBindingKey}
                        />
                      </section>

                      <section className="py-3">
                        <h3 className="mb-2 flex items-center gap-1.5 text-xs font-semibold text-book-text-sub">
                          <FileText size={12} />
                          {t("comic.prompt.layout")}
                        </h3>
                        <LayoutSummaryGrid items={activePage.layoutItems} />
                      </section>

                      <section className="divide-y divide-book-border/35 py-3">
                        <div className="grid gap-1 px-3 py-2 md:grid-cols-[88px_minmax(0,1fr)] md:gap-3">
                          <div className="flex items-center gap-1.5 text-small font-medium text-book-text-muted">
                            <Users size={11} />
                            {t("comic.prompt.characters")}
                          </div>
                          <div className="text-xs leading-relaxed text-book-text-main">
                            {activePage.characterNames.length > 0
                              ? activePage.characterNames.join(t("common.listSeparator"))
                              : "—"}
                          </div>
                        </div>
                        <div className="grid gap-1 px-3 py-2 md:grid-cols-[88px_minmax(0,1fr)] md:gap-3">
                          <div className="flex items-center gap-1.5 text-small font-medium text-book-text-muted">
                            <MessageSquareText size={11} />
                            {t("comic.prompt.text")}
                          </div>
                          <div className="line-clamp-4 text-xs leading-relaxed text-book-text-main">
                            {activePage.dialogueText ?? "—"}
                          </div>
                        </div>
                      </section>
                    </div>
                    )}
                  </div>
                  </>
                  )}
                </div>
              </article>
            ) : null}
        </section>
        </div>
    </div>
  );
};
