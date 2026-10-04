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
import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type { Portrait } from "../../../api/portrait";
import type {
  BindMangaReferenceImageInput,
  MangaPipelineStatusSummary,
  MangaPagePromptSummary,
} from "../../../api/manga";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { EmptyHint } from "../shared";
import { MangaPageImagePreview } from "./MangaPageImagePreview";
import {
  LayoutSummaryGrid,
  statusLabel,
  type MangaPageDetailView,
} from "./MangaPagePromptListParts";
import { ReferenceBindingsPanel } from "./ReferenceBindingsPanel";
import {
  buildPagePrompts,
  findGeneratedPageImage,
  missingReferenceBindingLabels,
  sumPipelinePrompts,
  type MangaPagePromptLabels,
  type MangaPagePrompt,
} from "./mangaPagePromptModel";
import {
  mangaPageImageBlockReason,
  mangaPageStageLabel,
  type MangaPageWorkspaceStage,
} from "./mangaTabUtils";
import { useI18n } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";
import { getScrollRailCue, scrollRailItemIntoView } from "../../../utils/scrollRail";

interface MangaPagePromptListProps {
  stageMode?: MangaPageWorkspaceStage;
  onDirtyChange?: (dirty: boolean) => void;
  prompts: MangaPagePromptSummary[];
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
    input: Omit<BindMangaReferenceImageInput, "downstream_strategy">,
  ) => void | Promise<void>;
  onUpdatePrompt?: (promptId: number, imagePrompt: string) => void | Promise<void>;
  onOptimizePrompt?: (prompt: MangaPagePromptSummary) => void | Promise<void>;
  onDeletePrompt?: (prompt: MangaPagePromptSummary) => void | Promise<void>;
  onRetryPagePrompts: () => void;
}

type MangaPagePlanView = "page_content" | "page_adaptation" | "storyboard";

const pageDetailItems = (
  rows: Array<[string, string, string | null | undefined]>,
) =>
  rows
    .filter(([, , value]) => value != null && value.trim() !== "")
    .map(([key, label, value]) => ({ key, label, value: value ?? "" }));

