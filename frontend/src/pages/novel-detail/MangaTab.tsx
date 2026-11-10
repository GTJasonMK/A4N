import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Clapperboard, Eye, FileText, ImagePlus, Images, PanelTop, Pencil, RefreshCw, Save, Trash2, X } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import { isPositiveSafeInteger, isSafeInteger } from "../../utils/numberRange";
import { getChapter, listChapterIndex } from "../../api/chapter";
import {
  getSourceSnapshotChapter,
  type SourceSnapshotChapterIndex,
  type SourceSnapshotChapterSummary,
} from "../../api/project";
import { listProfiles, type CharacterProfile } from "../../api/character";
import {
  listImagesByProject,
  listImagesByProjects,
  type GeneratedImage,
} from "../../api/image";
import {
  listPortraitsByProject,
  startPortraitGeneration,
  verifyPortraitGenerationResult,
  type Portrait,
} from "../../api/portrait";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { generationClient } from "../../api/generationTasks";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import {
  getMangaPipelineStatus,
  listMangaPagePrompts,
  listMangaPipelineArtifacts,
  listMangaPipelineStatuses,
  previewResumeMangaPipelineRequest,
  previewStartMangaPipelineRequest,
  prepareStandaloneComicSource,
  resumeMangaPipeline,
  startMangaPipeline,
  type MangaPipelineStatusSummary,
  type MangaPagePromptSummary,
} from "../../api/manga";
import {
  deleteMangaPipelineArtifact,
  previewDeleteMangaPipelineArtifact,
  saveMangaPipelineArtifact,
} from "../../api/manga/pipelineArtifacts";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  chapterGeneratingMessage,
  EmptyHint,
  type ChapterRow,
  isChapterGenerating,
} from "./shared";
import { sha256Hex } from "../../utils/contentHash";
import { isCancelledOperation } from "../../utils/cancellation";
import { MangaChapterDetailPanel } from "./manga/MangaChapterDetailPanel";
import { MangaChapterList } from "./manga/MangaChapterList";
import {
  MangaPageSlotsArtifactEditor,
  validateMangaPageSlotsDraft,
} from "./manga/MangaPipelineArtifactEditor";
import { MangaPreprocessingEditor } from "./manga/MangaPreprocessingEditor";
import { MangaPreprocessingView } from "./manga/MangaPreprocessingView";
import { useProgressTask } from "../../hooks/useProgressTask";
import { usePersistedTab } from "../../hooks/usePersistedTab";
import { useNovelDetailDirty, useNovelDetailDirtyRegistry } from "./novelDetailDirty";
import { MediaStageHeader } from "../../components/shell/MediaStageHeader";
import { confirmDiscardMediaChanges } from "../../components/feedback/UnsavedChangesGuard";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { MediaAuthoringPanel } from "../derived/MediaAuthoringPanel";
import { useMediaAuthoring, authoringStageLabel } from "../derived/useMediaAuthoring";
import { BaseMaterialsWorkspace } from "../derived/BaseMaterialsWorkspace";
import type {
  MangaPipelineArtifactRecord,
  MangaPipelineArtifactStage,
  MangaPipelineStopAfter,
} from "../../api/generated/host";
import { useI18n } from "../../i18n";
import { useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { explainMangaPipelineAiConfigError } from "./manga/mangaPipelineErrors";
import {
  mangaPipelineConfigBlockReason,
  type MangaTabProps,
} from "./manga/mangaPipelineProgress";
import {
  chapterHeading,
  countStaleMangaPagePrompts,
  isMangaPageWorkspaceStage,
  mangaDeletionDownstreamSummary,
  mangaWorkspaceDeletionStage,
  mangaChapterRowFromIndex,
  mangaChapterRowFromSummary,
  mangaStageLabel,
  type MangaChapterArtifactStage,
  mangaPageStageNeedsGeneration,
  type MangaPageWorkspaceStage,
} from "./manga/mangaTabUtils";

type MangaTextSourceSelection = import("../derived/ProjectTextMaterialsPanel").ProjectTextSourceSelection;
type MangaChapterArtifactChange =
  | { type: "saved"; artifact: MangaPipelineArtifactRecord }
  | { type: "deleted"; artifactIds: number[] };

function applyMangaPageArtifactToPrompt(
  prompt: MangaPagePromptSummary,
  artifact: MangaPipelineArtifactRecord,
): MangaPagePromptSummary {
  if (artifact.page_number == null || prompt.page_number !== artifact.page_number) return prompt;
  const staleStages = prompt.stale_stages.filter((stage) => stage !== artifact.stage);
  switch (artifact.stage) {
    case "page_content":
      return { ...prompt, page_content: artifact.payload as MangaPagePromptSummary["page_content"], stale_stages: staleStages };
    case "page_adaptation":
      return { ...prompt, page_adaptation: artifact.payload as MangaPagePromptSummary["page_adaptation"], stale_stages: staleStages };
    case "storyboard":
      return { ...prompt, page_plan: artifact.payload as MangaPagePromptSummary["page_plan"], stale_stages: staleStages };
    case "reference_discovery":
      return { ...prompt, reference_discovery: artifact.payload as MangaPagePromptSummary["reference_discovery"], stale_stages: staleStages };
    case "drawing_prompt":
      return { ...prompt, drawing_prompt: artifact.payload as MangaPagePromptSummary["drawing_prompt"], stale_stages: staleStages };
    default:
      return prompt;
  }
}

function applyMangaChapterArtifactChange(
  current: MangaPipelineArtifactRecord[],
  change: MangaChapterArtifactChange,
): MangaPipelineArtifactRecord[] {
  if (change.type === "saved") {
    return [
      ...current.filter((item) => item.id !== change.artifact.id),
      change.artifact,
    ];
  }
  const removed = new Set(change.artifactIds);
  return current.filter((item) => !removed.has(item.id));
}

type MangaPipelineStatusLoadError =
  | { scope: "project"; message: string }
  | { scope: "chapter"; chapterId: number; message: string };

function snapshotIndexChapterRow(chapter: SourceSnapshotChapterIndex): ChapterRow {
  return {
    id: chapter.source_chapter_id,
    chapter_number: chapter.chapter_number,
    title: chapter.title,
    content: "",
    content_hash: null,
    summary: null,
    status: "completed",
    analysis_data: null,
  };
}

function snapshotChapterRow(chapter: SourceSnapshotChapterSummary): ChapterRow {
  return {
    id: chapter.source_chapter_id,
    chapter_number: chapter.chapter_number,
    title: chapter.title,
    content: chapter.content,
    content_hash: chapter.content_hash,
    summary: chapter.summary,
    status: "completed",
    analysis_data: chapter.analysis_data_json,
  };
}

function readMangaTextSource(projectId: number): MangaTextSourceSelection | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(`a4n:comic:${projectId}:source`) ?? "null");
    if (!value || typeof value !== "object") return null;
    const source = value as Partial<MangaTextSourceSelection>;
    if (
      !isPositiveSafeInteger(source.assetId) ||
      !isPositiveSafeInteger(source.version) ||
      !isSafeInteger(source.startSection) || source.startSection! < 0 ||
      !isSafeInteger(source.endSection) || source.endSection! < source.startSection!
    ) return null;
    return source as MangaTextSourceSelection;
  } catch {
    return null;
  }
}

const chapterArtifactLabel = (
  stage: MangaChapterArtifactStage,
  t: ReturnType<typeof useI18n>["t"],
): string => {
  switch (stage) {
    case "page_slots": return t("comic.pipelineStages.pagination");
    case "preprocessing": return t("comic.pipelineStages.preprocessing");
  }
};

