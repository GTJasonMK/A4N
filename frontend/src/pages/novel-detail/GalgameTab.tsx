import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Download,
  FileSearch,
  FileText,
  PenLine,
  Plus,
  RefreshCw,
  X,
} from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { promptDialog } from "../../components/feedback/PromptDialog";
import { useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { PipelineStatusPanel } from "./galgame/PipelineStatusPanel";
import {
  bindGalgameMedia,
  cancelGalgamePipeline,
  deleteGalgameLine,
  exportGalgameWebgal,
  getGalgamePipelineStatus,
  getGalgameScript,
  getGalgameAssetPlan,
  importGalgameAudio,
  insertGalgameLine,
  planGalgameAssets,
  previewGalgamePipelineRequest,
  reorderGalgameLines,
  runGalgameAssetGeneration,
  startGalgamePipeline,
  unbindGalgameMedia,
  updateGalgameChoices,
  updateGalgameLine,
  updateGalgameScene,
  type GalgameChoice,
  type GalgameLine,
  type GalgameAssetPlan,
  type GalgameMediaBindingSummary,
  type GalgamePipelineStatusSummary,
  type GalgameScene,
  type GalgameScriptDetail,
  type GalgameSceneDetail,
  type GalgameSpeakerProfile,
  type GalgameTextAnchorSummary,
  type GalgamePipelineStartStage,
  type UpdateGalgameChoiceOptionInput,
  type UpdateGalgameLineInput,
  type UpdateGalgameSceneInput,
} from "../../api/galgame";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { extractErrorMessage } from "../../api";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import type { SourceSnapshotChapterSummary } from "../../api/project";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { downloadBase64ExportFile } from "../../utils/downloadFile";
import { isCancelledOperation } from "../../utils/cancellation";
import { useI18n } from "../../i18n";
import {
  PIPELINE_START_STAGES,
  audioBusyTarget,
  bindMediaBusyTarget,
  emptyGalgamePresentation,
  insertBusyTarget,
  lineBusyTarget,
  pipelineStartStageLabel,
  sceneEntryKindLabel,
  sceneBusyTarget,
  presentationForLinePatch,
} from "./galgame/galgameTabShared";
import { AssetPlanPanel } from "./galgame/AssetPlanPanels";
import { LineEditor, SceneMetaEditor } from "./galgame/GalgameLineEditors";
import { buildChoiceDrafts } from "./galgame/GalgameChoiceEditor";
import { buildCgDraft } from "./galgame/presentation";
import { useGalgameDrafts } from "./galgame/GalgameDrafts";
import { confirmDiscardMediaChanges } from "../../components/feedback/UnsavedChangesGuard";
import { MediaStageHeader } from "../../components/shell/MediaStageHeader";
import { MediaAuthoringPanel } from "../derived/MediaAuthoringPanel";
import { useMediaAuthoring, authoringStageLabel } from "../derived/useMediaAuthoring";
import type { GalgamePipelineStopAfter } from "../../api/generated/host";
import { usePersistedTab } from "../../hooks/usePersistedTab";
import { useDirty } from "../../components/navigation/dirtyRegistry";
import { BaseMaterialsWorkspace } from "../derived/BaseMaterialsWorkspace";


interface GalgameTabProps {
  onDirtyChange?: (dirty: boolean) => void;
  projectId: number;
  materialProjectId: number;
  canEditGalgame: boolean;
  snapshotChapters: SourceSnapshotChapterSummary[];
}

export const GalgameTab: React.FC<GalgameTabProps> = ({
  projectId,
  materialProjectId,
  canEditGalgame,
  snapshotChapters,
  onDirtyChange,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [authoringDirty, setAuthoringDirty] = useState(false);
  const [discardVersion, setDiscardVersion] = useState(0);
  const authoring = useMediaAuthoring(projectId);
  const stageId = `galgame-stages-${projectId}`;
  const outputState = authoring.workspace?.artifacts.find((artifact) => artifact.kind === "galgame_script");
  const stageTabs = useMemo(() => [
    { id: "materials" as const, label: t("common.mediaAuthoring.materials") },
    { id: "adaptation" as const, label: t("common.mediaAuthoring.adaptation"), status: authoringStageLabel(authoring.workspace, "adaptation_plan", undefined, t) },
    { id: "branches" as const, label: t("common.mediaAuthoring.branches"), status: authoringStageLabel(authoring.workspace, "galgame_branch_plan", undefined, t) },
    { id: "script" as const, label: t("common.mediaAuthoring.script"), status: outputState?.count ? t(outputState.stale ? "common.mediaAuthoring.needsUpdate" : "common.mediaAuthoring.ready") : undefined },
    { id: "asset_plan" as const, label: t("common.mediaAuthoring.assetPlan") },
    { id: "asset_generation" as const, label: t("common.mediaAuthoring.assetGeneration") },
    { id: "export" as const, label: t("common.mediaAuthoring.export") },
  ], [authoring.workspace, outputState, t]);
  const [workspaceStage, setWorkspaceStage] = usePersistedTab(
    `a4n:galgame:${projectId}:stage`,
    "materials",
    stageTabs.map((tab) => tab.id),
  );
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    size: imageSize,
    sizes: imageSizes,
    setSize: setImageSize,
  } = useImageGenerationSize();
  const { runProgressTask } = useProgressTask();
  const [detail, setDetail] = useState<GalgameScriptDetail | null>(null);
  const [pipelineStatus, setPipelineStatus] =
    useState<GalgamePipelineStatusSummary | null>(null);
  const [selectedSceneId, setSelectedSceneId] = useState<number | null>(null);
  const [mobileWorkspaceView, setMobileWorkspaceView] = useState<
    "scenes" | "editor"
  >("scenes");
  const [pipelineControlsOpen, setPipelineControlsOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [pipelineCancelling, setPipelineCancelling] = useState(false);
  const [planningAssets, setPlanningAssets] = useState(false);
  const [generatingAssets, setGeneratingAssets] = useState(false);
  const [generatingAssetSpecId, setGeneratingAssetSpecId] = useState<
    number | null
  >(null);
  const [exportingWebgal, setExportingWebgal] = useState(false);
  const [assetPlan, setAssetPlan] = useState<GalgameAssetPlan | null>(null);
  const [assetPlanError, setAssetPlanError] = useState<string | null>(null);
  const [busyTarget, setBusyTarget] = useState<string | null>(null);
  const [pipelineStartStage, setPipelineStartStage] = useState<
    GalgamePipelineStartStage | ""
  >("");
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const requestSeqRef = useRef(0);
  const previewSeqRef = useRef(0);

  const load = React.useCallback(async (): Promise<boolean> => {
    const seq = requestSeqRef.current + 1;
    requestSeqRef.current = seq;
    setLoading(true);
    try {
      const [statusResult, scriptResult, assetPlanResult] = await Promise.allSettled([
        getGalgamePipelineStatus(projectId),
        getGalgameScript(projectId),
        getGalgameAssetPlan(projectId),
      ]);
      if (requestSeqRef.current !== seq) return false;
      if (statusResult.status === "fulfilled") {
        setPipelineStatus(statusResult.value);
      } else {
        setPipelineStatus(null);
        addToast(
          extractErrorMessage(statusResult.reason, t("galgame.pipelineFailed")),
          "error",
        );
      }
      if (scriptResult.status === "fulfilled") {
        setLoadError(null);
        setDetail(scriptResult.value);
        setSelectedSceneId(
          (current) =>
            current ?? scriptResult.value?.scenes[0]?.scene.id ?? null,
        );
      } else {
        setLoadError(extractErrorMessage(scriptResult.reason, t("galgame.loadFailed")));
      }
      if (assetPlanResult.status === "fulfilled") {
        setAssetPlan(assetPlanResult.value);
        setAssetPlanError(null);
      } else {
        setAssetPlanError(extractErrorMessage(assetPlanResult.reason, t("galgame.planLoadFailed")));
      }
      return [statusResult, scriptResult, assetPlanResult].every((result) => result.status === "fulfilled");
    } finally {
      if (requestSeqRef.current === seq) setLoading(false);
    }
  }, [addToast, projectId]);

  useEffect(() => {
    setDetail(null);
    setLoadError(null);
    setPipelineStatus(null);
    setAssetPlan(null);
    setAssetPlanError(null);
    setSelectedSceneId(null);
    setMobileWorkspaceView("scenes");
    void load();
    return () => {
      requestSeqRef.current += 1;
    };
  }, [load]);

  const commitDetail = (update: (current: GalgameScriptDetail) => GalgameScriptDetail) => {
    requestSeqRef.current += 1;
    setLoading(false);
    setLoadError(null);
    setDetail((current) => current == null ? current : update(current));
  };

  const commitMediaBinding = (saved: GalgameMediaBindingSummary) => {
    commitDetail((current) => ({
      ...current,
      media_bindings: [
        ...current.media_bindings.filter(({ binding }) =>
          binding.target_kind !== saved.binding.target_kind ||
          binding.target_id !== saved.binding.target_id ||
          binding.role !== saved.binding.role,
        ),
        saved,
      ],
    }));
  };

  const sceneById = useMemo(() => {
    const map = new Map<number, GalgameSceneDetail>();
    for (const scene of detail?.scenes ?? []) map.set(scene.scene.id, scene);
    return map;
  }, [detail?.scenes]);

  const speakerProfileById = useMemo(() => {
    const map = new Map<number, GalgameSpeakerProfile>();
    for (const profile of detail?.speaker_profiles ?? []) {
      if (!map.has(profile.id)) map.set(profile.id, profile);
    }
    return map;
  }, [detail?.speaker_profiles]);

  const selectedScene = useMemo(
    () =>
      (selectedSceneId != null
        ? (sceneById.get(selectedSceneId) ?? null)
        : null) ??
      detail?.scenes[0] ??
      null,
    [detail?.scenes, sceneById, selectedSceneId],
  );

  const chapterBySnapshotId = useMemo(() => {
    const map = new Map<number, SourceSnapshotChapterSummary>();
    for (const chapter of snapshotChapters) map.set(chapter.id, chapter);
    return map;
  }, [snapshotChapters]);
  const selectedSourceChapter = selectedScene
    ? (chapterBySnapshotId.get(
        selectedScene.scene.source_snapshot_chapter_id,
      ) ?? null)
    : null;

  const textAnchorById = useMemo(() => {
    const map = new Map<number, GalgameTextAnchorSummary>();
    for (const anchor of detail?.text_anchors ?? []) map.set(anchor.id, anchor);
    return map;
  }, [detail?.text_anchors]);

  const mediaBindingsByLine = useMemo(() => {
    const map = new Map<number, GalgameMediaBindingSummary[]>();
    for (const binding of detail?.media_bindings ?? []) {
      if (binding.binding.target_kind !== "galgame_line") continue;
      const list = map.get(binding.binding.target_id) ?? [];
      list.push(binding);
      map.set(binding.binding.target_id, list);
    }
    return map;
  }, [detail?.media_bindings]);

  const mediaBindingsByScene = useMemo(() => {
    const map = new Map<number, GalgameMediaBindingSummary[]>();
    for (const binding of detail?.media_bindings ?? []) {
      if (binding.binding.target_kind !== "galgame_scene") continue;
      const list = map.get(binding.binding.target_id) ?? [];
      list.push(binding);
      map.set(binding.binding.target_id, list);
    }
    return map;
  }, [detail?.media_bindings]);

  const choicesByLine = useMemo(() => {
    const map = new Map<number, GalgameChoice[]>();
    for (const scene of detail?.scenes ?? []) {
      for (const choice of scene.choices) {
        const list = map.get(choice.line_id) ?? [];
        list.push(choice);
        map.set(choice.line_id, list);
      }
    }
    for (const list of map.values()) {
      list.sort((left, right) => left.option_index - right.option_index);
    }
    return map;
  }, [detail?.scenes]);

  const {
    choiceDrafts, cgDrafts, acceptChoiceDraft, hasUnsavedDrafts,
    fieldDrafts, fieldDraftResetRevision, updateFieldDraft, clearFieldDraft,
    updateChoiceDraft, updateCgDraft, clearChoiceDraft, clearCgDraft, clearDrafts, saveCgDraft,
  } = useGalgameDrafts(projectId, choicesByLine);
  const hasUnsavedChanges = hasUnsavedDrafts || authoringDirty;
  useEffect(() => { onDirtyChange?.(hasUnsavedChanges); }, [hasUnsavedChanges, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  const discardAllDrafts = () => { clearDrafts(); setAuthoringDirty(false); setDiscardVersion((version) => version + 1); };
  useDirty(`galgame-editor:${projectId}`, hasUnsavedChanges, discardAllDrafts, () => busyTarget == null && !generating && !pipelineCancelling && !planningAssets && !generatingAssets && generatingAssetSpecId == null && !exportingWebgal);
  const confirmLeaveStage = async () => {
    if (busyTarget != null) return false;
    if (!hasUnsavedChanges) return true;
    const discard = await confirmDiscardMediaChanges(true, t);
    if (discard) discardAllDrafts();
    return discard;
  };

  const confirmDiscardScriptDrafts = async () => {
    if (!hasUnsavedDrafts) return true;
    const discard = await confirmDiscardMediaChanges(true, t);
    if (discard) clearDrafts();
    return discard;
  };

  const handleRefreshScript = async () => {
    if (busyTarget != null || !await confirmDiscardScriptDrafts()) return;
    await load();
  };

  const currentPipelineStartStage = (): GalgamePipelineStartStage | null =>
    pipelineStartStage || null;

  const pipelineRunning =
    generating || pipelineStatus?.run?.status === "running";
  const pipelineCanResume = Boolean(pipelineStatus?.run?.can_resume);
  const showPipelinePrimary = !detail || pipelineCanResume;
  const scriptStatusLabel = detail
    ? t("galgame.generated")
    : loading ? t("galgame.loadingScript")
      : loadError ? t("galgame.loadFailed") : t("galgame.notGenerated");
  const regenerateScriptLabel = pipelineStartStage
    ? t("galgame.regenerateFromStage", { stage: pipelineStartStageLabel(pipelineStartStage, t) })
    : t("galgame.regenerateFullScript");

  useEffect(() => {
    if (pipelineRunning || (!loading && !detail)) setPipelineControlsOpen(true);
  }, [detail, loading, pipelineRunning]);

  const handleGenerate = async (resumeFromCheckpoint = false, stopAfter: GalgamePipelineStopAfter | null = "script", guard = true) => {
    if (!canEditGalgame || generating || busyTarget != null || (!detail && (loading || loadError))) return;
    if (guard && !await confirmLeaveStage()) return;
    const taskId = createWorkflowTaskId("galgame-pipeline", projectId);
    setPipelineStatus(null);
    setGenerating(true);
    try {
      const completed = await runProgressTask(
        {
          title: t("galgame.pipelineTitle"),
          startMessage: t("galgame.preparingPipeline"),
          successMessage: t("galgame.pipelineCompleted"),
          errorMessage: t("galgame.pipelineFailed"),
          total: 7,
          cancelLabel: t("galgame.cancelPipeline"),
          runningHint: t("galgame.pipelineRunningHint"),
          cancelTaskId: taskId,
          pollTaskProgress: true,
          cancelTask: async () => {
            const nextStatus = await cancelGalgamePipeline(projectId);
            setPipelineStatus(nextStatus);
          },
        },
        async ({ setProgress }) => {
          setProgress(t("galgame.submittingPipeline"), 0, 7);
          const result = await startGalgamePipeline({
            project_id: projectId,
            task_id: taskId,
            start_from_stage: stopAfter === "branch_design" ? "branch_design" : currentPipelineStartStage(),
            resume_from_checkpoint: resumeFromCheckpoint,
            stop_after: stopAfter,
          });
          setProgress(t("galgame.refreshingPipeline"), 7, 7);
          const status = await getGalgamePipelineStatus(projectId);
          await authoring.reload();
          return { detail: result, status };
        },
      );
      const next = completed.detail;
      setPipelineStatus(completed.status);
      if (next) {
        setDetail(next);
        setSelectedSceneId(next.scenes[0]?.scene.id ?? null);
      }
      addToast(t("galgame.pipelineCompleted"), "success");
    } catch (err) {
      const message = extractErrorMessage(err, t("galgame.pipelineFailed"));
      const cancelled = isCancelledOperation(message);
      addToast(
        cancelled ? t("galgame.pipelineCancelled") : message,
        cancelled ? "info" : "error",
      );
      try {
        setPipelineStatus(await getGalgamePipelineStatus(projectId));
      } catch (statusError) {
        addToast(
          extractErrorMessage(statusError, t("galgame.pipelineStatusLoadFailed")),
          "error",
        );
      }
    } finally {
      setGenerating(false);
    }
  };

  const handleCancelPipeline = async () => {
    if (!canEditGalgame || pipelineCancelling || !pipelineRunning) return;
    setPipelineCancelling(true);
    try {
      const next = await cancelGalgamePipeline(projectId);
      setPipelineStatus(next);
      addToast(t("galgame.cancelRequested"), "info");
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.cancelFailed")), "error");
    } finally {
      setPipelineCancelling(false);
    }
  };

  const handlePreviewPipeline = async () => {
    if (!canEditGalgame || requestPreviewLoading || busyTarget != null) return;
    if (!await confirmDiscardScriptDrafts()) return;
    const seq = previewSeqRef.current + 1;
    previewSeqRef.current = seq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGalgamePipelineRequest({
        project_id: projectId,
        start_from_stage: currentPipelineStartStage(),
        resume_from_checkpoint: false,
        stop_after: workspaceStage === "branches" ? "branch_design" : "script",
      });
      if (previewSeqRef.current !== seq) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (previewSeqRef.current === seq) {
        addToast(
          extractErrorMessage(err, t("galgame.previewFailed")),
          "error",
        );
        setRequestPreviewOpen(false);
      }
    } finally {
      if (previewSeqRef.current === seq) setRequestPreviewLoading(false);
    }
  };

  const closeRequestPreview = () => {
    previewSeqRef.current += 1;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  };

  const handlePlanAssets = async () => {
    if (!canEditGalgame || planningAssets || busyTarget != null || !detail) return;
    if (!await confirmDiscardScriptDrafts()) return;
    setPlanningAssets(true);
    try {
      const next = await planGalgameAssets(projectId);
      requestSeqRef.current += 1;
      setLoading(false);
      setAssetPlan(next);
      setAssetPlanError(null);
      addToast(t("galgame.planWritten"), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.planFailed")), "error");
    } finally {
      setPlanningAssets(false);
    }
  };

  const handleGenerateAssets = async () => {
    if (!canEditGalgame || generatingAssets || busyTarget != null || !detail) return;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    if (!await confirmDiscardScriptDrafts()) return;
    setGeneratingAssets(true);
    try {
      const result = await runGalgameAssetGeneration({
        project_id: projectId,
        width: requestSize.width,
        height: requestSize.height,
      });
      await load();
      addToast(
        t("galgame.assetGenerationSummary", {
          generated: result.generated_count,
          skipped: result.skipped_completed_count,
        }),
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.assetGenerationFailed")), "error");
    } finally {
      setGeneratingAssets(false);
    }
  };

  const handleGenerateAssetSpec = async (generationSpecId: number) => {
    if (
      !canEditGalgame ||
      generatingAssets ||
      busyTarget != null ||
      generatingAssetSpecId != null ||
      !detail
    )
      return;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    if (!await confirmDiscardScriptDrafts()) return;
    setGeneratingAssetSpecId(generationSpecId);
    try {
      const result = await runGalgameAssetGeneration({
        project_id: projectId,
        generation_spec_ids: [generationSpecId],
        width: requestSize.width,
        height: requestSize.height,
      });
      await load();
      addToast(
        result.generated_count > 0
          ? t("galgame.portraitGenerated")
          : t("galgame.portraitReused"),
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.portraitGenerationFailed")), "error");
    } finally {
      setGeneratingAssetSpecId(null);
    }
  };

  const handleLinePatch = async (
    line: GalgameLine,
    patch: Partial<GalgameLine>,
  ) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(lineBusyTarget(line.id));
    try {
      const input: UpdateGalgameLineInput = { line_id: line.id };
      if ("line_type" in patch && patch.line_type != null)
        input.line_type = patch.line_type;
      if ("speaker_profile_id" in patch) {
        input.speaker_profile_id = patch.speaker_profile_id ?? null;
      }
      if ("text" in patch && patch.text != null) input.text = patch.text;
      if (
        "line_type" in patch ||
        "speaker_profile_id" in patch ||
        "position" in patch ||
        "expression" in patch ||
        "pose" in patch ||
        "presentation_json" in patch
      ) {
        input.presentation = presentationForLinePatch(line, patch);
      }
      const saved = await updateGalgameLine(projectId, input);
      commitDetail((current) => ({
        ...current,
        scenes: current.scenes.map((scene) => scene.scene.id === saved.scene_id
          ? { ...scene, lines: scene.lines.map((item) => item.id === saved.id ? saved : item) }
          : scene),
      }));
      if ("text" in patch || "pose" in patch) clearFieldDraft(`line:${line.id}`);
      if (patch.line_type != null && patch.line_type !== "choice") clearChoiceDraft(line.id);
      if (!await load()) return false;
      return true;
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.saveLineFailed")), "error");
      return false;
    } finally {
      setBusyTarget(null);
    }
  };

  const handleScenePatch = async (
    scene: GalgameScene,
    patch: Partial<GalgameScene>,
  ) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(sceneBusyTarget(scene.id));
    try {
      const input: UpdateGalgameSceneInput = { scene_id: scene.id };
      if ("title" in patch && patch.title != null) input.title = patch.title;
      if ("location" in patch) input.location = patch.location ?? null;
      if ("time_of_day" in patch) input.time_of_day = patch.time_of_day ?? null;
      if ("background_note" in patch)
        input.background_note = patch.background_note ?? null;
      if ("next_scene_id" in patch)
        input.next_scene_id = patch.next_scene_id ?? null;
      const saved = await updateGalgameScene(projectId, input);
      commitDetail((current) => ({
        ...current,
        scenes: current.scenes.map((item) => item.scene.id === saved.id ? { ...item, scene: saved } : item),
      }));
      clearFieldDraft(`scene:${scene.id}`);
      if (!await load()) return;
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.saveSceneFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleAddLine = async (
    scene: GalgameSceneDetail,
    afterLine: GalgameLine | null,
  ) => {
    if (!canEditGalgame || busyTarget != null) return;
    const inputText = await promptDialog({
      title: t("galgame.addNarration"),
      message: t("galgame.newNarrationPrompt"),
      confirmText: t("galgame.addNarration"),
    });
    if (inputText == null) return;
    const text = inputText.trim();
    if (!text) {
      addToast(t("galgame.emptyNarration"), "error");
      return;
    }
    setBusyTarget(insertBusyTarget(scene.scene.id, afterLine?.id ?? null));
    try {
      await insertGalgameLine(projectId, {
        scene_id: scene.scene.id,
        after_line_id: afterLine?.id ?? null,
        line_type: "narration",
        speaker_profile_id: null,
        text,
        presentation: emptyGalgamePresentation(),
        text_anchor_id: null,
      });
      if (!await load()) return;
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.insertLineFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleDeleteLine = async (line: GalgameLine) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(lineBusyTarget(line.id));
    try {
      await deleteGalgameLine(projectId, { line_id: line.id });
      clearChoiceDraft(line.id);
      clearCgDraft(line.id);
      clearFieldDraft(`line:${line.id}`);
      if (!await load()) return;
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.deleteLineFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleMoveLine = async (
    scene: GalgameSceneDetail,
    line: GalgameLine,
    direction: -1 | 1,
  ) => {
    if (!canEditGalgame || busyTarget != null) return;
    const index = scene.lines.findIndex((item) => item.id === line.id);
    const nextIndex = index + direction;
    if (index < 0 || nextIndex < 0 || nextIndex >= scene.lines.length) return;
    const lineIds = scene.lines.map((item) => item.id);
    [lineIds[index], lineIds[nextIndex]] = [lineIds[nextIndex], lineIds[index]];
    setBusyTarget(lineBusyTarget(line.id));
    try {
      await reorderGalgameLines(projectId, {
        scene_id: scene.scene.id,
        line_ids: lineIds,
      });
      if (!await load()) return;
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.reorderLineFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleChoicesUpdate = async (
    line: GalgameLine,
    choices: UpdateGalgameChoiceOptionInput[],
  ) => {
    if (!canEditGalgame || busyTarget != null) return;
    const submittedDraft = choiceDrafts[line.id];
    setBusyTarget(lineBusyTarget(line.id));
    try {
      await updateGalgameChoices(projectId, {
        line_id: line.id,
        choices,
      });
      if (!await load()) return;
      acceptChoiceDraft(line.id, submittedDraft);
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.saveChoicesFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleImportAudio = async (line: GalgameLine, role: "bgm" | "sfx") => {
    if (!canEditGalgame || busyTarget != null) return;
    const file = await pickLocalFile(
      "audio/mpeg,audio/wav,audio/ogg,.mp3,.wav,.ogg",
    );
    if (!file) return;
    if (!file.type) {
      addToast(t("galgame.invalidAudioMime"), "error");
      return;
    }
    setBusyTarget(audioBusyTarget(line.id, role));
    try {
      const saved = await importGalgameAudio({
        project_id: projectId,
        target_line_id: line.id,
        role,
        file_name: file.name,
        mime_type: file.type,
        data_base64: await readFileAsBase64(file),
      });
      commitMediaBinding(saved);
      addToast(
        t("galgame.audioImported", {
          role: role === "bgm" ? "BGM" : t("galgame.soundEffect"),
        }),
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.importAudioFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleUnbindMedia = async (binding: GalgameMediaBindingSummary) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(`media:${binding.binding.id}`);
    try {
      await unbindGalgameMedia({
        project_id: projectId,
        target_kind: binding.binding.target_kind,
        target_id: binding.binding.target_id,
        role: binding.binding.role,
        media_asset_id: binding.asset.id,
      });
      commitDetail((current) => ({
        ...current,
        media_bindings: current.media_bindings.filter((item) => item.binding.id !== binding.binding.id),
      }));
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.unbindMediaFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleBindMedia = async (
    targetKind: "galgame_scene" | "galgame_line",
    targetId: number,
    role: "background" | "cg" | "bgm" | "sfx",
    mediaAssetId: number,
  ) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(bindMediaBusyTarget(targetKind, targetId, role));
    try {
      const saved = await bindGalgameMedia({
        project_id: projectId,
        media_asset_id: mediaAssetId,
        target_kind: targetKind,
        target_id: targetId,
        role,
      });
      commitMediaBinding(saved);
      addToast(t("galgame.mediaBound"), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.bindMediaFailed")), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleExportWebgal = async () => {
    if (!detail || exportingWebgal) return;
    setExportingWebgal(true);
    try {
      downloadBase64ExportFile(await exportGalgameWebgal(projectId));
      addToast(t("galgame.exportedWebgal"), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.exportWebgalFailed")), "error");
    } finally {
      setExportingWebgal(false);
    }
  };

  const totalLines =
    detail?.scenes.reduce((sum, scene) => sum + scene.lines.length, 0) ?? 0;

  return (
    <div className="flex min-h-[60vh] flex-col gap-3 lg:h-full lg:min-h-0">
      {loadError ? <p role="alert" className="text-small text-error">{loadError}</p> : null}
      <MediaStageHeader id={stageId} label={t("common.mediaAuthoring.stages")} tabs={stageTabs} activeTab={workspaceStage}
        onTabChange={async (stage) => { if (!await confirmLeaveStage()) return false; if (workspaceStage === "materials") void load(); setWorkspaceStage(stage); return true; }}>
        {canEditGalgame && <BookButton size="sm" variant="secondary" disabled={pipelineRunning || loading} onClick={() => void handleGenerate(false, null)}>{t("common.mediaAuthoring.oneClick")}</BookButton>}
      </MediaStageHeader>
      {outputState?.stale && workspaceStage !== "adaptation" && workspaceStage !== "branches" && <p role="status" className="text-xs text-warning">{t("common.mediaAuthoring.outputsStale")}</p>}
      {authoring.error && workspaceStage !== "adaptation" && workspaceStage !== "branches" && <p role="alert" className="text-xs text-error">{authoring.error}</p>}
      <div id={`${stageId}-panel`} role="tabpanel" aria-labelledby={`${stageId}-tab-${workspaceStage}`} className="flex min-h-0 flex-1 flex-col gap-3">
      {workspaceStage === "materials" ? <BaseMaterialsWorkspace projectId={materialProjectId} canEdit={canEditGalgame}
        onChanged={load} onNext={() => { void load(); setWorkspaceStage("adaptation"); }} />
      : workspaceStage === "adaptation" || workspaceStage === "branches" ? <MediaAuthoringPanel key={`${projectId}:${workspaceStage}:${discardVersion}`} projectId={projectId}
        kind={workspaceStage === "adaptation" ? "adaptation_plan" : "galgame_branch_plan"}
        workspace={authoring.workspace} loading={authoring.loading} error={authoring.error} canEdit={canEditGalgame} stageRunning={pipelineRunning}
        onReload={authoring.reload} onDirtyChange={setAuthoringDirty} onGenerateStage={() => handleGenerate(false, "branch_design", false)}
        onNext={() => setWorkspaceStage(workspaceStage === "adaptation" ? "branches" : "script")} />
      : workspaceStage === "asset_plan" || workspaceStage === "asset_generation" ? <AssetPlanPanel inline mode={workspaceStage === "asset_plan" ? "planning" : "generation"}
        plan={assetPlan} error={assetPlanError} speakerProfiles={detail?.speaker_profiles ?? []} loading={planningAssets || loading}
        generating={generatingAssets} generatingSpecId={generatingAssetSpecId} imageSize={imageSize} imageSizes={imageSizes}
        imageSizeLoading={imageSizeLoading} imageSizeError={imageSizeError} onImageSizeChange={setImageSize}
        disabled={!canEditGalgame || planningAssets || generatingAssets || generatingAssetSpecId != null || busyTarget != null || !detail}
        onPlan={handlePlanAssets} onGenerate={handleGenerateAssets} onGenerateSpec={handleGenerateAssetSpec} />
      : workspaceStage === "export" ? <section className="space-y-4 overflow-y-auto p-1 custom-scrollbar">
        <h2 className="text-base font-semibold">{t("common.mediaAuthoring.export")}</h2>
        <p className="text-sm text-book-text-muted">{t("common.mediaAuthoring.exportHint")}</p>
        {detail && <p className="text-sm">{t("galgame.sceneCount", { scenes: detail.scenes.length, lines: totalLines })}</p>}
        <BookButton size="sm" disabled={!canEditGalgame || !detail || exportingWebgal} onClick={handleExportWebgal}>{exportingWebgal ? <LoadingSpinner size="sm" /> : <Download size={14} />}{t("galgame.exportWebgal")}</BookButton>
      </section> : <>
      <div className="lg:hidden">
        <SegmentedControl
          ariaLabel={t("galgame.mobileWorkspace")}
          value={mobileWorkspaceView}
          onChange={setMobileWorkspaceView}
          options={[
            { id: "scenes", label: t("galgame.scenes") },
            { id: "editor", label: t("galgame.editor") },
          ]}
        />
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 lg:grid-cols-[248px_minmax(0,1fr)] lg:gap-4">
        <aside
          aria-label={t("galgame.sceneAndPipeline")}
          className={`${mobileWorkspaceView === "scenes" ? "block" : "hidden"} min-h-0 space-y-3 lg:block lg:overflow-y-auto custom-scrollbar`}
        >
          <div className="flex items-start gap-3 px-1 pb-1">
            <div className="min-w-0 flex-1">
              <h2 className="text-xs font-semibold text-book-text-main">
                {t("galgame.scenes")}
              </h2>
              <p className="mt-0.5 text-caption text-book-text-muted">
                {detail
                  ? t("galgame.sceneCount", { scenes: detail.scenes.length, lines: totalLines })
                  : scriptStatusLabel}
              </p>
            </div>
            <button
              type="button"
              onClick={() => void handleRefreshScript()}
              disabled={loading || generating || pipelineCancelling || busyTarget != null}
              className="pyqt-btn-icon !h-9 !w-9 !rounded-xl sm:!h-8 sm:!w-8"
              aria-label={t("galgame.refreshScript")}
              title={t("common.refresh")}
            >
              {loading ? (
                <LoadingSpinner size="sm" />
              ) : (
                <RefreshCw size={13} />
              )}
            </button>
            {detail && canEditGalgame ? (
              <Dropdown
                items={[
                  {
                    label: exportingWebgal
                      ? t("galgame.exportingWebgal")
                      : t("galgame.exportWebgal"),
                    icon: exportingWebgal ? (
                      <LoadingSpinner size="sm" />
                    ) : (
                      <Download size={13} />
                    ),
                    onClick: handleExportWebgal,
                    disabled: exportingWebgal,
                  },
                ]}
              />
            ) : null}
          </div>
          <details
            aria-label={t("galgame.scriptGeneration")}
            className="a4n-flat-surface overflow-hidden"
            open={pipelineControlsOpen}
            onToggle={(event) =>
              setPipelineControlsOpen(event.currentTarget.open)
            }
          >
            <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 text-xs font-medium text-book-text-main hover:bg-book-bg-tertiary/55 [&::-webkit-details-marker]:hidden">
              <span>{t("galgame.scriptGeneration")}</span>
              <span className="flex-1" />
              <span className="text-small font-normal text-book-text-muted">
                {pipelineRunning
                  ? t("galgame.generating")
                  : pipelineCanResume
                    ? t("galgame.resumable")
                    : scriptStatusLabel}
              </span>
            </summary>
            <div className="border-t border-book-border/40 px-3 pb-3 pt-2.5">
              {pipelineRunning ? (
                <BookButton
                  variant="secondary"
                  size="sm"
                  className="w-full"
                  onClick={handleCancelPipeline}
                  disabled={!canEditGalgame || pipelineCancelling}
                >
                  {pipelineCancelling ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <X size={13} />
                  )}
                  {t("galgame.stopGeneration")}
                </BookButton>
              ) : showPipelinePrimary ? (
                <BookButton
                  variant="primary"
                  size="sm"
                  className="w-full"
                  onClick={() => handleGenerate(pipelineCanResume)}
                  disabled={!canEditGalgame || generating || pipelineCancelling || busyTarget != null || (!detail && (loading || loadError != null))}
                >
                  {generating ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <FileText size={14} />
                  )}
                  {pipelineStartStage ? regenerateScriptLabel : pipelineCanResume ? t("galgame.continueGeneration") : t("galgame.generateScript")}
                </BookButton>
              ) : null}
              {!pipelineRunning ? (
                <div className="mt-2 flex items-start justify-between gap-2">
                  <details className="min-w-0 flex-1">
                    <summary className="cursor-pointer rounded-lg px-1 py-1 text-caption text-book-text-muted hover:bg-book-bg-tertiary/55 hover:text-book-text-main">
                      {t("galgame.generationOptions")} · {pipelineStartStageLabel(pipelineStartStage, t)}
                    </summary>
                    <ThemedSelect
                      aria-label={t("galgame.generationOptions")}
                      className="pyqt-input mt-1.5 w-full text-xs"
                      value={pipelineStartStage}
                      disabled={
                        !canEditGalgame || generating || pipelineCancelling
                      }
                      onChange={(selectedValue) =>
                        setPipelineStartStage(
                          selectedValue as GalgamePipelineStartStage | "",
                        )
                      }
                    >
                      {PIPELINE_START_STAGES.map((stage) => (
                        <option key={stage.value || "all"} value={stage.value}>
                          {t(stage.labelKey)}
                        </option>
                      ))}
                    </ThemedSelect>
                  </details>
                  <Dropdown
                    items={[
                      {
                        label: requestPreviewLoading
                          ? t("galgame.preparingModelRequest")
                          : t("galgame.viewModelRequest"),
                        icon: requestPreviewLoading ? (
                          <LoadingSpinner size="sm" />
                        ) : (
                          <FileSearch size={13} />
                        ),
                        onClick: handlePreviewPipeline,
                        disabled:
                          !canEditGalgame ||
                          requestPreviewLoading ||
                          generating ||
                          pipelineCancelling ||
                          busyTarget != null,
                      },
                      ...(detail
                        ? [
                            { type: "divider" as const },
                            {
                              label: regenerateScriptLabel,
                              icon: <RefreshCw size={13} />,
                              onClick: () => handleGenerate(false),
                              disabled:
                                !canEditGalgame ||
                                generating ||
                                pipelineCancelling ||
                                busyTarget != null,
                            },
                          ]
                        : []),
                    ]}
                  />
                </div>
              ) : null}
            </div>
          </details>
          <PipelineStatusPanel
            status={pipelineStatus}
            loading={loading || generating}
          />
          {detail && detail.scenes.length > 0 ? (
            <div className="a4n-flat-surface divide-y divide-book-border/35 overflow-hidden">
              {detail.scenes.map((item) => {
                const chapter = chapterBySnapshotId.get(
                  item.scene.source_snapshot_chapter_id,
                );
                const active = item.scene.id === selectedScene?.scene.id;
                const nextScene =
                  item.scene.next_scene_id != null
                    ? (sceneById.get(item.scene.next_scene_id) ?? null)
                    : null;
                return (
                  <button
                    key={item.scene.id}
                    type="button"
                    aria-current={active ? "true" : undefined}
                    className={`w-full px-3 py-2.5 text-left text-xs transition-colors ${
                      active
                        ? "bg-book-primary/8 text-book-text-main shadow-[inset_3px_0_0_rgb(var(--color-primary))]"
                        : "text-book-text-muted hover:bg-book-bg-tertiary/55 hover:text-book-text-main"
                    }`}
                    onClick={() => { void (async () => { if (await confirmLeaveStage()) { setSelectedSceneId(item.scene.id); setMobileWorkspaceView("editor"); } })(); }}
                  >
                    <div className="truncate font-semibold">
                      {item.scene.scene_index}. {item.scene.title}
                    </div>
                    <div className="mt-1 truncate text-small">
                      {chapter
                        ? t("novel.chapterRangeSingle", { number: chapter.chapter_number })
                        : t("galgame.sourceChapter")}
                      {" · "}
                      {t("galgame.lineCount", { count: item.lines.length })}
                    </div>
                    <div className="mt-1 text-caption leading-relaxed text-book-text-subtle">
                      {sceneEntryKindLabel(item.scene.entry_kind, t)}
                      {item.scene.next_scene_id != null ? (
                        <span
                          className={
                            nextScene &&
                            !(
                              item.scene.entry_kind === "branch" &&
                              nextScene.scene.entry_kind !== "main"
                            )
                              ? ""
                              : "text-error"
                          }
                        >
                          {" · "}
                          {nextScene
                            ? item.scene.entry_kind === "branch" &&
                              nextScene.scene.entry_kind !== "main"
                              ? t("galgame.nextSceneNonMain", {
                                  number: nextScene.scene.scene_index,
                                })
                              : t("galgame.nextSceneLabel", {
                                  number: nextScene.scene.scene_index,
                                })
                            : t("galgame.nextSceneMissingLabel")}
                        </span>
                      ) : item.scene.entry_kind === "branch" ? (
                        <span className="text-error">
                          {t("galgame.branchNotConverged")}
                        </span>
                      ) : null}
                    </div>
                  </button>
                );
              })}
            </div>
          ) : null}
        </aside>

        <section
          aria-label={t("galgame.editor")}
          className={`${mobileWorkspaceView === "editor" ? "block" : "hidden"} a4n-flat-surface min-h-0 overflow-hidden ${
            selectedScene ? "lg:flex lg:flex-col" : "self-start lg:block"
          }`}
        >
          <div className="flex items-start gap-3 border-b border-book-border/40 px-3 py-3 sm:px-4">
            <div className="min-w-0 flex-1">
              <h2 className="truncate text-sm font-semibold text-book-text-main">
                {selectedScene?.scene.title ?? t("galgame.workspace")}
              </h2>
              <p className="mt-0.5 truncate text-caption text-book-text-muted">
                {selectedScene
                  ? [
                      selectedScene.scene.background_note,
                      sceneEntryKindLabel(selectedScene.scene.entry_kind, t),
                      selectedSourceChapter
                        ? t("novel.chapterRangeSingle", { number: selectedSourceChapter.chapter_number })
                        : t("galgame.sourceChapter"),
                      t("galgame.lineCount", { count: selectedScene.lines.length }),
                    ]
                      .filter(Boolean)
                      .join(" · ")
                  : loadError ? t("galgame.loadFailed") : t("galgame.generatedEditorHint")}
              </p>
            </div>
            {selectedScene && canEditGalgame ? (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() =>
                  handleAddLine(
                    selectedScene,
                    selectedScene.lines.length
                      ? selectedScene.lines[selectedScene.lines.length - 1]
                      : null,
                  )
                }
                disabled={busyTarget != null}
                className="!min-h-9 !px-2.5 !text-xs sm:!min-h-8"
              >
                <Plus size={14} />
                {t("galgame.addNarration")}
              </BookButton>
            ) : null}
          </div>

          {selectedScene ? (
            <SceneMetaEditor
              key={`${selectedScene.scene.id}:${discardVersion}`}
              scene={selectedScene.scene}
              scenes={detail?.scenes ?? []}
              mediaBindings={
                mediaBindingsByScene.get(selectedScene.scene.id) ?? []
              }
              disabled={!canEditGalgame || busyTarget != null}
              fieldDraftResetRevision={fieldDraftResetRevision}
              fieldDraft={fieldDrafts[`scene:${selectedScene.scene.id}`] ?? null}
              onPatch={(patch) => handleScenePatch(selectedScene.scene, patch)}
              onDraftChange={updateFieldDraft}
              onBindMedia={(mediaAssetId) =>
                handleBindMedia(
                  "galgame_scene",
                  selectedScene.scene.id,
                  "background",
                  mediaAssetId,
                )
              }
              onUnbindMedia={handleUnbindMedia}
            />
          ) : null}

          <div
            className={`p-3 lg:p-4 custom-scrollbar ${
              selectedScene
                ? "min-h-[60vh] lg:min-h-0 lg:flex-1 lg:overflow-y-auto"
                : ""
            }`}
          >
            {loading && !detail ? (
              <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
                <LoadingSpinner size="sm" />
                {t("galgame.loadingScript")}
              </div>
            ) : !detail && loadError ? null : !selectedScene ? (
              <div className="m-auto flex min-h-[160px] max-w-md flex-col items-center justify-center gap-3 text-xs text-book-text-muted">
                <PenLine size={22} />
                <span>{t("galgame.scriptNotGenerated")}</span>
              </div>
            ) : (
              <div className="divide-y divide-book-border/40">
                {selectedScene.lines.map((line, index) => (
                  <LineEditor
                    key={`${line.id}:${discardVersion}`}
                    line={line}
                    speakerProfiles={detail?.speaker_profiles ?? []}
                    speakerProfileById={speakerProfileById}
                    sourceAnchor={
                      line.text_anchor_id
                        ? (textAnchorById.get(line.text_anchor_id) ?? null)
                        : null
                    }
                    sourceChapter={
                      line.text_anchor_id
                        ? (chapterBySnapshotId.get(
                            textAnchorById.get(line.text_anchor_id)
                              ?.source_snapshot_chapter_id ?? -1,
                          ) ?? null)
                        : null
                    }
                    disabled={!canEditGalgame || busyTarget != null}
                    saving={busyTarget === lineBusyTarget(line.id)}
                    fieldDraftResetRevision={fieldDraftResetRevision}
                    fieldDraft={fieldDrafts[`line:${line.id}`] ?? null}
                    onDraftChange={updateFieldDraft}
                    mediaBindings={mediaBindingsByLine.get(line.id) ?? []}
                    choiceDrafts={choiceDrafts[line.id] ?? buildChoiceDrafts(choicesByLine.get(line.id) ?? [])}
                    onChoiceDraftsChange={(drafts) => updateChoiceDraft(line.id, drafts)}
                    cgDraft={cgDrafts[line.id] ?? buildCgDraft(line.presentation_json)}
                    onCgDraftChange={(draft) => updateCgDraft(line, draft)}
                    onCgSave={(draft) => saveCgDraft(line, draft, handleLinePatch)}
                    scenes={detail?.scenes ?? []}
                    sceneById={sceneById}
                    onPatch={(patch) => handleLinePatch(line, patch)}
                    onChoicesUpdate={(choices) =>
                      handleChoicesUpdate(line, choices)
                    }
                    onMoveUp={() => handleMoveLine(selectedScene, line, -1)}
                    onMoveDown={() => handleMoveLine(selectedScene, line, 1)}
                    canMoveUp={index > 0}
                    canMoveDown={index < selectedScene.lines.length - 1}
                    onInsertAfter={() => handleAddLine(selectedScene, line)}
                    onDelete={() => handleDeleteLine(line)}
                    onImportAudio={(role) => handleImportAudio(line, role)}
                    onBindMedia={(role, mediaAssetId) =>
                      handleBindMedia(
                        "galgame_line",
                        line.id,
                        role,
                        mediaAssetId,
                      )
                    }
                    onUnbindMedia={handleUnbindMedia}
                  />
                ))}
              </div>
            )}
          </div>
        </section>
      </div>
      </>}
      </div>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("galgame.requestTitle")}
        onClose={closeRequestPreview}
      />
    </div>
  );
};
