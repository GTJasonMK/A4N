import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { Download,
  FileText,
  FileSearch,
  Images,
  Music,
  PenLine,
  Plus,
  RefreshCw,
  Trash2,
  X, } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { ImageSizeSelect, useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { MediaAssetIdBinder } from "./galgame/MediaAssetIdBinder";
import { PipelineStatusPanel } from "./galgame/PipelineStatusPanel";
import {
  presentationCgNote,
  updatePresentationJson,
  type PresentationPatch,
} from "./galgame/presentation";
import {
  bindGalgameMedia,
  cancelGalgamePipeline,
  deleteGalgameLine,
  exportGalgameWebgal,
  getGalgamePipelineStatus,
  getGalgameScript,
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
  type GalgameLinePresentationDocumentV2,
  type GalgameLineType,
  type GalgameCgImportance,
  type GalgameTransition,
  type GalgameStagePositionV2,
  type GalgameVisibleCastMemberV2,
  type GalgameAssetPlan,
  type GalgameAssetPlanItem,
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
import type { ImageSize } from "../../api/image";
import { extractErrorMessage } from "../../api";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { ThemedCombobox } from "../../components/ui/ThemedCombobox";
import type { SourceSnapshotChapterSummary } from "../../api/project";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";
import { toMediaSrc } from "../../utils/mediaSrc";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { downloadBase64ExportFile } from "../../utils/downloadFile";
import { useI18n } from "../../i18n";
import {
  emptyGalgamePresentation,
  pipelineStartStageLabel,
  sceneEntryKindLabel,
  sceneSelectLabel,
} from "./galgame/galgameTabShared";
import { AssetPlanPanel, PortraitExpressionMatrix } from "./galgame/AssetPlanPanels";
import { LineEditor, SceneMetaEditor } from "./galgame/GalgameLineEditors";


interface GalgameTabProps {
  projectId: number;
  canEditGalgame: boolean;
  snapshotChapters: SourceSnapshotChapterSummary[];
}

export const GalgameTab: React.FC<GalgameTabProps> = ({
  projectId,
  canEditGalgame,
  snapshotChapters,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
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
  const [generating, setGenerating] = useState(false);
  const [pipelineCancelling, setPipelineCancelling] = useState(false);
  const [planningAssets, setPlanningAssets] = useState(false);
  const [generatingAssets, setGeneratingAssets] = useState(false);
  const [generatingAssetSpecId, setGeneratingAssetSpecId] = useState<
    number | null
  >(null);
  const [exportingWebgal, setExportingWebgal] = useState(false);
  const [assetPlan, setAssetPlan] = useState<GalgameAssetPlan | null>(null);
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

  const load = React.useCallback(async () => {
    const seq = requestSeqRef.current + 1;
    requestSeqRef.current = seq;
    setLoading(true);
    try {
      const [statusResult, scriptResult] = await Promise.allSettled([
        getGalgamePipelineStatus(projectId),
        getGalgameScript(projectId),
      ]);
      if (requestSeqRef.current !== seq) return;
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
        setDetail(scriptResult.value);
        setSelectedSceneId(
          (current) =>
            current ?? scriptResult.value.scenes[0]?.scene.id ?? null,
        );
      } else {
        setDetail(null);
        const message = extractErrorMessage(scriptResult.reason, "");
        if (!message.includes("不存在")) {
          addToast(message || t("galgame.loadFailed"), "error");
        }
      }
    } finally {
      if (requestSeqRef.current === seq) setLoading(false);
    }
  }, [addToast, projectId]);

  useEffect(() => {
    setDetail(null);
    setPipelineStatus(null);
    setSelectedSceneId(null);
    setMobileWorkspaceView("scenes");
    void load();
    return () => {
      requestSeqRef.current += 1;
    };
  }, [load]);

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

  const currentPipelineStartStage = (): GalgamePipelineStartStage | null =>
    pipelineStartStage || null;

  const pipelineRunning =
    generating || pipelineStatus?.run?.status === "running";
  const pipelineCanResume = Boolean(pipelineStatus?.run?.can_resume);
  const showPipelinePrimary = !detail || pipelineCanResume;
  const regenerateScriptLabel = pipelineStartStage
    ? t("galgame.regenerateFromStage", { stage: pipelineStartStageLabel(pipelineStartStage, t) })
    : t("galgame.regenerateFullScript");

  useEffect(() => {
    if (pipelineRunning || (!loading && !detail)) setPipelineControlsOpen(true);
  }, [detail, loading, pipelineRunning]);

  const handleGenerate = async (resumeFromCheckpoint = false) => {
    if (!canEditGalgame || generating) return;
    const taskId = createWorkflowTaskId("galgame-pipeline", projectId);
    setGenerating(true);
    try {
      const next = await runProgressTask(
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
            start_from_stage: currentPipelineStartStage(),
            resume_from_checkpoint: resumeFromCheckpoint,
          });
          setProgress(t("galgame.refreshingPipeline"), 7, 7);
          return result;
        },
      );
      const nextStatus = await getGalgamePipelineStatus(projectId);
      setDetail(next);
      setPipelineStatus(nextStatus);
      setSelectedSceneId(next.scenes[0]?.scene.id ?? null);
      addToast(t("galgame.pipelineCompleted"), "success");
    } catch (err) {
      const message = extractErrorMessage(err, t("galgame.pipelineFailed"));
      const cancelled = message.includes("取消");
      addToast(
        cancelled ? t("galgame.pipelineCancelled") : message,
        cancelled ? "info" : "error",
      );
      const nextStatus = await getGalgamePipelineStatus(projectId).catch(
        () => pipelineStatus,
      );
      setPipelineStatus(nextStatus);
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
    if (!canEditGalgame || requestPreviewLoading) return;
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
    if (!canEditGalgame || planningAssets || !detail) return;
    setPlanningAssets(true);
    try {
      const next = await planGalgameAssets(projectId);
      setAssetPlan(next);
      addToast(t("galgame.planWritten"), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.planFailed")), "error");
    } finally {
      setPlanningAssets(false);
    }
  };

  const handleGenerateAssets = async () => {
    if (!canEditGalgame || generatingAssets || !detail) return;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    setGeneratingAssets(true);
    try {
      const result = await runGalgameAssetGeneration({
        project_id: projectId,
        width: requestSize.width,
        height: requestSize.height,
      });
      const nextDetail = await getGalgameScript(projectId);
      setDetail(nextDetail);
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
      generatingAssetSpecId != null ||
      !detail
    )
      return;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    setGeneratingAssetSpecId(generationSpecId);
    try {
      const result = await runGalgameAssetGeneration({
        project_id: projectId,
        generation_spec_ids: [generationSpecId],
        width: requestSize.width,
        height: requestSize.height,
      });
      setAssetPlan(await planGalgameAssets(projectId));
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
      await updateGalgameLine(projectId, input);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, t("galgame.saveLineFailed")), "error");
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
      await updateGalgameScene(projectId, input);
      await load();
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
    const inputText = window.prompt(t("galgame.newNarrationPrompt"));
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
      await load();
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
      await load();
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
      await load();
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
    setBusyTarget(lineBusyTarget(line.id));
    try {
      await updateGalgameChoices(projectId, {
        line_id: line.id,
        choices,
      });
      await load();
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
      await importGalgameAudio({
        project_id: projectId,
        target_line_id: line.id,
        role,
        file_name: file.name,
        mime_type: file.type,
        data_base64: await readFileAsBase64(file),
      });
      await load();
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
      await load();
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
      await bindGalgameMedia({
        project_id: projectId,
        media_asset_id: mediaAssetId,
        target_kind: targetKind,
        target_id: targetId,
        role,
      });
      await load();
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
                  : t("galgame.notGenerated")}
              </p>
            </div>
            <button
              type="button"
              onClick={load}
              disabled={loading || generating || pipelineCancelling}
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
                    : detail
                      ? t("galgame.generated")
                      : t("galgame.notGenerated")}
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
                  disabled={!canEditGalgame || generating || pipelineCancelling}
                >
                  {generating ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <FileText size={14} />
                  )}
                  {pipelineCanResume ? t("galgame.continueGeneration") : t("galgame.generateScript")}
                </BookButton>
              ) : null}
              {!pipelineRunning ? (
                <div className="mt-2 flex items-start justify-between gap-2">
                  <details className="min-w-0 flex-1">
                    <summary className="cursor-pointer rounded-lg px-1 py-1 text-caption text-book-text-muted hover:bg-book-bg-tertiary/55 hover:text-book-text-main">
                      {t("galgame.generationOptions")} · {pipelineStartStageLabel(pipelineStartStage, t)}
                    </summary>
                    <ThemedSelect
                      className="pyqt-input mt-1.5 w-full text-xs"
                      value={pipelineStartStage}
                      disabled={
                        !canEditGalgame || generating || pipelineCancelling
                      }
                      onChange={(event) =>
                        setPipelineStartStage(
                          event.target.value as GalgamePipelineStartStage | "",
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
                          pipelineCancelling,
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
                                pipelineCancelling,
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
                    onClick={() => {
                      setSelectedSceneId(item.scene.id);
                      setMobileWorkspaceView("editor");
                    }}
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
          <AssetPlanPanel
            plan={assetPlan}
            speakerProfiles={detail?.speaker_profiles ?? []}
            loading={planningAssets}
            generating={generatingAssets}
            generatingSpecId={generatingAssetSpecId}
            imageSize={imageSize}
            imageSizes={imageSizes}
            imageSizeLoading={imageSizeLoading}
            imageSizeError={imageSizeError}
            onImageSizeChange={setImageSize}
            disabled={
              !canEditGalgame ||
              planningAssets ||
              generatingAssets ||
              generatingAssetSpecId != null ||
              !detail
            }
            onPlan={handlePlanAssets}
            onGenerate={handleGenerateAssets}
            onGenerateSpec={handleGenerateAssetSpec}
          />
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
                  : t("galgame.generatedEditorHint")}
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
              scene={selectedScene.scene}
              scenes={detail?.scenes ?? []}
              mediaBindings={
                mediaBindingsByScene.get(selectedScene.scene.id) ?? []
              }
              disabled={!canEditGalgame || busyTarget != null}
              onPatch={(patch) => handleScenePatch(selectedScene.scene, patch)}
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
            {loading ? (
              <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
                <LoadingSpinner size="sm" />
                {t("galgame.loadingScript")}
              </div>
            ) : !selectedScene ? (
              <div className="m-auto flex min-h-[160px] max-w-md flex-col items-center justify-center gap-3 text-xs text-book-text-muted">
                <PenLine size={22} />
                <span>{t("galgame.scriptNotGenerated")}</span>
              </div>
            ) : (
              <div className="divide-y divide-book-border/40">
                {selectedScene.lines.map((line, index) => (
                  <LineEditor
                    key={line.id}
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
                    mediaBindings={mediaBindingsByLine.get(line.id) ?? []}
                    choices={choicesByLine.get(line.id) ?? []}
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