const MangaChapterArtifactPanel: React.FC<{
  projectId: number;
  chapterId: number;
  stage: MangaChapterArtifactStage;
  artifact: import("../../api/generated/host").MangaPipelineArtifactRecord | null;
  canEdit: boolean;
  onGenerate?: () => void;
  onPreview?: () => void;
  resumable?: boolean;
  onBusyChange?: (busy: boolean) => void;
  onChanged: (change: MangaChapterArtifactChange) => Promise<void>;
  compact?: boolean;
}> = ({ projectId, chapterId, stage, artifact, canEdit, onGenerate, onPreview, resumable = false, onBusyChange, onChanged, compact = false }) => {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const [editing, setEditing] = useState(false);
  const [structuredDraft, setStructuredDraft] = useState<unknown>(() => artifact?.payload ?? {});
  const [busy, setBusy] = useState(false);
  useNovelDetailDirty(`manga-artifact:${projectId}:${chapterId}:${stage}`,
    editing && JSON.stringify(structuredDraft) !== JSON.stringify(artifact?.payload ?? {}),
    () => { if (busy) return false; setEditing(false); setStructuredDraft(artifact?.payload ?? {}); },
    () => !busy);
  useEffect(() => { onBusyChange?.(busy); return () => onBusyChange?.(false); }, [busy, onBusyChange]);
  const label = chapterArtifactLabel(stage, t);
  const validationError = stage === "page_slots"
    ? validateMangaPageSlotsDraft(structuredDraft, t)
    : null;

  React.useEffect(() => {
    if (!editing) {
      const payload = artifact?.payload ?? {};
      setStructuredDraft(payload);
    }
  }, [artifact, editing]);

  const beginEdit = () => {
    const payload = artifact?.payload ?? {};
    setStructuredDraft(payload);
    setEditing(true);
  };

  const save = async () => {
    if (validationError) {
      addToast(validationError, "error");
      return;
    }
    setBusy(true);
    try {
      const savedArtifact = await saveMangaPipelineArtifact({
        project_id: projectId,
        chapter_id: chapterId,
        stage,
        page_number: null,
        payload: structuredDraft,
      });
      await onChanged({ type: "saved", artifact: savedArtifact });
      setEditing(false);
      addToast(t("comic.prompt.saved"), "success");
    } catch (error) {
      addToast(extractErrorMessage(error, t("comic.prompt.saveFailed")), "error");
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    if (!artifact) return;
    setBusy(true);
    try {
      const preview = await previewDeleteMangaPipelineArtifact({
        project_id: projectId,
        chapter_id: chapterId,
        stage,
        page_number: null,
      });
      const downstream = mangaDeletionDownstreamSummary(preview, t);
      const confirmed = await confirmDialog({
        title: t("comic.prompt.deleteStageTitle"),
        message: downstream
          ? t("comic.prompt.deleteChapterStageCascadeMessage", {
              stage: mangaStageLabel(stage, t),
              downstream,
            })
          : t("comic.prompt.deleteChapterStageMessage", {
              stage: mangaStageLabel(stage, t),
            }),
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!confirmed) return;
      const artifactIds = [
        preview.target.artifact_id,
        ...preview.downstream.map((item) => item.artifact_id),
      ].filter((id): id is number => id != null);
      await deleteMangaPipelineArtifact({
        project_id: projectId,
        chapter_id: chapterId,
        stage,
        page_number: null,
        expected_impact_fingerprint: preview.fingerprint,
      });
      await onChanged({ type: "deleted", artifactIds });
      addToast(t("comic.prompt.deleted"), "success");
    } catch (error) {
      addToast(extractErrorMessage(error, t("comic.prompt.deleteFailed")), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    compact ? (
      canEdit && artifact ? (
        <div className={`flex min-w-0 flex-wrap items-center justify-end gap-2 ${editing ? "w-full" : ""}`}>
          {editing ? (
            <>
              <div className="flex w-full justify-end gap-2">
                <BookButton size="sm" variant="secondary" onClick={() => setEditing(false)} disabled={busy}>
                  <X size={12} /> {t("comic.prompt.cancel")}
                </BookButton>
                <BookButton size="sm" variant="primary" onClick={() => void save()} disabled={busy || validationError != null}>
                  {busy ? <LoadingSpinner size="sm" /> : <Save size={12} />} {t("comic.prompt.save")}
                </BookButton>
              </div>
              {stage === "preprocessing" ? (
                <div className="max-h-[60vh] w-full overflow-y-auto">
                  <MangaPreprocessingEditor value={structuredDraft} onChange={setStructuredDraft} disabled={busy} />
                </div>
              ) : (
                <div className="max-h-[60vh] w-full overflow-y-auto">
                  <MangaPageSlotsArtifactEditor
                    value={structuredDraft}
                    onChange={setStructuredDraft}
                    disabled={busy}
                  />
                </div>
              )}
              {validationError ? (
                <p role="alert" className="w-full text-small text-error">{validationError}</p>
              ) : null}
            </>
          ) : (
            <Dropdown
              ariaLabel={t("comic.prompt.pageSlotsActions")}
              items={[
                {
                  label: t("comic.prompt.editPageSlots"),
                  icon: <Pencil size={12} />,
                  onClick: beginEdit,
                },
                { type: "divider" },
                {
                  label: t("comic.prompt.deletePageSlots"),
                  icon: <Trash2 size={12} />,
                  danger: true,
                  onClick: () => void remove(),
                },
              ]}
            />
          )}
        </div>
      ) : null
    ) : (
    <section className={`a4n-flat-surface flex min-h-0 flex-col overflow-hidden ${compact ? "shrink-0" : "flex-1"}`}>
      <header className={`flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-book-border/45 ${compact ? "p-2" : "p-3"}`}>
        <div>
          <h2 className="text-sm font-semibold text-book-text-main">{label}</h2>
          {!compact ? <p className="mt-1 text-small text-book-text-muted">
            {artifact?.stale ? t("common.mediaAuthoring.needsUpdate") : artifact ? t("comic.prompt.available") : t("comic.pipelineStages.notStarted")}
          </p> : null}
        </div>
        {canEdit ? (
          <div className="flex flex-wrap items-center gap-1.5">
            {stage === "preprocessing" && onGenerate && !artifact ? (
              <>
                {onPreview ? (
                  <BookButton size="sm" variant="secondary" onClick={onPreview} disabled={busy}>
                    <Eye size={12} /> {t(resumable ? "comic.prompt.viewContinueRequest" : "comic.prompt.viewGenerateRequest")}
                  </BookButton>
                ) : null}
                <BookButton size="sm" variant="primary" onClick={onGenerate} disabled={busy}>
                  <RefreshCw size={12} /> {resumable ? t("comic.continueToStage", { stage: label }) : t("comic.prompt.generatePreprocessing")}
                </BookButton>
              </>
            ) : null}
            {editing ? (
              <>
                <BookButton size="sm" variant="secondary" onClick={() => setEditing(false)} disabled={busy}>
                  <X size={12} /> {t("comic.prompt.cancel")}
                </BookButton>
                <BookButton size="sm" variant="primary" onClick={() => void save()} disabled={busy}>
                  {busy ? <LoadingSpinner size="sm" /> : <Save size={12} />} {t("comic.prompt.save")}
                </BookButton>
              </>
            ) : artifact ? (
              <BookButton size="sm" variant="secondary" onClick={beginEdit} disabled={busy}>
                <Pencil size={12} /> {t("comic.prompt.editArtifact")}
              </BookButton>
            ) : null}
            {!editing && artifact ? (
              <BookButton size="sm" variant="secondary" onClick={() => void remove()} disabled={busy}>
                <Trash2 size={12} /> {t("common.delete")}
              </BookButton>
            ) : null}
          </div>
        ) : null}
      </header>
      {compact && !editing ? null : <div className={`min-h-0 flex-1 overflow-y-auto custom-scrollbar ${compact ? "p-2" : "p-3"}`}>
        {editing ? (
          stage === "preprocessing" ? (
            <MangaPreprocessingEditor value={structuredDraft} onChange={setStructuredDraft} disabled={busy} />
          ) : (
            <MangaPageSlotsArtifactEditor
              value={structuredDraft}
              onChange={setStructuredDraft}
              disabled={busy}
            />
          )
        ) : artifact ? (
          stage === "preprocessing" ? (
            <MangaPreprocessingView payload={artifact.payload} />
          ) : (
            <MangaPageSlotsArtifactEditor
              value={artifact.payload}
              onChange={() => undefined}
              disabled
            />
          )
        ) : (
          <p className="text-small text-book-text-muted">{t("comic.pipelineStages.notStarted")}</p>
        )}
      </div>}
    </section>
    )
  );
};

export const MangaTab: React.FC<MangaTabProps> = ({
  projectId,
  canEditManga = true,
  snapshotChapters,
  assetProjectId,
  imageProjectIds,
  standalone = false,
  fixedChapter,
  onDirtyChange,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const staged = snapshotChapters !== undefined || standalone;
  const [authoringDirty, setAuthoringDirty] = useState(false);
  const [textMaterialsDirty, setTextMaterialsDirty] = useState(false);
  const [selectedTextSource, setSelectedTextSource] = useState<MangaTextSourceSelection | null>(
    () => standalone ? readMangaTextSource(projectId) : null,
  );
  const [authoringBusy, setAuthoringBusy] = useState(false);
  const [standaloneSourceLoading, setStandaloneSourceLoading] = useState(false);
  const [promptDirty, setPromptDirty] = useState(false);
  const [discardVersion, setDiscardVersion] = useState(0);
  const discardStageDrafts = () => {
    setAuthoringDirty(false);
    setTextMaterialsDirty(false);
    setPromptDirty(false);
    setDiscardVersion((version) => version + 1);
  };
  const isMutationBusy = () =>
    actionBusyKeyRef.current != null ||
    pipelineChapterId != null ||
    progressCancelBusyKeyRef.current != null ||
    authoringBusy ||
    standaloneSourceLoading;
  useEffect(() => { onDirtyChange?.(authoringDirty || promptDirty || textMaterialsDirty); }, [authoringDirty, promptDirty, textMaterialsDirty, onDirtyChange]);
  useEffect(() => {
    if (!standalone) return;
    try {
      const key = `a4n:comic:${projectId}:source`;
      if (selectedTextSource) localStorage.setItem(key, JSON.stringify(selectedTextSource));
      else localStorage.removeItem(key);
    } catch {
      addToast(t("comic.sourceSelectionSaveFailed"), "error");
    }
  }, [addToast, projectId, selectedTextSource, standalone, t]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  const discardMangaDrafts = () => {
    if (isMutationBusy() || acting != null) return false;
    discardStageDrafts();
  };
  useNovelDetailDirty(`manga-editor:${projectId}`, authoringDirty || promptDirty || textMaterialsDirty, discardMangaDrafts, () => !isMutationBusy() && acting == null);
  const stageId = `comic-stages-${projectId}`;
  const confirmLeaveStage = async () => {
    if (isMutationBusy()) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    const stageDraftDirty = authoringDirty || promptDirty || textMaterialsDirty;
    if (!stageDraftDirty && !dirtyRegistry.hasDirty()) return true;
    if (!dirtyRegistry.canDiscardDirty()) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    const confirmed = await confirmDiscardMediaChanges(true, t);
    if (!confirmed) return false;
    const mangaDraftOwnerDirty = dirtyRegistry.hasDirty(`manga-editor:${projectId}`);
    if (!await dirtyRegistry.discardDirty()) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    if (stageDraftDirty && !mangaDraftOwnerDirty) discardStageDrafts();
    return true;
  };
  const { runProgressTask } = useProgressTask();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    size: imageSize,
    sizes: imageSizes,
    setSize: setImageSize,
  } = useImageGenerationSize();
  const hasFixedChapterContext = fixedChapter !== undefined;
  const [chapters, setChapters] = useState<ChapterRow[]>([]);
  const [statuses, setStatuses] = useState<
    Record<number, MangaPipelineStatusSummary | null>
  >({});
  const [pipelineStatusError, setPipelineStatusError] =
    useState<MangaPipelineStatusLoadError | null>(null);
  const [loading, setLoading] = useState(true);
  const [chapterLoadError, setChapterLoadError] = useState<string | null>(null);
  const [snapshotChapterLoading, setSnapshotChapterLoading] = useState(false);
  const [selectedChapterLoadError, setSelectedChapterLoadError] = useState<string | null>(null);
  const [acting, setActing] = useState<string | null>(null);
  const [selectedChapter, setSelectedChapter] = useState<ChapterRow | null>(
    fixedChapter ?? null,
  );
  const authoring = useMediaAuthoring(
    projectId,
    staged && (!standalone || selectedChapter != null),
    selectedChapter?.id ?? null,
  );
  const outputState = authoring.workspace?.artifacts.find((artifact) => artifact.kind === "manga_pages" && artifact.source_chapter_id === selectedChapter?.id);
  const [pagePrompts, setPagePrompts] = useState<MangaPagePromptSummary[]>([]);
  const [pagePromptsLoading, setPagePromptsLoading] = useState(false);
  const [pagePromptsError, setPagePromptsError] = useState<string | null>(null);
  const [chapterArtifacts, setChapterArtifacts] = useState<MangaPipelineArtifactRecord[]>([]);
  const [chapterArtifactsLoading, setChapterArtifactsLoading] = useState(false);
  const [chapterArtifactsLoadError, setChapterArtifactsLoadError] = useState<string | null>(null);
  const [selectedPageNumber, setSelectedPageNumber] = useState<number | null>(null);
  const [currentContentHash, setCurrentContentHash] = useState<string | null>(
    null,
  );
  const preprocessingArtifact = chapterArtifacts.find(
    (artifact) => artifact.stage === "preprocessing",
  );
  const pageSlotsArtifact = chapterArtifacts.find(
    (artifact) => artifact.stage === "page_slots",
  );
  const preprocessingStale = Boolean(preprocessingArtifact?.stale);
  const pageSlotsNeedsGeneration = !pageSlotsArtifact || pageSlotsArtifact.stale;
  const stageTabs = useMemo(() => {
    const stageProgress = (ready: number) =>
      pagePrompts.length > 0
        ? t("comic.prompt.stageProgress", {
            ready,
            total: pagePrompts.length,
          })
        : undefined;
    const pagePromptsStale = pagePrompts.length > 0 && (
      currentContentHash == null ||
      countStaleMangaPagePrompts(pagePrompts, currentContentHash) > 0
    );
    const pageStagesStale = Boolean(outputState?.stale || preprocessingStale || pagePromptsStale);
    const pageStageProgress = (stage: MangaPageWorkspaceStage) =>
      pageStagesStale
        ? t("common.mediaAuthoring.needsUpdate")
        : stageProgress(
            pagePrompts.filter((page) => !mangaPageStageNeedsGeneration(page, stage)).length,
          );
    return [
      { id: "materials" as const, label: t("common.mediaAuthoring.materials"), icon: <ImagePlus size={13} /> },
      ...([
        ["preprocessing", t("comic.pipelineStages.preprocessing"), FileText],
      ] as const).map(([id, label, Icon]) => ({
        id,
        label,
        icon: <Icon size={13} />,
        status: preprocessingStale
          ? t("common.mediaAuthoring.needsUpdate")
          : preprocessingArtifact
            ? t("comic.prompt.available")
            : undefined,
      })),
      {
        id: "adaptation" as const,
        label: t("common.mediaAuthoring.adaptation"),
        icon: <FileText size={13} />,
        status: preprocessingStale
          ? t("common.mediaAuthoring.needsUpdate")
          : authoringStageLabel(authoring.workspace, "adaptation_plan", selectedChapter?.id, t),
      },
      {
        id: "page_planning" as const,
        label: t("comic.pipelineStages.pagePlanning"),
        icon: <PanelTop size={13} />,
        status: pageStagesStale || pageSlotsArtifact?.stale === true
          ? t("common.mediaAuthoring.needsUpdate")
          : pageSlotsArtifact
            ? pageStageProgress("page_planning")
            : t("comic.pipelineStages.notStarted"),
      },
      {
        id: "visual_design" as const,
        label: t("comic.pipelineStages.visualDesign"),
        icon: <Clapperboard size={13} />,
        status: pageStageProgress("visual_design"),
      },
      {
        id: "image_creation" as const,
        label: t("comic.pipelineStages.imageCreation"),
        icon: <Images size={13} />,
        status: pageStageProgress("image_creation"),
      },
    ];
  }, [authoring.workspace, currentContentHash, pagePrompts, pageSlotsArtifact, preprocessingArtifact, preprocessingStale, selectedChapter?.id, t]);
  const [workspaceStage, setWorkspaceStage] = usePersistedTab(
    staged ? `a4n:comic:${projectId}:stage` : null,
    staged ? "materials" : "image_creation",
    stageTabs.map((tab) => tab.id),
  );
  const activePageStage: MangaPageWorkspaceStage | undefined =
    staged && isMangaPageWorkspaceStage(workspaceStage)
      ? workspaceStage
      : undefined;
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [portraits, setPortraits] = useState<Portrait[]>([]);
  const [projectImages, setProjectImages] = useState<GeneratedImage[]>([]);
  const [projectImagesLoading, setProjectImagesLoading] = useState(false);
  const [projectImagesError, setProjectImagesError] = useState<string | null>(null);
  const [characterAssetsLoading, setCharacterAssetsLoading] = useState(false);
  const [characterAssetsError, setCharacterAssetsError] = useState<string | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [pipelineChapterId, setPipelineChapterId] = useState<number | null>(
    null,
  );
  const selectedChapterIdRef = useRef<number | null>(fixedChapter?.id ?? null);
  const pagePromptReadStateRef = useRef({ loading: pagePromptsLoading, error: pagePromptsError });
  pagePromptReadStateRef.current = { loading: pagePromptsLoading, error: pagePromptsError };

  const isSingleMode = hasFixedChapterContext;
  const showChapterNavigation = !isSingleMode && chapters.length > 1;
  const characterAssetProjectId = assetProjectId ?? projectId;
  const imageSourceProjectIds = useMemo(
    () => (imageProjectIds?.length ? imageProjectIds : [projectId]),
    [imageProjectIds, projectId],
  );
  const imageSourceKey = useMemo(
    () => imageSourceProjectIds.join(","),
    [imageSourceProjectIds],
  );
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const activeCharacterAssetProjectIdRef = useRef(characterAssetProjectId);
  const activeImageSourceKeyRef = useRef(imageSourceKey);
  const standaloneSourceLoadSeqRef = useRef(0);
  const chapterLoadSeqRef = useRef(0);
  const snapshotChapterLoadSeqRef = useRef(0);
  const statusLoadSeqRef = useRef(0);
  const pagePromptsLoadSeqRef = useRef(0);
  const chapterArtifactsLoadSeqRef = useRef(0);
  const characterAssetsLoadSeqRef = useRef(0);
  const projectImagesLoadSeqRef = useRef(0);
  const chapterReadFreshRef = useRef(hasFixedChapterContext);
  const chapterLoadErrorRef = useRef<string | null>(chapterLoadError);
  const statusFreshRef = useRef(new Map<number, boolean>());
  const statusLoadingSeqRef = useRef(new Map<number, number>());
  const statusProjectLoadingRef = useRef(false);
  const pipelineStatusErrorRef = useRef<MangaPipelineStatusLoadError | null>(pipelineStatusError);
  const actionSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const actionBusyKeyRef = useRef<string | null>(null);
  const previewBusyKeyRef = useRef<string | null>(null);
  const progressCancelBusyKeyRef = useRef<string | null>(null);

  activeProjectIdRef.current = projectId;
  activeCharacterAssetProjectIdRef.current = characterAssetProjectId;
  activeImageSourceKeyRef.current = imageSourceKey;
  chapterLoadErrorRef.current = chapterLoadError;
  pipelineStatusErrorRef.current = pipelineStatusError;

  const isProjectCurrent = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [],
  );

  const ensureCanEditManga = useCallback(() => {
    if (canEditManga) return true;
    addToast(t("comic.noEditPermission"), "error");
    return false;
  }, [addToast, canEditManga, t]);

  const ensureTargetedPagePromptData = (pageNumber: number | null, runStage: MangaPipelineArtifactStage | null) => {
    const pageStage = runStage != null && runStage !== "preprocessing" && runStage !== "page_slots";
    if (pageNumber == null && !pageStage) return true;
    const state = pagePromptReadStateRef.current;
    if (!state.loading && state.error == null) return true;
    addToast(state.error ?? t("common.operationInProgress"), "error");
    return false;
  };

  const ensureChapterReadCurrent = () => {
    if (
      chapterReadFreshRef.current &&
      chapterLoadErrorRef.current == null
    ) return true;
    addToast(
      chapterLoadErrorRef.current ?? t("common.operationInProgress"),
      "error",
    );
    return false;
  };

  const ensurePipelineStatusCurrent = (chapterId: number) => {
    const statusError = pipelineStatusErrorRef.current;
    const statusLoading = statusProjectLoadingRef.current || statusLoadingSeqRef.current.has(chapterId);
    const chapterError = statusError?.scope === "chapter" && statusError.chapterId === chapterId;
    const projectError = statusError?.scope === "project";
    if (
      statusFreshRef.current.get(chapterId) === true &&
      !statusLoading &&
      !chapterError &&
      !projectError
    ) return true;
    addToast(
      statusError?.message ?? t("common.operationInProgress"),
      "error",
    );
    return false;
  };

  const isCharacterAssetsCurrent = useCallback(
    (
      requestProjectId: number,
      requestAssetProjectId: number,
      requestSeq: number,
    ) =>
      isProjectCurrent(requestProjectId) &&
      activeCharacterAssetProjectIdRef.current === requestAssetProjectId &&
      characterAssetsLoadSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isProjectImagesCurrent = useCallback(
    (
      requestProjectId: number,
      requestImageSourceKey: string,
      requestSeq: number,
    ) =>
      isProjectCurrent(requestProjectId) &&
      activeImageSourceKeyRef.current === requestImageSourceKey &&
      projectImagesLoadSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isChapterCurrent = useCallback(
    (requestProjectId: number, requestChapterId: number) =>
      isProjectCurrent(requestProjectId) &&
      selectedChapterIdRef.current === requestChapterId,
    [isProjectCurrent],
  );

  const isActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && actionSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isPreviewCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) &&
      previewSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const beginAction = useCallback((key: string): boolean => {
    if (actionBusyKeyRef.current) return false;
    actionBusyKeyRef.current = key;
    return true;
  }, []);

  const endAction = useCallback((key: string) => {
    if (actionBusyKeyRef.current === key) actionBusyKeyRef.current = null;
  }, []);

  const beginPreview = useCallback((key: string): boolean => {
    if (previewBusyKeyRef.current) return false;
    previewBusyKeyRef.current = key;
    return true;
  }, []);

  const endPreview = useCallback((key: string) => {
    if (previewBusyKeyRef.current === key) previewBusyKeyRef.current = null;
  }, []);

  const resetInteractionLocks = useCallback(() => {
    actionBusyKeyRef.current = null;
    previewBusyKeyRef.current = null;
    progressCancelBusyKeyRef.current = null;
  }, []);

  const resetRequestPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyKeyRef.current = null;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const primaryPortraitCount = useMemo(
    () => portraits.filter((portrait) => portrait.is_primary).length,
    [portraits],
  );
  const missingPortraitCount = useMemo(() => {
    const profileIdsWithPortrait = new Set(
      portraits.map((portrait) => portrait.profile_id),
    );
    return profiles.filter((profile) => !profileIdsWithPortrait.has(profile.id))
      .length;
  }, [profiles, portraits]);

  useEffect(() => {
    selectedChapterIdRef.current = selectedChapter?.id ?? null;
  }, [selectedChapter?.id]);

  useEffect(() => {
    setSelectedPageNumber(null);
  }, [selectedChapter?.id]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      chapterLoadSeqRef.current += 1;
      statusLoadSeqRef.current += 1;
      pagePromptsLoadSeqRef.current += 1;
      chapterArtifactsLoadSeqRef.current += 1;
      characterAssetsLoadSeqRef.current += 1;
      projectImagesLoadSeqRef.current += 1;
      actionSeqRef.current += 1;
      previewSeqRef.current += 1;
      chapterReadFreshRef.current = isSingleMode;
      statusFreshRef.current.clear();
      statusLoadingSeqRef.current.clear();
      statusProjectLoadingRef.current = false;
      resetInteractionLocks();
    };
  }, [resetInteractionLocks]);

  const refreshPipelineTarget = async (
    chapterId: number,
    requestProjectId = projectId,
  ): Promise<boolean> => {
    if (!isProjectCurrent(requestProjectId)) return false;
    const results = await Promise.all([
      loadSingleStatus(chapterId, requestProjectId),
      isChapterCurrent(requestProjectId, chapterId)
        ? loadPagePrompts(chapterId, requestProjectId, { showLoading: false })
        : Promise.resolve(true),
      isChapterCurrent(requestProjectId, chapterId)
        ? loadChapterArtifacts(chapterId, requestProjectId)
        : Promise.resolve(true),
      loadProjectImages(requestProjectId),
      ...(staged && isChapterCurrent(requestProjectId, chapterId)
        ? [authoring.reload().then(() => true).catch(() => false)]
        : []),
    ]);
    return isProjectCurrent(requestProjectId) && results.every(Boolean);
  };

  const loadCharacterAssets = async (
    requestProjectId = projectId,
    requestAssetProjectId = characterAssetProjectId,
  ): Promise<boolean> => {
    const requestSeq = characterAssetsLoadSeqRef.current + 1;
    characterAssetsLoadSeqRef.current = requestSeq;
    if (
      isCharacterAssetsCurrent(
        requestProjectId,
        requestAssetProjectId,
        requestSeq,
      )
    ) {
      setCharacterAssetsLoading(true);
      setCharacterAssetsError(null);
    }
    try {
      const [profilesResult, portraitsResult] = await Promise.allSettled([
        listProfiles(requestAssetProjectId),
        listPortraitsByProject(requestAssetProjectId),
      ]);
      if (
        !isCharacterAssetsCurrent(
          requestProjectId,
          requestAssetProjectId,
          requestSeq,
        )
      )
        return false;
      const assetErrors: string[] = [];
      if (profilesResult.status === "fulfilled") {
        setProfiles(profilesResult.value);
      } else {
        const message = extractErrorMessage(profilesResult.reason, t("comic.loadCharacterAssetsFailed"));
        assetErrors.push(message);
      }
      if (portraitsResult.status === "fulfilled") {
        setPortraits(portraitsResult.value);
      } else {
        const message = extractErrorMessage(portraitsResult.reason, t("comic.loadCharacterAssetsFailed"));
        assetErrors.push(message);
      }
      setCharacterAssetsError(assetErrors.length > 0 ? assetErrors.join(" · ") : null);
      return profilesResult.status === "fulfilled" && portraitsResult.status === "fulfilled";
    } finally {
      if (
        isCharacterAssetsCurrent(
          requestProjectId,
          requestAssetProjectId,
          requestSeq,
        )
      ) {
        setCharacterAssetsLoading(false);
      }
    }
  };

  const loadProjectImages = async (
    requestProjectId = projectId,
    requestImageSourceProjectIds = imageSourceProjectIds,
    requestImageSourceKey = imageSourceKey,
  ): Promise<boolean> => {
    const requestSeq = projectImagesLoadSeqRef.current + 1;
    projectImagesLoadSeqRef.current = requestSeq;
    if (
      isProjectImagesCurrent(
        requestProjectId,
        requestImageSourceKey,
        requestSeq,
      )
    ) {
      setProjectImagesLoading(true);
      setProjectImagesError(null);
    }
    try {
      const images =
        requestImageSourceProjectIds.length === 1
          ? listImagesByProject(requestImageSourceProjectIds[0])
          : listImagesByProjects(requestImageSourceProjectIds);
      const loadedImages = await images;
      if (
        !isProjectImagesCurrent(
          requestProjectId,
          requestImageSourceKey,
          requestSeq,
        )
      )
        return false;
      const byId = new Map<number, GeneratedImage>();
      for (const image of loadedImages) byId.set(image.id, image);
      setProjectImages([...byId.values()]);
      setProjectImagesError(null);
      return true;
    } catch (err) {
      if (
        isProjectImagesCurrent(
          requestProjectId,
          requestImageSourceKey,
          requestSeq,
        )
      ) {
        const message = extractErrorMessage(err, t("comic.loadProjectImagesFailed"));
        setProjectImagesError(message);
      }
      return false;
    } finally {
      if (
        isProjectImagesCurrent(
          requestProjectId,
          requestImageSourceKey,
          requestSeq,
        )
      ) {
        setProjectImagesLoading(false);
      }
    }
  };

  const loadMangaAssets = (requestProjectId = projectId) =>
    Promise.all([
      loadCharacterAssets(requestProjectId, characterAssetProjectId),
      loadProjectImages(requestProjectId, imageSourceProjectIds, imageSourceKey),
    ]);

  const reloadMangaAssets = async () => {
    const [characterAssetsOk, projectImagesOk] = await loadMangaAssets();
    return characterAssetsOk && projectImagesOk;
  };

  const resetChapterDetail = () => {
    selectedChapterIdRef.current = null;
    pagePromptsLoadSeqRef.current += 1;
    chapterArtifactsLoadSeqRef.current += 1;
    statusLoadSeqRef.current += 1;
    resetRequestPreview();
    setSelectedChapter(null);
    setSelectedChapterLoadError(null);
    setPagePrompts([]);
    setChapterArtifacts([]);
    setChapterArtifactsLoadError(null);
    setPagePromptsError(null);
    setPipelineStatusError((current) => current?.scope === "chapter" ? null : current);
    setCurrentContentHash(null);
  };

  const openChapterDetail = async (row: ChapterRow) => {
    const requestProjectId = projectId;
    const shouldLoadAssets = !staged && selectedChapterIdRef.current == null;
    const requestSeq = snapshotChapterLoadSeqRef.current + 1;
    const needsFullChapter = snapshotChapters !== undefined || !standalone;
    snapshotChapterLoadSeqRef.current = requestSeq;
    selectedChapterIdRef.current = row.id;
    if (shouldLoadAssets) void loadMangaAssets(requestProjectId);
    resetRequestPreview();
    setSelectedChapter(row);
    setSnapshotChapterLoading(needsFullChapter);
    setSelectedChapterLoadError(null);
    setChapterLoadError(null);
    setPagePrompts([]);
    setChapterArtifacts([]);
    setChapterArtifactsLoadError(null);
    setPagePromptsError(null);
    setPipelineStatusError((current) => current?.scope === "chapter" ? null : current);
    setCurrentContentHash(needsFullChapter ? null : row.content_hash ?? null);

    let selected = row;
    if (needsFullChapter) {
      try {
        selected = snapshotChapters
          ? snapshotChapterRow(
              await getSourceSnapshotChapter(requestProjectId, row.id, row.snapshot_id),
            )
          : mangaChapterRowFromSummary(await getChapter(requestProjectId, row.id));
      } catch (error) {
        if (
          isProjectCurrent(requestProjectId) &&
          snapshotChapterLoadSeqRef.current === requestSeq
        ) {
          setSelectedChapter(row);
          setSelectedChapterLoadError(extractErrorMessage(error, t("comic.loadChaptersFailed")));
        }
        if (snapshotChapterLoadSeqRef.current === requestSeq) {
          setSnapshotChapterLoading(false);
        }
        return;
      }
    }
    if (
      !isProjectCurrent(requestProjectId) ||
      snapshotChapterLoadSeqRef.current !== requestSeq
    ) return;
    setSnapshotChapterLoading(false);
    setSelectedChapterLoadError(null);
    setSelectedChapter(selected);
    setCurrentContentHash(selected.content_hash ?? null);
    void loadPagePrompts(row.id, requestProjectId);
    void loadChapterArtifacts(row.id, requestProjectId);
    void loadSingleStatus(row.id, requestProjectId);
  };

  const prepareStandaloneSource = async (
    selection: MangaTextSourceSelection,
    requestProjectId = projectId,
    showLoading = false,
  ): Promise<boolean> => {
    const requestSeq = standaloneSourceLoadSeqRef.current + 1;
    standaloneSourceLoadSeqRef.current = requestSeq;
    setStandaloneSourceLoading(true);
    if (showLoading) setLoading(true);
    resetChapterDetail();
    setChapters([]);
    setChapterLoadError(null);
    setStatuses({});
    try {
      const source = await prepareStandaloneComicSource(
        requestProjectId,
        selection.assetId,
        selection.version,
        selection.startSection,
        selection.endSection,
      );
      if (!isProjectCurrent(requestProjectId) || standaloneSourceLoadSeqRef.current !== requestSeq) {
        return false;
      }
      const row: ChapterRow = {
        id: source.chapter_id,
        chapter_number: source.chapter_number,
        title: source.title,
        content: source.content,
        content_hash: source.content_sha256,
        summary: null,
        status: "completed",
        analysis_data: null,
      };
      setChapters([row]);
      await openChapterDetail(row);
      return true;
    } catch (error) {
      if (isProjectCurrent(requestProjectId) && standaloneSourceLoadSeqRef.current === requestSeq) {
        const message = extractErrorMessage(error, t("comic.prepareSourceFailed"));
        setChapterLoadError(message);
      }
      return false;
    } finally {
      if (standaloneSourceLoadSeqRef.current === requestSeq) {
        setStandaloneSourceLoading(false);
        if (showLoading) setLoading(false);
      }
    }
  };

  const selectMangaTextSource = (selection: MangaTextSourceSelection) => {
    if (!ensureCanEditManga()) return;
    if (
      selectedTextSource?.assetId === selection.assetId &&
      selectedTextSource.version === selection.version &&
      selectedTextSource.startSection === selection.startSection &&
      selectedTextSource.endSection === selection.endSection
    ) return;
    if (isMutationBusy()) {
      addToast(t("common.operationInProgress"), "info");
      return;
    }
    setSelectedTextSource(selection);
    if (standalone) void prepareStandaloneSource(selection);
  };

  const loadSingleStatus = async (
    chapterId: number,
    requestProjectId = projectId,
  ): Promise<boolean> => {
    const requestSeq = statusLoadSeqRef.current + 1;
    statusLoadSeqRef.current = requestSeq;
    statusFreshRef.current.set(chapterId, false);
    statusLoadingSeqRef.current.set(chapterId, requestSeq);
    if (isProjectCurrent(requestProjectId)) {
      setPipelineStatusError((current) =>
        current?.scope === "chapter" && current.chapterId === chapterId ? null : current,
      );
    }
    try {
      const status = await getMangaPipelineStatus(requestProjectId, chapterId);
      if (
        !isProjectCurrent(requestProjectId) ||
        statusLoadSeqRef.current !== requestSeq
      ) return false;
      setStatuses((current) => ({ ...current, [chapterId]: status }));
      statusFreshRef.current.set(chapterId, true);
      setPipelineStatusError((current) =>
        current?.scope === "chapter" && current.chapterId === chapterId ? null : current,
      );
      return true;
    } catch (error) {
      if (
        isProjectCurrent(requestProjectId) &&
        statusLoadSeqRef.current === requestSeq
      ) {
        statusFreshRef.current.set(chapterId, false);
        setPipelineStatusError({
          scope: "chapter",
          chapterId,
          message: extractErrorMessage(error, t("comic.loadPipelineStatusFailed")),
        });
      }
      return false;
    } finally {
      if (statusLoadingSeqRef.current.get(chapterId) === requestSeq) {
        statusLoadingSeqRef.current.delete(chapterId);
      }
    }
  };

  const loadChapters = async (
    requestProjectId = projectId,
    textSource = selectedTextSource,
  ) => {
    const requestSeq = chapterLoadSeqRef.current + 1;
    const needsFullChapter = snapshotChapters !== undefined || !standalone;
    chapterLoadSeqRef.current = requestSeq;
    const snapshotRequestSeq = snapshotChapterLoadSeqRef.current + 1;
    snapshotChapterLoadSeqRef.current = snapshotRequestSeq;
    setSnapshotChapterLoading(false);
    setSelectedChapterLoadError(null);
    chapterReadFreshRef.current = false;
    statusFreshRef.current.clear();
    statusProjectLoadingRef.current = false;
    if (isSingleMode) {
      chapterReadFreshRef.current = true;
      if (
        isProjectCurrent(requestProjectId) &&
        chapterLoadSeqRef.current === requestSeq
      ) {
        setLoading(false);
      }
      return;
    }
    if (standalone) {
      setChapters([]);
      setStatuses({});
      setSelectedChapter(null);
      selectedChapterIdRef.current = null;
      if (textSource) {
        const prepared = await prepareStandaloneSource(textSource, requestProjectId, true);
        if (prepared) chapterReadFreshRef.current = true;
      } else if (isProjectCurrent(requestProjectId) && chapterLoadSeqRef.current === requestSeq) {
        chapterReadFreshRef.current = true;
        setLoading(false);
      }
      return;
    }
    if (
      isProjectCurrent(requestProjectId) &&
      chapterLoadSeqRef.current === requestSeq
    ) {
      setLoading(true);
      setChapterLoadError(null);
      setPipelineStatusError(null);
      setSelectedChapterLoadError(null);
    }
    let loadingSelectedChapterDetail = false;
    let selectedChapterForDetail: ChapterRow | null = null;
    try {
      const rows: ChapterRow[] = snapshotChapters
        ? snapshotChapters.map(snapshotIndexChapterRow)
        : (await listChapterIndex(requestProjectId)).map(mangaChapterRowFromIndex);
      if (
        !isProjectCurrent(requestProjectId) ||
        chapterLoadSeqRef.current !== requestSeq
      )
        return;
      const previousChapterId = selectedChapterIdRef.current;
      const nextSelectedRow =
        rows.find((row) => row.id === previousChapterId) ??
        (snapshotChapters || rows.length === 1 ? rows[0] ?? null : null);
      selectedChapterForDetail = nextSelectedRow;
      if (!staged && nextSelectedRow && previousChapterId == null) {
        void loadMangaAssets(requestProjectId);
      }
      setChapters(rows);
      chapterReadFreshRef.current = true;
      setChapterLoadError(null);
      setStatuses({});
      selectedChapterIdRef.current = nextSelectedRow?.id ?? null;
      let nextSelectedChapter = nextSelectedRow;
      if (needsFullChapter && nextSelectedRow) {
        setSnapshotChapterLoading(true);
        loadingSelectedChapterDetail = true;
        setSelectedChapter(nextSelectedRow);
        nextSelectedChapter = snapshotChapters
          ? snapshotChapterRow(
              await getSourceSnapshotChapter(
                requestProjectId,
                nextSelectedRow.id,
                nextSelectedRow.snapshot_id,
              ),
            )
          : mangaChapterRowFromSummary(await getChapter(requestProjectId, nextSelectedRow.id));
        if (
          !isProjectCurrent(requestProjectId) ||
          chapterLoadSeqRef.current !== requestSeq ||
          snapshotChapterLoadSeqRef.current !== snapshotRequestSeq
        ) return;
      }
      if (
        !isProjectCurrent(requestProjectId) ||
        chapterLoadSeqRef.current !== requestSeq ||
        snapshotChapterLoadSeqRef.current !== snapshotRequestSeq
      ) return;
      setSnapshotChapterLoading(false);
      selectedChapterIdRef.current = nextSelectedChapter?.id ?? null;
      setSelectedChapter(nextSelectedChapter);
      if (nextSelectedChapter && nextSelectedChapter.id !== previousChapterId) {
        resetRequestPreview();
        setPagePrompts([]);
        setChapterArtifacts([]);
        setChapterArtifactsLoadError(null);
        setCurrentContentHash(nextSelectedChapter.content_hash ?? null);
        void loadPagePrompts(nextSelectedChapter.id, requestProjectId);
        void loadChapterArtifacts(nextSelectedChapter.id, requestProjectId);
      } else if (!nextSelectedChapter) {
        setPagePrompts([]);
        setChapterArtifacts([]);
        setChapterArtifactsLoadError(null);
        setCurrentContentHash(null);
      }
      setLoading(false);
      if (rows.length === 0) return;
      statusProjectLoadingRef.current = true;
      try {
        const statuses = await listMangaPipelineStatuses(
          requestProjectId,
          rows.map((row) => row.id),
        );
        if (
          !isProjectCurrent(requestProjectId) ||
          chapterLoadSeqRef.current !== requestSeq
        ) return;
        const statusMap: Record<number, MangaPipelineStatusSummary | null> = {};
        for (const status of statuses) statusMap[status.chapter_id] = status;
        setStatuses(statusMap);
        rows.forEach((row) => statusFreshRef.current.set(row.id, true));
        setPipelineStatusError(null);
      } catch (err) {
        if (
          isProjectCurrent(requestProjectId) &&
          chapterLoadSeqRef.current === requestSeq
        ) {
          rows.forEach((row) => statusFreshRef.current.set(row.id, false));
          const message = extractErrorMessage(err, t("comic.loadPipelineStatusFailed"));
          setPipelineStatusError({ scope: "project", message });
        }
      } finally {
        if (chapterLoadSeqRef.current === requestSeq) statusProjectLoadingRef.current = false;
      }
    } catch (err) {
      if (
        isProjectCurrent(requestProjectId) &&
        chapterLoadSeqRef.current === requestSeq &&
        snapshotChapterLoadSeqRef.current === snapshotRequestSeq
      ) {
        chapterReadFreshRef.current = false;
        const message = extractErrorMessage(err, t("comic.loadChaptersFailed"));
        if (loadingSelectedChapterDetail && selectedChapterForDetail) {
          selectedChapterIdRef.current = selectedChapterForDetail.id;
          setSelectedChapter(selectedChapterForDetail);
          setSelectedChapterLoadError(message);
        } else {
          if (snapshotChapters) {
            selectedChapterIdRef.current = null;
            setSelectedChapter(null);
          }
          setChapterLoadError(message);
        }
      }
    } finally {
      if (
        isProjectCurrent(requestProjectId) &&
        chapterLoadSeqRef.current === requestSeq
      ) {
        setLoading(false);
      }
      if (snapshotChapterLoadSeqRef.current === snapshotRequestSeq) {
        setSnapshotChapterLoading(false);
      }
    }
  };

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    activeCharacterAssetProjectIdRef.current = characterAssetProjectId;
    activeImageSourceKeyRef.current = imageSourceKey;
    chapterLoadSeqRef.current += 1;
    snapshotChapterLoadSeqRef.current += 1;
    statusLoadSeqRef.current += 1;
    pagePromptsLoadSeqRef.current += 1;
    characterAssetsLoadSeqRef.current += 1;
    projectImagesLoadSeqRef.current += 1;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    resetInteractionLocks();
    setChapters([]);
    setStatuses({});
    setSnapshotChapterLoading(false);
    setSelectedChapterLoadError(null);
    setPipelineStatusError(null);
    setPagePrompts([]);
    setPagePromptsError(null);
    setCurrentContentHash(null);
    setProfiles([]);
    setPortraits([]);
    setProjectImages([]);
    setProjectImagesError(null);
    setProjectImagesLoading(false);
    setCharacterAssetsError(null);
    setCharacterAssetsLoading(false);
    setPagePromptsLoading(false);
    setActing(null);
    resetRequestPreview();
    setPipelineChapterId(null);
    if (!isSingleMode) {
      selectedChapterIdRef.current = null;
      snapshotChapterLoadSeqRef.current += 1;
      setSnapshotChapterLoading(false);
      setSelectedChapterLoadError(null);
      setSelectedChapter(null);
      setLoading(true);
    }
    void loadChapters(projectId);
    if (staged || fixedChapter != null) void loadMangaAssets(projectId);
    // snapshotChapters is an external source list; when it changes, the visible chapter list must be rebuilt.
    // imageSourceKey 已表示图片来源的值身份；不能再依赖数组引用，否则父级重渲染会清空当前页。
  }, [
    characterAssetProjectId,
    imageSourceKey,
    isSingleMode,
    projectId,
    snapshotChapters,
    resetInteractionLocks,
    resetRequestPreview,
  ]);

  const loadPagePrompts = async (
    chapterId: number,
    requestProjectId = projectId,
    options: { showLoading?: boolean } = {},
  ): Promise<boolean> => {
    const showLoading = options.showLoading ?? true;
    const requestSeq = pagePromptsLoadSeqRef.current + 1;
    pagePromptsLoadSeqRef.current = requestSeq;
    if (showLoading && isChapterCurrent(requestProjectId, chapterId)) {
      setPagePromptsLoading(true);
    }
    if (isChapterCurrent(requestProjectId, chapterId)) setPagePromptsError(null);
    try {
      const data = await listMangaPagePrompts(requestProjectId, chapterId);
      if (
        !isChapterCurrent(requestProjectId, chapterId) ||
        pagePromptsLoadSeqRef.current !== requestSeq
      ) {
        return false;
      }
      setPagePrompts(data);
      setPagePromptsError(null);
      return true;
    } catch (error) {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        pagePromptsLoadSeqRef.current === requestSeq
      ) {
        setPagePromptsError(extractErrorMessage(error, t("comic.loadPagePromptsFailed")));
      }
      return false;
    } finally {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        pagePromptsLoadSeqRef.current === requestSeq
      ) {
        setPagePromptsLoading(false);
      }
    }
  };

  const loadChapterArtifacts = async (
    chapterId: number,
    requestProjectId = projectId,
  ): Promise<boolean> => {
    const requestSeq = chapterArtifactsLoadSeqRef.current + 1;
    chapterArtifactsLoadSeqRef.current = requestSeq;
    if (isChapterCurrent(requestProjectId, chapterId)) {
      setChapterArtifactsLoading(true);
      setChapterArtifactsLoadError(null);
    }
    try {
      const artifacts = await listMangaPipelineArtifacts(requestProjectId, chapterId);
      if (
        !isChapterCurrent(requestProjectId, chapterId) ||
        chapterArtifactsLoadSeqRef.current !== requestSeq
      ) return false;
      setChapterArtifacts(artifacts);
      setChapterArtifactsLoadError(null);
      return true;
    } catch (error) {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        chapterArtifactsLoadSeqRef.current === requestSeq
      ) {
        setChapterArtifactsLoadError(extractErrorMessage(error, t("comic.loadPagePromptsFailed")));
      }
      return false;
    } finally {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        chapterArtifactsLoadSeqRef.current === requestSeq
      ) setChapterArtifactsLoading(false);
    }
  };

  useLayoutEffect(() => {
    let cancelled = false;
    const chapterId = selectedChapter?.id;
    const content = selectedChapter?.content ?? "";
    if (selectedChapter?.content_hash) {
      setCurrentContentHash(selectedChapter.content_hash);
      return;
    }
    if (!chapterId || !content.trim()) {
      setCurrentContentHash(null);
      return;
    }
    void sha256Hex(content)
      .then((hash) => {
        if (!cancelled) setCurrentContentHash(hash);
      })
      .catch((err) => {
        if (!cancelled) {
          setCurrentContentHash(null);
          addToast(extractErrorMessage(err, t("comic.contentHashFailed")), "error");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [addToast, selectedChapter?.content, selectedChapter?.id]);

  const handleSelect = async (row: ChapterRow) => {
    if (selectedChapter?.id === row.id || selectedChapterIdRef.current === row.id) return;
    if (await confirmLeaveStage()) {
      setSelectedPageNumber(null);
      openChapterDetail(row);
    }
  };

  const handleDeleteChapterStage = async () => {
    if (!ensureCanEditManga()) return;
    const chapter = selectedChapter;
    const stage = mangaWorkspaceDeletionStage(workspaceStage);
    if (!chapter || !stage) return;
    if (!ensureChapterReadCurrent() || !ensurePipelineStatusCurrent(chapter.id)) return;
    if (isChapterGenerating(chapter)) {
      addToast(
        chapterGeneratingMessage(chapter, t("comic.operationFailed"), t),
        "error",
      );
      return;
    }

    const requestProjectId = projectId;
    if (!await confirmLeaveStage() || !isChapterCurrent(requestProjectId, chapter.id)) return;
    const actionKey = `delete-${chapter.id}-${stage}`;
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing(actionKey);
    try {
      const input = { project_id: requestProjectId, chapter_id: chapter.id, stage, page_number: null };
      const preview = await previewDeleteMangaPipelineArtifact(input);
      if (!isActionCurrent(requestProjectId, actionSeq) || !isChapterCurrent(requestProjectId, chapter.id)) return;
      const stageLabel = stageTabs.find((tab) => tab.id === workspaceStage)!.label;
      const downstream = mangaDeletionDownstreamSummary(preview, t);
      const confirmed = await confirmDialog({
        title: t("comic.deleteStageArtifactsTitle", { stage: stageLabel }),
        message: [
          t("comic.deleteStageArtifactsMessage", { chapter: chapterHeading(chapter, t), stage: stageLabel }),
          downstream ? t("comic.deleteStageArtifactsDownstream", { downstream }) : null,
        ].filter(Boolean).join("\n\n"),
        confirmText: t("comic.deleteStageArtifacts"),
        dialogType: "danger",
      });
      if (
        !confirmed ||
        !isChapterCurrent(requestProjectId, chapter.id) ||
        !ensureChapterReadCurrent() ||
        !ensurePipelineStatusCurrent(chapter.id)
      ) return;

      await deleteMangaPipelineArtifact({ ...input, expected_impact_fingerprint: preview.fingerprint });
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (isChapterCurrent(requestProjectId, chapter.id)) {
        discardStageDrafts();
      }
      addToast(t("comic.deletedStageArtifacts", { stage: stageLabel }), "success");
      const refreshed = await refreshPipelineTarget(chapter.id, requestProjectId);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (!refreshed) addToast(t("comic.deleteReadbackFailed"), "error");
    } catch (error) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(error, t("comic.operationFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        setActing(null);
      }
      endAction(actionKey);
    }
  };

  const handleRunPipeline = async (
    mode: "start" | "resume",
    stopAfter: MangaPipelineStopAfter | null = null,
    guard = true,
    pageNumber: number | null = null,
    runStage: MangaPipelineArtifactStage | null = null,
  ) => {
    if (!ensureCanEditManga()) return;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (!ensureChapterReadCurrent() || !ensurePipelineStatusCurrent(chapter.id)) return;
    if (!ensureTargetedPagePromptData(pageNumber, runStage)) return;
    if (guard && !await confirmLeaveStage()) return;
    if (
      !ensureChapterReadCurrent() ||
      !ensurePipelineStatusCurrent(chapter.id) ||
      !ensureTargetedPagePromptData(pageNumber, runStage)
    ) return;
    const requestProjectId = projectId;
    if (isChapterGenerating(chapter)) {
      addToast(
        chapterGeneratingMessage(chapter, t("comic.pipelineStartTitle"), t),
        "error",
      );
      return;
    }
    if (
      !imageSize &&
      runStage !== "preprocessing" &&
      (stopAfter == null || stopAfter === "image")
    ) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const isResume = mode === "resume";
    const actionKey = `${mode}-${chapter.chapter_number}`;
    const title = isResume
      ? t("comic.pipelineResumeTitle")
      : t("comic.pipelineStartTitle");
    const configBlockReason = mangaPipelineConfigBlockReason(t);
    if (configBlockReason) {
      addToast(configBlockReason, "error");
      return;
    }
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing(actionKey);
    setPipelineChapterId(chapter.id);

    let taskStarted = false;
    let terminalTaskCompleted = false;
    try {
      const handle = isResume
        ? await resumeMangaPipeline(requestProjectId, chapter.chapter_number, stopAfter, imageSize, pageNumber, runStage)
        : await startMangaPipeline(requestProjectId, chapter.chapter_number, stopAfter, imageSize, pageNumber, runStage);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      taskStarted = true;
      void loadSingleStatus(chapter.id, requestProjectId);
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title,
          startMessage: t("comic.pipelineSubmitted"),
          successMessage: t("comic.pipelineCompleted"),
          errorMessage: t("comic.pipelineFailed"),
          cancelLabel: t("comic.cancelPipeline"),
          runningHint: t("comic.pipelineRunningHint"),
        },
        async (task) => {
          if (task.taskStatus === "cancelled") {
            throw new Error("Generation task cancelled");
          }
          if (task.taskStatus !== "completed") {
            throw new Error(task.error?.message ?? t("comic.pipelineFailed"));
          }
          if (!isActionCurrent(requestProjectId, actionSeq)) return;
          terminalTaskCompleted = true;
          void refreshPipelineTarget(chapter.id, requestProjectId);
          if (isActionCurrent(requestProjectId, actionSeq)) {
            setPipelineChapterId(null);
          }
        },
      );
    } catch (error) {
        if (!isActionCurrent(requestProjectId, actionSeq)) return;
        const message = extractErrorMessage(
          error,
          isResume ? t("comic.pipelineResumeFailed") : t("comic.pipelineStartFailed"),
        );
        const cancelled = isCancelledOperation(message);
        if (taskStarted && !terminalTaskCompleted) {
          void refreshPipelineTarget(chapter.id, requestProjectId);
        }
        if (!isActionCurrent(requestProjectId, actionSeq)) return;
        if (!cancelled) {
          addToast(
            explainMangaPipelineAiConfigError(message, t),
            "error",
          );
        }
        setPipelineChapterId(null);
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setActing(null);
      endAction(actionKey);
    }
  };

  const handlePreviewPipeline = async (
    mode: "start" | "resume",
    stopAfter: MangaPipelineStopAfter | null = null,
    pageNumber: number | null = null,
    runStage: MangaPipelineArtifactStage | null = null,
  ) => {
    if (!ensureCanEditManga()) return;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (!ensureChapterReadCurrent() || !ensurePipelineStatusCurrent(chapter.id)) return;
    if (!ensureTargetedPagePromptData(pageNumber, runStage)) return;
    const requestProjectId = projectId;
    if (isChapterGenerating(chapter)) {
      addToast(
        chapterGeneratingMessage(chapter, t("comic.pipelineRequestPreview"), t),
        "error",
      );
      return;
    }
    if (!chapter.content?.trim()) {
      addToast(t("comic.emptyChapterPreviewPipeline"), "error");
      return;
    }
    const previewKey = `${mode}:${requestProjectId}:${chapter.id}`;
    if (!beginPreview(previewKey)) return;
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview =
        mode === "resume"
          ? await previewResumeMangaPipelineRequest(
              requestProjectId,
              chapter.chapter_number,
              stopAfter,
              pageNumber,
              runStage,
            )
          : await previewStartMangaPipelineRequest(
              requestProjectId,
              chapter.chapter_number,
              stopAfter,
              pageNumber,
              runStage,
            );
      if (
        !isPreviewCurrent(requestProjectId, requestSeq) ||
        !isChapterCurrent(requestProjectId, chapter.id)
      ) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, t("comic.previewPipelineFailed")), "error");
        setRequestPreviewOpen(false);
        endPreview(previewKey);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, requestSeq))
        setRequestPreviewLoading(false);
    }
  };

  const closeRequestPreview = resetRequestPreview;

  const requestPipelineCancellation = async (
    chapterId: number,
    handle: GenerationTaskHandle,
  ) => {
    const requestProjectId = projectId;
    const cancelKey = `${requestProjectId}:${chapterId}`;
    if (progressCancelBusyKeyRef.current) return;
    progressCancelBusyKeyRef.current = cancelKey;
    try {
      await generationClient.cancel(handle);
      if (!isProjectCurrent(requestProjectId)) return;
      addToast(t("comic.pipelineCancelRequested"), "info");
      void refreshPipelineTarget(chapterId, requestProjectId);
    } catch (error) {
      if (isProjectCurrent(requestProjectId)) {
        addToast(extractErrorMessage(error, t("comic.pipelineCancelFailed")), "error");
      }
    } finally {
      if (progressCancelBusyKeyRef.current === cancelKey) {
        progressCancelBusyKeyRef.current = null;
      }
    }
  };

  const handleGenerateMissingPortraits = async () => {
    if (!ensureCanEditManga()) return;
    if (characterAssetsLoading || characterAssetsError != null) {
      addToast(characterAssetsError ?? t("common.operationInProgress"), "error");
      return;
    }
    const requestProjectId = projectId;
    const requestAssetProjectId = characterAssetProjectId;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const actionKey = `generate-missing-portraits:${requestAssetProjectId}`;
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing("generate-missing-portraits");
    try {
      const handle = await startPortraitGeneration({
        project_id: requestAssetProjectId,
        operation: {
          kind: "generate_missing",
          width: requestSize.width,
          height: requestSize.height,
        },
      });
      const createdCount = await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("comic.portraitGenerationTitle"),
          startMessage: t("comic.portraitGenerationStart"),
          successMessage: t("comic.portraitGenerationSuccess"),
          errorMessage: t("comic.portraitGenerationFailed"),
          cancelLabel: t("comic.stopGeneration"),
        },
        async (task) => {
          const generated = await verifyPortraitGenerationResult(task, requestAssetProjectId);
          if (!isActionCurrent(requestProjectId, actionSeq))
            throw new Error("任务已停止");
          void loadCharacterAssets(requestProjectId, requestAssetProjectId);
          return generated.length;
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(t("comic.missingPortraitsGenerated", { count: createdCount }), "success");
    } catch (err) {
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const message = extractErrorMessage(err, t("comic.portraitGenerationFailed"));
      if (isCancelledOperation(message)) {
        addToast(t("comic.portraitGenerationStopped"), "info");
      } else {
        addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setActing(null);
      endAction(actionKey);
    }
  };

  const selectedStatus = selectedChapter ? statuses[selectedChapter.id] : null;
  const assetsError = [characterAssetsError, projectImagesError].filter(Boolean).join(" · ") || null;
  useEffect(() => {
    if (staged && selectedChapter && selectedStatus?.pipeline_status === "completed") {
      // A reloaded view may have read artifacts before the background task finished.
      void loadChapterArtifacts(selectedChapter.id, projectId);
      void authoring.reload().catch(() => { /* Stage read errors are shown by the authoring panel. */ });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [staged, projectId, selectedChapter?.id, selectedStatus?.pipeline_status, authoring.reload]);

  // fixedChapter 变化时联动
  useEffect(() => {
    if (fixedChapter) {
      if (!isProjectCurrent(projectId)) return;
      openChapterDetail(fixedChapter);
    } else if (isSingleMode) {
      resetChapterDetail();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    fixedChapter?.content,
    fixedChapter?.chapter_number,
    fixedChapter?.id,
    fixedChapter?.status,
    isSingleMode,
  ]);

  // 进度对话框订阅 Generation Task；其余运行状态由章节详情轮询。
  useEffect(() => {
    const chapterId = selectedChapter?.id;
    const shouldPoll =
      selectedStatus?.pipeline_status === "running" ||
      selectedStatus?.can_cancel === true;
    if (!chapterId || !shouldPoll) return;
    const timer = window.setInterval(() => {
      if (!isChapterCurrent(projectId, chapterId)) return;
      void loadSingleStatus(chapterId, projectId);
      void loadPagePrompts(chapterId, projectId, { showLoading: false });
    }, 2500);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    pipelineChapterId,
    selectedChapter?.id,
    selectedStatus?.pipeline_status,
    selectedStatus?.can_cancel,
  ]);

  if (loading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("comic.loadingChapters")}
      </div>
    );
  }

  if (!standalone && !isSingleMode && chapters.length === 0) {
    return (
      <div className="flex min-h-0 flex-1 flex-col gap-3">
        {chapterLoadError ? (
          <div role="alert" className="flex flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
            <span className="min-w-0 flex-1">{chapterLoadError}</span>
            <BookButton variant="secondary" size="sm" onClick={() => void loadChapters(projectId)} disabled={loading}>
              <RefreshCw size={12} /> {t("common.retry")}
            </BookButton>
          </div>
        ) : null}
        <div className="content-main">
          <MangaChapterList
            chapters={chapters}
            statuses={statuses}
            workspaceStage={staged ? workspaceStage : undefined}
            selectedChapter={selectedChapter}
            onSelect={handleSelect}
          />
        </div>
      </div>
    );
  }

  const deletionStage = mangaWorkspaceDeletionStage(workspaceStage);
  const deletingChapterStage = selectedChapter != null && acting === `delete-${selectedChapter.id}-${deletionStage}`;
  const deleteStageArtifactsButton = canEditManga && selectedChapter && deletionStage ? (
    <BookButton
      size="sm"
      variant="danger"
      onClick={() => { if (!deletingChapterStage) void handleDeleteChapterStage(); }}
      aria-disabled={deletingChapterStage || undefined}
      aria-busy={deletingChapterStage || undefined}
      className={deletingChapterStage ? "pointer-events-none opacity-50" : undefined}
      disabled={!deletingChapterStage && (acting != null || isMutationBusy() || chapterArtifactsLoading || chapterArtifactsLoadError != null || !chapterArtifacts.some((artifact) => artifact.stage === deletionStage) || selectedStatus?.pipeline_status === "running" || isChapterGenerating(selectedChapter) || snapshotChapterLoading)}
    >
      {deletingChapterStage ? <LoadingSpinner size="sm" /> : <Trash2 size={13} />}
      {t("comic.deleteStageArtifacts")}
    </BookButton>
  ) : null;

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col gap-3">
      {staged && <MediaStageHeader id={stageId} label={t("common.mediaAuthoring.stages")} tabs={stageTabs} activeTab={workspaceStage}
        onTabChange={async (stage) => {
          if (stage === workspaceStage) return true;
          if (workspaceStage === "materials" && !await reloadMangaAssets()) return false;
          if (!await confirmLeaveStage()) return false;
          setWorkspaceStage(stage);
          return true;
        }}>
          {deleteStageArtifactsButton}
        </MediaStageHeader>}
      {!staged && deleteStageArtifactsButton ? (
        <div className="flex shrink-0 flex-wrap justify-end gap-2">{deleteStageArtifactsButton}</div>
      ) : null}
      {outputState?.stale && (workspaceStage === "visual_design" || workspaceStage === "image_creation") && (
        <p role="status" className="shrink-0 rounded-xl border border-warning/40 bg-warning-bg px-3 py-2 text-small text-warning">
          {t("common.mediaAuthoring.outputsStale")}
        </p>
      )}
      {chapterLoadError ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{chapterLoadError}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void loadChapters(projectId)} disabled={loading || standaloneSourceLoading}>
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      {pipelineStatusError ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{pipelineStatusError.message}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void (pipelineStatusError.scope === "chapter" ? loadSingleStatus(pipelineStatusError.chapterId, projectId) : loadChapters(projectId))} disabled={loading}>
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      {chapterArtifactsLoadError && selectedChapter ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{chapterArtifactsLoadError}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void loadChapterArtifacts(selectedChapter.id, projectId)} disabled={chapterArtifactsLoading}>
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      {assetsError && (!staged || workspaceStage === "materials" || workspaceStage === "visual_design" || workspaceStage === "image_creation") ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{assetsError}</span>
          <BookButton
            variant="secondary"
            size="sm"
            disabled={characterAssetsLoading || projectImagesLoading}
            onClick={() => {
              if (characterAssetsError) void loadCharacterAssets(projectId, characterAssetProjectId);
              if (projectImagesError) void loadProjectImages(projectId, imageSourceProjectIds, imageSourceKey);
            }}
          >
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      {authoring.error && workspaceStage !== "adaptation" ? (
        <p role="alert" className="shrink-0 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          {authoring.error}
        </p>
      ) : null}
      <div
      className={
        isSingleMode
          ? "flex min-h-0 flex-1 flex-col"
          : showChapterNavigation
            ? "grid min-h-0 flex-1 grid-cols-1 grid-rows-[auto_minmax(0,1fr)] gap-3 md:grid-cols-[220px_minmax(0,1fr)] md:grid-rows-1 md:gap-4"
            : "flex min-h-0 flex-1 flex-col"
      }
    >
      {/* 章节列表（仅完整模式） */}
      {showChapterNavigation && (
        <MangaChapterList
          chapters={chapters}
          statuses={statuses}
          workspaceStage={staged ? workspaceStage : undefined}
          selectedChapter={selectedChapter}
          onSelect={handleSelect}
        />
      )}

      <div id={staged ? `${stageId}-panel` : undefined} role={staged ? "tabpanel" : undefined} aria-labelledby={staged ? `${stageId}-tab-${workspaceStage}` : undefined} className="flex min-h-0 min-w-0 flex-1 flex-col">
      {workspaceStage === "adaptation" ? <MediaAuthoringPanel
        key={`${projectId}:${selectedChapter?.id}:${workspaceStage}:${discardVersion}`} projectId={projectId} kind="adaptation_plan"
        workspace={authoring.workspace} loading={authoring.loading} error={authoring.error} chapterId={selectedChapter?.id ?? null}
        canEdit={canEditManga} stageRunning={selectedStatus?.pipeline_status === "running"}
        preconditionError={selectedChapter && (!preprocessingArtifact || preprocessingStale)
          ? t("comic.preprocessingRequired")
          : null}
        onReload={async () => {
          if (selectedChapter) {
            if (!await refreshPipelineTarget(selectedChapter.id)) throw new Error(t("common.mediaAuthoring.readbackFailed"));
          } else await authoring.reload();
        }} onDirtyChange={setAuthoringDirty}
        onBusyChange={setAuthoringBusy}
      /> : workspaceStage === "materials" ? <BaseMaterialsWorkspace projectId={characterAssetProjectId} textProjectId={projectId} canEdit={canEditManga} showNext={!standalone} standaloneComic={standalone}
        onChanged={loadProjectImages} onBeforeKindChange={confirmLeaveStage} onTextDirtyChange={setTextMaterialsDirty} selectedTextSource={standalone ? selectedTextSource : null} onSelectTextSource={standalone ? (selection) => void selectMangaTextSource(selection) : undefined} onNext={async () => { if (!await reloadMangaAssets()) return; if (!await confirmLeaveStage()) return; setWorkspaceStage("adaptation"); }} /> : !isMangaPageWorkspaceStage(workspaceStage) ? (
        selectedChapter ? (
          <MangaChapterArtifactPanel
            key={`${projectId}:${selectedChapter.id}:preprocessing:${discardVersion}`}
            projectId={projectId}
            chapterId={selectedChapter.id}
            stage="preprocessing"
            artifact={preprocessingArtifact ?? null}
            canEdit={canEditManga && !chapterArtifactsLoading && chapterArtifactsLoadError == null && !isChapterGenerating(selectedChapter) && selectedStatus?.pipeline_status !== "running"}
            onBusyChange={setAuthoringBusy}
            resumable={selectedStatus?.can_resume === true}
            onGenerate={() => void handleRunPipeline(selectedStatus?.can_resume ? "resume" : "start", null, true, null, "preprocessing")}
            onPreview={() => void handlePreviewPipeline(selectedStatus?.can_resume ? "resume" : "start", null, null, "preprocessing")}
            onChanged={async (change) => {
              setChapterArtifacts((current) => applyMangaChapterArtifactChange(current, change));
              if (!await refreshPipelineTarget(selectedChapter.id)) throw new Error(t("common.mediaAuthoring.readbackFailed"));
            }}
          />
        ) : <EmptyHint text={t("comic.chapter.noSelection")} />
      ) : (
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <MangaChapterDetailPanel
        key={`${projectId}:${selectedChapter?.id}:${workspaceStage}:${discardVersion}`}
        stageMode={activePageStage}
        onDirtyChange={setPromptDirty}
        projectId={projectId}
        isSingleMode={isSingleMode}
        selectedChapter={selectedChapter}
        selectedChapterLoading={snapshotChapterLoading}
        selectedChapterLoadError={selectedChapterLoadError}
        onRetryChapter={() => {
          if (selectedChapter) void openChapterDetail(selectedChapter);
        }}
        selectedStatus={selectedStatus}
        pagePrompts={pagePrompts}
        pageSlotsNeedsGeneration={pageSlotsNeedsGeneration}
        pageSlotsArtifactStale={pageSlotsArtifact?.stale === true}
        preprocessingArtifactStale={preprocessingStale}
        paginationActions={workspaceStage === "page_planning" && selectedChapter && pageSlotsArtifact ? (
          <MangaChapterArtifactPanel
            projectId={projectId}
            chapterId={selectedChapter.id}
            stage="page_slots"
            artifact={pageSlotsArtifact}
            canEdit={canEditManga && !chapterArtifactsLoading && chapterArtifactsLoadError == null && selectedStatus?.pipeline_status !== "running"}
            onBusyChange={setAuthoringBusy}
            compact
            onChanged={async (change) => {
              setChapterArtifacts((current) => applyMangaChapterArtifactChange(current, change));
              if (!await refreshPipelineTarget(selectedChapter.id)) throw new Error(t("common.mediaAuthoring.readbackFailed"));
            }}
          />
        ) : null}
        selectedPageNumber={selectedPageNumber}
        onSelectedPageChange={setSelectedPageNumber}
        pagePromptsLoading={pagePromptsLoading}
        pagePromptsError={pagePromptsError}
        currentContentHash={currentContentHash}
        characterAssetsLoading={characterAssetsLoading}
        characterAssetsError={characterAssetsError}
        projectImagesLoading={projectImagesLoading}
        profiles={profiles}
        portraits={portraits}
        projectImages={projectImages}
        onProjectImageImported={(image) =>
          setProjectImages((current) => [
            image,
            ...current.filter((item) => item.id !== image.id),
          ])
        }
        imageSize={imageSize}
        imageSizes={imageSizes}
        imageSizeLoading={imageSizeLoading}
        imageSizeError={imageSizeError}
        onImageSizeChange={setImageSize}
        primaryPortraitCount={primaryPortraitCount}
        characterProfileCount={profiles.length}
        portraitCount={portraits.length}
        missingPortraitCount={missingPortraitCount}
        acting={acting}
        canEditManga={canEditManga}
        onPreviewPipeline={(mode, stopAfter, pageNumber, runStage) => void handlePreviewPipeline(mode, stopAfter, pageNumber, runStage)}
        onRunPipeline={(mode, stopAfter, pageNumber, runStage) => void handleRunPipeline(mode, stopAfter, true, pageNumber, runStage)}
        {...(!standalone ? { onGenerateMissingPortraits: () => void handleGenerateMissingPortraits() } : {})}
        onCancelPipeline={() => {
          if (!ensureCanEditManga()) return;
          const chapter = selectedChapter;
          const handle = selectedStatus?.task_handle;
          if (!chapter || !handle) {
            addToast(t("comic.pipelineCancelFailed"), "error");
            return;
          }
          if (!ensureChapterReadCurrent() || !ensurePipelineStatusCurrent(chapter.id)) return;
          void requestPipelineCancellation(chapter.id, handle);
        }}
        onPagePromptsChanged={() => selectedChapter ? refreshPipelineTarget(selectedChapter.id) : Promise.resolve(false)}
        onPagePromptChanged={(prompt) => {
          setPagePrompts((current) => {
            const index = current.findIndex((item) => item.id === prompt.id);
            if (index < 0) return [...current, prompt];
            const next = [...current];
            next[index] = prompt;
            return next;
          });
        }}
        onPageArtifactChanged={(artifact) => {
          setPagePrompts((current) => current.map((prompt) => applyMangaPageArtifactToPrompt(prompt, artifact)));
        }}
        onRetryPagePrompts={() => {
          if (!selectedChapter) return;
          void loadPagePrompts(selectedChapter.id, projectId);
        }}
        />
        </div>
      )}
      </div>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("comic.pipelineRequestPreview")}
        onClose={closeRequestPreview}
      />
      </div>
    </div>
  );
};