const MangaPageContentDetails: React.FC<{ page: MangaPagePrompt }> = ({ page }) => {
  const { t } = useI18n();
  const content = page.pageContent;
  return (
    <section className="space-y-2">
      <h3 className="text-xs font-semibold text-book-text-main">
        {t("comic.prompt.pageContent")}
      </h3>
      {content ? (
        <LayoutSummaryGrid
          items={pageDetailItems([
            ["summary", t("comic.prompt.pageContent"), content.summary],
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
      )}
    </section>
  );
};

const MangaPageAdaptationDetails: React.FC<{ page: MangaPagePrompt }> = ({ page }) => {
  const { t } = useI18n();
  const adaptation = page.pageAdaptation;
  return (
    <section className="space-y-2">
      <h3 className="text-xs font-semibold text-book-text-main">
        {t("comic.prompt.pageAdaptation")}
      </h3>
      {adaptation ? (
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
      )}
    </section>
  );
};

const MangaPageStoryboardDetails: React.FC<{ page: MangaPagePrompt }> = ({ page }) => {
  const { t } = useI18n();
  const storyboard = page.prompt.page_plan;
  return (
    <section className="space-y-3">
      <h3 className="text-xs font-semibold text-book-text-main">
        {t("comic.pipelineStages.storyboard")}
      </h3>
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

const MangaPagePlanDetails: React.FC<{
  page: MangaPagePrompt;
  view: MangaPagePlanView;
}> = ({ page, view }) => {
  if (view === "page_content") return <MangaPageContentDetails page={page} />;
  if (view === "page_adaptation") return <MangaPageAdaptationDetails page={page} />;
  return <MangaPageStoryboardDetails page={page} />;
};

export const MangaPagePromptList: React.FC<MangaPagePromptListProps> = ({
  stageMode,
  onDirtyChange,
  prompts,
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
  onUpdatePrompt,
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
    () => buildPagePrompts(prompts, currentContentHash, pagePromptLabels),
    [currentContentHash, pagePromptLabels, prompts],
  );
  const pageRailRef = useRef<HTMLElement | null>(null);
  const activePageButtonRef = useRef<HTMLButtonElement | null>(null);
  const [pageRailHasBefore, setPageRailHasBefore] = useState(false);
  const [pageRailHasMore, setPageRailHasMore] = useState(false);
  const [detailView, setDetailView] = useState<MangaPageDetailView | null>(null);
  const [promptDrafts, setPromptDrafts] = useState<Record<number, string>>({});
  const [promptSaving, setPromptSaving] = useState(false);
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
  const activeDetailView: MangaPageDetailView = stageMode === "page_plan"
    ? "page_content"
    : stageMode === "page_adaptation"
      ? "page_adaptation"
      : stageMode === "storyboard"
        ? "storyboard"
        : stageMode === "prompts"
          ? "prompt"
          : stageMode === "images"
            ? "image"
            : detailView ?? (activePage?.prompt.generated_image_id != null ? "image" : "prompt");
  const editingPromptId = activePage && promptDrafts[activePage.prompt.id] !== undefined ? activePage.prompt.id : null;
  const promptDraft = editingPromptId == null ? "" : promptDrafts[editingPromptId];
  const promptDraftDirty = Object.entries(promptDrafts).some(([id, value]) => {
    const prompt = prompts.find((item) => item.id === Number(id));
    return prompt != null && value !== prompt.image_prompt;
  });
  useEffect(() => { onDirtyChange?.(promptDraftDirty); }, [promptDraftDirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  useNovelDetailDirty(`manga-prompts:${prompts.map((prompt) => prompt.id).join(",")}`, promptDraftDirty);
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
      sum + page.referenceBindings.filter((binding) => binding.boundAsset != null).length,
    0,
  );
  const missingBindingCount = pagePrompts.reduce(
    (sum, page) => sum + missingReferenceBindingLabels(page).length,
    0,
  );
  const pipelinePromptCount = sumPipelinePrompts(pipelineStatus);
  const pipelineCompleted = pipelineStatus?.completed ?? 0;
  const pipelineFailed = pipelineStatus?.failed ?? 0;
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
        {pagePromptsError ? (
          <div role="alert" className="flex flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
            <span className="min-w-0 flex-1">{pagePromptsError}</span>
            <BookButton size="sm" variant="secondary" onClick={onRetryPagePrompts}>
              <RefreshCw size={12} /> {t("common.retry")}
            </BookButton>
          </div>
        ) : null}
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyHint text={t("comic.prompt.empty")} />
        </div>
      </div>
    );
  }

  const renderGenerateButton = (page: MangaPagePrompt) => {
    if (!canEditManga || !onGeneratePageImage || !page.prompt.drawing_prompt) return null;
    const generating = generatingPageNumber === page.pageNumber;
    const generated = page.prompt.generated_image_id != null;
    const missingBindings = missingReferenceBindingLabels(page);
    const imageBlockReason = mangaPageImageBlockReason(page.prompt, t);
    if (generated && !page.sourceStale) return null;
    return (
      <BookButton
        size="sm"
        variant={generated ? "secondary" : "primary"}
        onClick={() => void onGeneratePageImage(page.pageNumber)}
        disabled={
          actionsDisabled ||
          chapterLocked ||
          generating ||
          generatingPageNumber != null ||
          missingBindings.length > 0
        }
        title={
          missingBindings.length > 0
            ? t("comic.prompt.missingBindings", { items: missingBindings.join(t("common.listSeparator")) })
            : imageBlockReason ?? undefined
        }
      >
        {generating ? (
          <LoadingSpinner size="sm" />
        ) : generated ? (
          <RefreshCw size={13} />
        ) : (
          <ImagePlus size={13} />
        )}
        {generated ? t("common.update") : t("comic.prompt.generateImage")}
      </BookButton>
    );
  };

  const beginPromptEdit = (page: MangaPagePrompt) => {
    if (!canEditManga) return;
    setDetailView("prompt");
    setPromptDrafts((current) => ({ ...current, [page.prompt.id]: page.prompt.image_prompt }));
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

  const activePromptDownstreamHint =
    activePage?.prompt.generated_image_id != null
      ? t("comic.prompt.downstreamImage")
      : undefined;
  const optimizingActivePrompt =
    activePage != null && optimizingPromptId === activePage.prompt.id;
  const activeMissingBindings = activePage
    ? missingReferenceBindingLabels(activePage)
    : [];
  const activePageImageBlocked = activeMissingBindings.length > 0;
  const pageSummaryParts = [
    t("comic.prompt.pageCount", { count: pagePrompts.length }),
    t("comic.prompt.generatedCount", { linked: linkedImageCount, total: pagePrompts.length }),
    bindingCount > 0 ? t("comic.prompt.referenceCount", { bound: boundBindingCount, total: bindingCount }) : "",
    missingBindingCount > 0
      ? t("comic.prompt.missingReferenceCount", { count: missingBindingCount })
      : "",
    pipelineStatus
      ? t("comic.prompt.pipelineCount", {
          completed: pipelineCompleted,
          total: pipelinePromptCount || pagePrompts.length,
        })
      : "",
    pipelineFailed > 0 ? t("comic.prompt.failedCount", { count: pipelineFailed }) : "",
    pipelineStatus?.has_checkpoint ? t("comic.prompt.checkpoint", { status: pipelineCheckpointLabel }) : "",
    staleCount > 0 ? t("comic.prompt.staleCount", { count: staleCount }) : "",
  ].filter(Boolean);
  const activePromptMenuItems: DropdownItem[] = [];
  const activePageMenuItems: DropdownItem[] = [];
  if (
    activePage &&
    activePage.prompt.drawing_prompt &&
    canEditManga &&
    (stageMode == null || stageMode === "images" || stageMode === "prompts")
  ) {
    if (onPreviewPageImage) {
      activePageMenuItems.push({
        label: t("comic.prompt.viewRequest"),
        icon: <Eye size={12} />,
        disabled:
          actionsDisabled ||
          chapterLocked ||
          activePageImageBlocked ||
          generatingPageNumber === activePage.pageNumber ||
          generatingPageNumber != null,
        onClick: () => void onPreviewPageImage(activePage.pageNumber),
      });
    }
    if (
      onGeneratePageImage &&
      activePage.prompt.generated_image_id != null &&
      !activePage.sourceStale
    ) {
      activePageMenuItems.push({
        label: t("comic.prompt.regenerateImage"),
        icon: <RefreshCw size={12} />,
        disabled:
          actionsDisabled ||
          chapterLocked ||
          activePageImageBlocked ||
          generatingPageNumber === activePage.pageNumber ||
          generatingPageNumber != null,
        onClick: () => void onGeneratePageImage(activePage.pageNumber),
      });
    }
    if (onDeletePrompt && !actionsDisabled && !chapterLocked) {
      if (activePageMenuItems.length > 0) activePageMenuItems.push({ type: "divider" });
      activePageMenuItems.push({
        label: t("comic.prompt.deleteCurrent"),
        icon: <Trash2 size={12} />,
        danger: true,
        onClick: () => void onDeletePrompt(activePage.prompt),
      });
    }
  }
  if (
    activePage &&
    activePage.prompt.drawing_prompt &&
    canEditManga &&
    (stageMode == null || stageMode === "prompts") &&
    editingPromptId !== activePage.prompt.id
  ) {
    if (onOptimizePrompt) {
      activePromptMenuItems.push({
        label: t("comic.prompt.optimize"),
        icon: <Wand2 size={12} />,
        disabled: chapterLocked || actionsDisabled || optimizingActivePrompt,
        onClick: () => void onOptimizePrompt(activePage.prompt),
      });
    }
    if (onUpdatePrompt) {
      activePromptMenuItems.push({
        label: t("comic.prompt.edit"),
        icon: <Pencil size={12} />,
        disabled: chapterLocked || actionsDisabled,
        onClick: () => beginPromptEdit(activePage),
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
      <div className="mb-3 shrink-0">
        <p
          className={`mt-1 text-small leading-relaxed ${
            pipelineFailed > 0 || staleCount > 0
              ? "text-warning"
              : pipelineCompleted > 0
                ? "text-book-text-sub"
                : "text-book-text-muted"
          }`}
        >
          {pageSummaryParts.join(" · ")}
        </p>
      </div>

      <div
        className={`grid min-h-0 grid-cols-1 content-start gap-3 xl:flex-1 xl:content-stretch ${
          hasMultiplePages ? "xl:grid-cols-[180px_minmax(0,1fr)]" : ""
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
              const imageLabel = page.prompt.generated_image_id != null
                ? t("comic.prompt.generatedStatus")
                : t("comic.prompt.pendingStatus");
              const pageStatus = statusLabel(page.prompt.status, t);
              const pageMeta = [
                page.pipelineStage ? mangaPageStageLabel(page.pipelineStage, t) : "",
                page.pipelineStatus ? statusLabel(page.pipelineStatus, t) : "",
                imageLabel,
                page.prompt.status !== "completed" && page.prompt.status !== "pending" ? pageStatus : "",
                page.promptCount > 1 ? t("comic.prompt.recordCount", { count: page.promptCount }) : "",
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
                  className="a4n-sidebar-item min-w-[8.75rem] shrink-0 text-xs xl:min-w-0"
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

        <section className="min-h-0 min-w-0 overflow-visible xl:overflow-y-auto xl:overscroll-contain custom-scrollbar">
            {activePage ? (
              <article className="a4n-flat-surface min-w-0 overflow-hidden">
                <header className="border-b border-book-border/40 p-3">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="min-w-0 flex-1 basis-32">
                      <h3 className="text-sm font-semibold text-book-text-main">
                        {t("comic.prompt.pageHeading", { page: activePage.pageNumber })}
                      </h3>
                      <p className="mt-1 text-small text-book-text-muted">
                        {[
                          activePage.pipelineStage
                            ? mangaPageStageLabel(activePage.pipelineStage, t)
                            : "",
                          statusLabel(activePage.pipelineStatus ?? activePage.prompt.status, t),
                          activePage.prompt.generated_image_id != null ? t("comic.prompt.hasImage") : t("comic.prompt.noImage"),
                          activePage.sourceStale ? t("comic.prompt.sourceStale") : "",
                          activePage.promptCount > 1 ? t("comic.prompt.recordCount", { count: activePage.promptCount }) : "",
                        ].filter(Boolean).join(" · ")}
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
                      {activePageMenuItems.length > 0 ? <Dropdown items={activePageMenuItems} /> : null}
                      {renderGenerateButton(activePage)}
                    </div>
                  </div>
                </header>

                {activePage.pipelineError && (
                  <div
                    role="alert"
                    className="mx-3 mt-3 rounded-lg border border-error/35 bg-error/10 px-3 py-2 text-xs text-error"
                  >
                    <p className="font-semibold">
                      {t("comic.prompt.pipelineError")} · {mangaPageStageLabel(activePage.pipelineError.stage, t)}
                    </p>
                    <p className="mt-1 whitespace-pre-wrap break-words">
                      {activePage.pipelineError.message}
                    </p>
                  </div>
                )}

                {!stageMode && <div className="border-b border-book-border/40 px-3 py-2">
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

                <div className="p-3">
                  {activeDetailView === "image" ? (
                    <MangaPageImagePreview
                      page={activePage}
                      image={activePageImage}
                      loading={assetsLoading}
                    />
                  ) : activeDetailView === "page_content" ||
                    activeDetailView === "page_adaptation" ||
                    activeDetailView === "storyboard" ? (
                    <MangaPagePlanDetails page={activePage} view={activeDetailView} />
                  ) : (
                    <div className="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1.08fr)_minmax(280px,0.92fr)]">
                    <section className="min-w-0">
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
                              title={activePromptDownstreamHint}
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

                    <div className="min-w-0 divide-y divide-book-border/40 border-y border-book-border/40">
                      <section className="py-3 first:pt-0">
                        <h3 className="mb-2 flex items-center gap-1.5 text-xs font-semibold text-book-text-sub">
                          <Link2 size={12} />
                          {t("comic.prompt.referenceBindings")}
                        </h3>
                        <ReferenceBindingsPanel
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
                  </div>
                  )}
                </div>
              </article>
            ) : null}
        </section>
        </div>
    </div>
  );
};
