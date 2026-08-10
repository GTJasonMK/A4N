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
import { MediaAssetIdBinder } from "./galgame/MediaAssetIdBinder";
import { PipelineStatusPanel } from "./galgame/PipelineStatusPanel";
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
import { extractErrorMessage } from "../../api";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import type { SourceSnapshotChapter } from "../../api/project";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";
import { toMediaSrc } from "../../utils/mediaSrc";
import { allocateHostTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { downloadBase64ExportFile } from "../../utils/downloadFile";

interface GalgameTabProps {
  projectId: number;
  canEditGalgame: boolean;
  snapshotChapters: SourceSnapshotChapter[];
}

const LINE_TYPES: Array<{ value: GalgameLineType; label: string }> = [
  { value: "narration", label: "旁白" },
  { value: "dialogue", label: "对白" },
  { value: "monologue", label: "独白" },
  { value: "stage_direction", label: "演出" },
  { value: "choice", label: "选择" },
];

const SPEAKING_LINE_TYPES = new Set(["dialogue", "monologue"]);

const POSITIONS: Array<{
  value: GalgameStagePositionV2 | "";
  label: string;
}> = [
  { value: "", label: "无站位" },
  { value: "left", label: "左" },
  { value: "center", label: "中" },
  { value: "right", label: "右" },
];

const EXPRESSIONS = [
  { value: "", label: "无表情" },
  { value: "neutral", label: "自然" },
  { value: "smile", label: "微笑" },
  { value: "sad", label: "悲伤" },
  { value: "angry", label: "生气" },
  { value: "surprised", label: "惊讶" },
  { value: "shy", label: "害羞" },
  { value: "serious", label: "严肃" },
];

const POSE_SUGGESTIONS = [
  { value: "natural", label: "自然" },
  { value: "standing", label: "站立" },
  { value: "sitting", label: "坐姿" },
  { value: "walking", label: "行走" },
  { value: "turning", label: "转身" },
  { value: "close_up", label: "近景" },
];

const DEFAULT_CG_IMPORTANCE = "normal" as const;
const CG_IMPORTANCE_OPTIONS: Array<{
  value: GalgameCgImportance | "";
  label: string;
}> = [
  { value: "", label: "CG 优先级" },
  { value: "normal", label: "普通" },
  { value: "high", label: "重要" },
  { value: "critical", label: "关键" },
];

const TRANSITION_OPTIONS = [
  { value: "", label: "无转场" },
  { value: "cut", label: "直接切换" },
  { value: "fade", label: "淡入淡出" },
  { value: "dissolve", label: "叠化" },
];

const PIPELINE_START_STAGES: Array<{
  value: GalgamePipelineStartStage | "";
  label: string;
}> = [
  { value: "", label: "从头开始" },
  { value: "shooting_script_projection", label: "拍摄剧本投影" },
  { value: "script_adaptation", label: "剧本改编" },
  { value: "character_staging", label: "角色演出" },
  { value: "visual_planning", label: "视觉规划" },
  { value: "branch_design", label: "分支设计" },
  { value: "persist_script", label: "写入剧本" },
  { value: "asset_generation", label: "资产意图" },
];

const lineBusyTarget = (lineId: number): string => `line:${lineId}`;
const sceneBusyTarget = (sceneId: number): string => `scene:${sceneId}`;
const audioBusyTarget = (lineId: number, role: "bgm" | "sfx"): string =>
  `audio:${lineId}:${role}`;
const bindMediaBusyTarget = (
  targetKind: "galgame_scene" | "galgame_line",
  targetId: number,
  role: "background" | "cg" | "bgm" | "sfx",
): string => `bind-media:${targetKind}:${targetId}:${role}`;
const insertBusyTarget = (
  sceneId: number,
  afterLineId: number | null,
): string => `insert:${sceneId}:${afterLineId ?? "start"}`;

export const GalgameTab: React.FC<GalgameTabProps> = ({
  projectId,
  canEditGalgame,
  snapshotChapters,
}) => {
  const { addToast } = useToast();
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
          extractErrorMessage(statusResult.reason, "加载 Galgame 管线状态失败"),
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
          addToast(message || "加载 Galgame 剧本失败", "error");
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
    const map = new Map<number, SourceSnapshotChapter>();
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
    ? `从${pipelineStartStageLabel(pipelineStartStage)}重新生成`
    : "重新生成完整剧本";

  useEffect(() => {
    if (pipelineRunning || (!loading && !detail)) setPipelineControlsOpen(true);
  }, [detail, loading, pipelineRunning]);

  const handleGenerate = async (resumeFromCheckpoint = false) => {
    if (!canEditGalgame || generating) return;
    const taskId = allocateHostTaskId("galgame-pipeline", projectId);
    setGenerating(true);
    try {
      const next = await runProgressTask(
        {
          title: "Galgame 管线",
          startMessage: "正在准备 Galgame 管线",
          successMessage: "Galgame 管线已完成",
          errorMessage: "运行 Galgame 管线失败",
          total: 7,
          cancelLabel: "取消管线",
          runningHint: "取消后已写入的检查点会保留，可从检查点续跑。",
          cancelTaskId: taskId,
          pollTaskProgress: true,
          cancelTask: async () => {
            const nextStatus = await cancelGalgamePipeline(projectId);
            setPipelineStatus(nextStatus);
          },
        },
        async ({ setProgress }) => {
          setProgress("正在提交 Galgame 管线...", 0, 7);
          const result = await startGalgamePipeline({
            project_id: projectId,
            task_id: taskId,
            start_from_stage: currentPipelineStartStage(),
            resume_from_checkpoint: resumeFromCheckpoint,
          });
          setProgress("正在刷新 Galgame 管线状态...", 7, 7);
          return result;
        },
      );
      const nextStatus = await getGalgamePipelineStatus(projectId);
      setDetail(next);
      setPipelineStatus(nextStatus);
      setSelectedSceneId(next.scenes[0]?.scene.id ?? null);
      addToast("Galgame 管线已完成", "success");
    } catch (err) {
      const message = extractErrorMessage(err, "运行 Galgame 管线失败");
      const cancelled = message.includes("取消");
      addToast(
        cancelled ? "Galgame 管线已取消" : message,
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
      addToast("已请求取消 Galgame 管线", "info");
    } catch (err) {
      addToast(extractErrorMessage(err, "取消 Galgame 管线失败"), "error");
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
          extractErrorMessage(err, "预览 Galgame 管线请求失败"),
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
      addToast("Galgame 资产规划已写入生成意图", "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "规划 Galgame 资产失败"), "error");
    } finally {
      setPlanningAssets(false);
    }
  };

  const handleGenerateAssets = async () => {
    if (!canEditGalgame || generatingAssets || !detail) return;
    setGeneratingAssets(true);
    try {
      const result = await runGalgameAssetGeneration({ project_id: projectId });
      const nextDetail = await getGalgameScript(projectId);
      setDetail(nextDetail);
      addToast(
        `Galgame 视觉资产已生成 ${result.generated_count} 项，跳过 ${result.skipped_completed_count} 项`,
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, "生成 Galgame 视觉资产失败"), "error");
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
    setGeneratingAssetSpecId(generationSpecId);
    try {
      const result = await runGalgameAssetGeneration({
        project_id: projectId,
        generation_spec_ids: [generationSpecId],
      });
      setAssetPlan(await planGalgameAssets(projectId));
      addToast(
        result.generated_count > 0 ? "角色差分已生成" : "角色差分已复用",
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, "生成角色差分失败"), "error");
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
      addToast(extractErrorMessage(err, "保存剧本行失败"), "error");
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
      addToast(extractErrorMessage(err, "保存场信息失败"), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleAddLine = async (
    scene: GalgameSceneDetail,
    afterLine: GalgameLine | null,
  ) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(insertBusyTarget(scene.scene.id, afterLine?.id ?? null));
    try {
      await insertGalgameLine(projectId, {
        scene_id: scene.scene.id,
        after_line_id: afterLine?.id ?? null,
        line_type: "narration",
        speaker_profile_id: null,
        text: "新的剧本行",
        presentation: emptyGalgamePresentation(),
        text_anchor_id: null,
      });
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, "新增剧本行失败"), "error");
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
      addToast(extractErrorMessage(err, "删除剧本行失败"), "error");
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
      addToast(extractErrorMessage(err, "调整剧本行顺序失败"), "error");
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
      addToast(extractErrorMessage(err, "保存选择支失败"), "error");
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
      addToast("无法识别音频 MIME 类型", "error");
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
        role === "bgm" ? "BGM 已导入并绑定" : "音效已导入并绑定",
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, "导入 Galgame 音频失败"), "error");
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
      addToast(extractErrorMessage(err, "解绑 Galgame 媒体失败"), "error");
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
      addToast("Galgame 媒体已绑定", "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "绑定 Galgame 媒体失败"), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleExportWebgal = async () => {
    if (!detail || exportingWebgal) return;
    setExportingWebgal(true);
    try {
      downloadBase64ExportFile(await exportGalgameWebgal(projectId));
      addToast("WebGAL 工程已导出", "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "导出 WebGAL 工程失败"), "error");
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
          ariaLabel="Galgame 移动工作区"
          value={mobileWorkspaceView}
          onChange={setMobileWorkspaceView}
          options={[
            { id: "scenes", label: "场次" },
            { id: "editor", label: "编辑" },
          ]}
        />
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 lg:grid-cols-[248px_minmax(0,1fr)] lg:gap-4">
        <aside
          aria-label="Galgame 场次与管线"
          className={`${mobileWorkspaceView === "scenes" ? "block" : "hidden"} min-h-0 space-y-3 lg:block lg:overflow-y-auto custom-scrollbar`}
        >
          <div className="flex items-start gap-3 px-1 pb-1">
            <div className="min-w-0 flex-1">
              <h2 className="text-xs font-semibold text-book-text-main">
                场次
              </h2>
              <p className="mt-0.5 text-caption text-book-text-muted">
                {detail
                  ? `${detail.scenes.length} 场 · ${totalLines} 行`
                  : "尚未生成"}
              </p>
            </div>
            <button
              type="button"
              onClick={load}
              disabled={loading || generating || pipelineCancelling}
              className="pyqt-btn-icon !h-9 !w-9 !rounded-xl sm:!h-8 sm:!w-8"
              aria-label="刷新 Galgame 剧本"
              title="刷新"
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
                      ? "正在导出 WebGAL 工程"
                      : "导出 WebGAL 工程",
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
            aria-label="Galgame 剧本生成"
            className="a4n-flat-surface overflow-hidden"
            open={pipelineControlsOpen}
            onToggle={(event) =>
              setPipelineControlsOpen(event.currentTarget.open)
            }
          >
            <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 text-xs font-medium text-book-text-main hover:bg-book-bg-tertiary/55 [&::-webkit-details-marker]:hidden">
              <span>剧本生成</span>
              <span className="flex-1" />
              <span className="text-small font-normal text-book-text-muted">
                {pipelineRunning
                  ? "生成中"
                  : pipelineCanResume
                    ? "可继续"
                    : detail
                      ? "已生成"
                      : "未生成"}
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
                  停止生成
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
                  {pipelineCanResume ? "继续生成" : "生成剧本"}
                </BookButton>
              ) : null}
              {!pipelineRunning ? (
                <div className="mt-2 flex items-start justify-between gap-2">
                  <details className="min-w-0 flex-1">
                    <summary className="cursor-pointer rounded-lg px-1 py-1 text-caption text-book-text-muted hover:bg-book-bg-tertiary/55 hover:text-book-text-main">
                      生成选项 · {pipelineStartStageLabel(pipelineStartStage)}
                    </summary>
                    <select
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
                          {stage.label}
                        </option>
                      ))}
                    </select>
                  </details>
                  <Dropdown
                    items={[
                      {
                        label: requestPreviewLoading
                          ? "正在准备模型请求"
                          : "查看模型请求",
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
                        ? `第 ${chapter.chapter_number} 章`
                        : "来源章节"}
                      {" · "}
                      {item.lines.length} 行
                    </div>
                    <div className="mt-1 text-caption leading-relaxed text-book-text-subtle">
                      {sceneEntryKindLabel(item.scene.entry_kind)}
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
                              ? `→ 非主线场 ${nextScene.scene.scene_index}`
                              : `→ 场次 ${nextScene.scene.scene_index}`
                            : "→ 下一场缺失"}
                        </span>
                      ) : item.scene.entry_kind === "branch" ? (
                        <span className="text-error"> · 未回收敛</span>
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
          aria-label="Galgame 剧本编辑区"
          className={`${mobileWorkspaceView === "editor" ? "block" : "hidden"} a4n-flat-surface min-h-0 overflow-hidden ${
            selectedScene ? "lg:flex lg:flex-col" : "self-start lg:block"
          }`}
        >
          <div className="flex items-start gap-3 border-b border-book-border/40 px-3 py-3 sm:px-4">
            <div className="min-w-0 flex-1">
              <h2 className="truncate text-sm font-semibold text-book-text-main">
                {selectedScene?.scene.title ?? "Galgame 剧本"}
              </h2>
              <p className="mt-0.5 truncate text-caption text-book-text-muted">
                {selectedScene
                  ? [
                      selectedScene.scene.background_note,
                      sceneEntryKindLabel(selectedScene.scene.entry_kind),
                      selectedSourceChapter
                        ? `第 ${selectedSourceChapter.chapter_number} 章`
                        : "来源章节",
                      `${selectedScene.lines.length} 行`,
                    ]
                      .filter(Boolean)
                      .join(" · ")
                  : "生成后可编辑场和行"}
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
                添加行
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
                加载 Galgame 剧本
              </div>
            ) : !selectedScene ? (
              <div className="m-auto flex min-h-[160px] max-w-md flex-col items-center justify-center gap-3 text-xs text-book-text-muted">
                <PenLine size={22} />
                <span>尚未生成剧本</span>
              </div>
            ) : (
              <div className="divide-y divide-book-border/40">
                {selectedScene.lines.map((line, index) => (
                  <LineEditor
                    key={line.id}
                    line={line}
                    speakerProfiles={detail?.speaker_profiles ?? []}
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
        fallbackTitle="Galgame 生成请求"
        onClose={closeRequestPreview}
      />
    </div>
  );
};

interface LineEditorProps {
  line: GalgameLine;
  speakerProfiles: GalgameSpeakerProfile[];
  sourceAnchor: GalgameTextAnchorSummary | null;
  sourceChapter: SourceSnapshotChapter | null;
  mediaBindings: GalgameMediaBindingSummary[];
  choices: GalgameChoice[];
  scenes: GalgameSceneDetail[];
  sceneById: Map<number, GalgameSceneDetail>;
  disabled: boolean;
  saving: boolean;
  onPatch: (patch: Partial<GalgameLine>) => void;
  onChoicesUpdate: (choices: UpdateGalgameChoiceOptionInput[]) => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  canMoveUp: boolean;
  canMoveDown: boolean;
  onInsertAfter: () => void;
  onDelete: () => void;
  onImportAudio: (role: "bgm" | "sfx") => void;
  onBindMedia: (role: "cg" | "bgm" | "sfx", mediaAssetId: number) => void;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}

const AssetPlanPanel: React.FC<{
  plan: GalgameAssetPlan | null;
  speakerProfiles: GalgameSpeakerProfile[];
  loading: boolean;
  generating: boolean;
  generatingSpecId: number | null;
  disabled: boolean;
  onPlan: () => void;
  onGenerate: () => void;
  onGenerateSpec: (generationSpecId: number) => void;
}> = ({
  plan,
  speakerProfiles,
  loading,
  generating,
  generatingSpecId,
  disabled,
  onPlan,
  onGenerate,
  onGenerateSpec,
}) => {
  const planSummary = plan
    ? [
        plan.summary.background_specs > 0
          ? `背景 ${plan.summary.background_specs}`
          : null,
        plan.summary.portrait_expression_specs > 0
          ? `角色差分 ${plan.summary.portrait_expression_specs}`
          : null,
        plan.summary.event_cg_specs > 0
          ? `事件 CG ${plan.summary.event_cg_specs}`
          : null,
      ]
        .filter(Boolean)
        .join(" · ")
    : null;
  const outcomeSummary = plan
    ? [
        plan.summary.created_specs > 0
          ? `新增 ${plan.summary.created_specs}`
          : null,
        plan.summary.updated_specs > 0
          ? `更新 ${plan.summary.updated_specs}`
          : null,
        plan.summary.reused_specs > 0
          ? `复用 ${plan.summary.reused_specs}`
          : null,
      ]
        .filter(Boolean)
        .join(" · ")
    : null;
  return (
    <details
      aria-label="资产规划"
      className="a4n-flat-surface overflow-hidden text-xs"
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 font-medium text-book-text-main hover:bg-book-bg-tertiary/55 [&::-webkit-details-marker]:hidden">
        <span>视觉资产</span>
        <span className="flex-1" />
        {loading || generating ? (
          <LoadingSpinner size="sm" />
        ) : null}
        <span className="text-small font-normal text-book-text-muted">
          {plan ? "已规划" : "未规划"}
        </span>
      </summary>
      <div className="border-t border-book-border/40 px-3 py-3">
        <BookButton
          variant="primary"
          size="sm"
          className="w-full"
          onClick={plan ? onGenerate : onPlan}
          disabled={disabled}
        >
          {loading || generating ? (
            <LoadingSpinner size="sm" />
          ) : (
            <Images size={13} />
          )}
          {plan ? "生成视觉资产" : "规划视觉资产"}
        </BookButton>
        {plan ? (
          <div className="mt-2 flex justify-end">
            <Dropdown
              items={[
                {
                  label: "重新规划视觉资产",
                  icon: <RefreshCw size={13} />,
                  onClick: onPlan,
                  disabled,
                },
              ]}
            />
          </div>
        ) : null}
        {!plan ? (
          <div className="mt-2 text-small text-book-text-muted">
            规划后可统一生成背景、角色差分与事件 CG。
          </div>
        ) : (
          <div className="mt-2 space-y-2 text-small text-book-text-muted">
            {planSummary ? <p>{planSummary}</p> : null}
            {outcomeSummary ? (
              <p className="text-caption text-book-text-subtle">
                {outcomeSummary}
              </p>
            ) : null}
            <AssetPlanList title="背景" items={plan.backgrounds} />
            <PortraitExpressionMatrix
              items={plan.portrait_expressions}
              speakerProfiles={speakerProfiles}
              disabled={disabled}
              generatingSpecId={generatingSpecId}
              onGenerateSpec={onGenerateSpec}
            />
            <AssetPlanList title="事件 CG" items={plan.event_cgs} />
          </div>
        )}
      </div>
    </details>
  );
};

const AssetPlanList: React.FC<{
  title: string;
  items: GalgameAssetPlan["backgrounds"];
}> = ({ title, items }) => {
  if (!items.length) return null;
  return (
    <div className="border-t border-book-border pt-2">
      <div className="mb-1 font-semibold text-book-text-main">{title}</div>
      <div className="space-y-1">
        {items.slice(0, 4).map((item) => (
          <div
            key={item.canonical_key}
            className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2"
          >
            <span className="min-w-0 truncate" title={item.label}>
              {item.label}
            </span>
            <span>{assetPlanOutcomeLabel(item.outcome)}</span>
          </div>
        ))}
        {items.length > 4 ? <div>还有 {items.length - 4} 项</div> : null}
      </div>
    </div>
  );
};

interface PortraitExpressionCell {
  rowKey: string;
  profileId: number;
  profileName: string;
  pose: string;
  expression: string;
  item: GalgameAssetPlanItem;
}

interface PortraitExpressionMatrixRow {
  rowKey: string;
  firstCell: PortraitExpressionCell;
}

const PortraitExpressionMatrix: React.FC<{
  items: GalgameAssetPlanItem[];
  speakerProfiles: GalgameSpeakerProfile[];
  disabled: boolean;
  generatingSpecId: number | null;
  onGenerateSpec: (generationSpecId: number) => void;
}> = ({
  items,
  speakerProfiles,
  disabled,
  generatingSpecId,
  onGenerateSpec,
}) => {
  const cells = useMemo(
    () => buildPortraitExpressionCells(items, speakerProfiles),
    [items, speakerProfiles],
  );
  const { rows, expressionValues, cellsByKey } = useMemo(
    () => buildPortraitExpressionMatrixModel(cells),
    [cells],
  );
  if (!cells.length) return null;

  return (
    <div className="border-t border-book-border pt-2">
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className="font-semibold text-book-text-main">角色差分</span>
        <span className="text-caption tabular-nums text-book-text-subtle">
          {cells.length} 项
        </span>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[360px] border-collapse text-small">
          <thead>
            <tr className="text-left text-book-text-muted">
              <th className="border border-book-border bg-book-bg-paper px-2 py-1 font-medium">
                角色
              </th>
              {expressionValues.map((expression) => (
                <th
                  key={expression || "none"}
                  className="border border-book-border bg-book-bg-paper px-2 py-1 font-medium"
                >
                  {portraitExpressionLabel(expression)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map(({ rowKey, firstCell }) => {
              return (
                <tr key={rowKey}>
                  <th className="max-w-28 border border-book-border bg-book-bg-paper px-2 py-1 text-left font-medium text-book-text-main">
                    <div className="truncate" title={firstCell.profileName}>
                      {firstCell.profileName}
                    </div>
                    <div
                      className="truncate text-book-text-muted"
                      title={poseDisplayValue(firstCell.pose) || "默认姿势"}
                    >
                      {poseDisplayValue(firstCell.pose) || "默认姿势"}
                    </div>
                  </th>
                  {expressionValues.map((expression) => {
                    const cell = cellsByKey.get(`${rowKey}:${expression}`);
                    const generating =
                      generatingSpecId === cell?.item.generation_spec_id;
                    return (
                      <td
                        key={`${rowKey}:${expression || "none"}`}
                        className="border border-book-border bg-book-bg px-1 py-1 align-top"
                      >
                        {cell ? (
                          <div className="flex min-w-20 flex-col gap-1">
                            <span
                              className={`truncate ${
                                cell.item.status === "failed"
                                  ? "text-error"
                                  : "text-book-text-muted"
                              }`}
                              title={cell.item.label}
                            >
                              {assetSpecStatusLabel(cell.item.status)}
                            </span>
                            <BookButton
                              variant="secondary"
                              size="sm"
                              className="h-7 w-full"
                              disabled={disabled}
                              onClick={() =>
                                onGenerateSpec(cell.item.generation_spec_id)
                              }
                            >
                              {generating ? (
                                <LoadingSpinner size="sm" />
                              ) : (
                                <RefreshCw size={12} />
                              )}
                              {cell.item.status === "completed"
                                ? "检查"
                                : "生成"}
                            </BookButton>
                          </div>
                        ) : (
                          <span className="text-book-text-muted">-</span>
                        )}
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
};

const SceneMetaEditor: React.FC<{
  scene: GalgameScene;
  scenes: GalgameSceneDetail[];
  mediaBindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  onPatch: (patch: Partial<GalgameScene>) => void;
  onBindMedia: (mediaAssetId: number) => void;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({
  scene,
  scenes,
  mediaBindings,
  disabled,
  onPatch,
  onBindMedia,
  onUnbindMedia,
}) => {
  const [draft, setDraft] = useState({
    title: scene.title,
    location: scene.location ?? "",
    time_of_day: scene.time_of_day ?? "",
    background_note: scene.background_note ?? "",
    next_scene_id: scene.next_scene_id?.toString() ?? "",
  });
  const availableNextScenes = useMemo(
    () =>
      scenes.filter(
        (item) =>
          item.scene.id !== scene.id &&
          (scene.entry_kind !== "branch" || item.scene.entry_kind === "main"),
      ),
    [scene.entry_kind, scene.id, scenes],
  );
  const selectedNextSceneSelectable =
    scene.next_scene_id == null ||
    availableNextScenes.some((item) => item.scene.id === scene.next_scene_id);

  useEffect(() => {
    setDraft({
      title: scene.title,
      location: scene.location ?? "",
      time_of_day: scene.time_of_day ?? "",
      background_note: scene.background_note ?? "",
      next_scene_id: scene.next_scene_id?.toString() ?? "",
    });
  }, [
    scene.background_note,
    scene.location,
    scene.next_scene_id,
    scene.time_of_day,
    scene.title,
  ]);

  const commit = (
    key: "title" | "location" | "time_of_day" | "background_note",
    value: string,
  ) => {
    const current =
      key === "title"
        ? scene.title
        : key === "location"
          ? (scene.location ?? "")
          : key === "time_of_day"
            ? (scene.time_of_day ?? "")
            : (scene.background_note ?? "");
    if (value === current) return;
    onPatch({ [key]: key === "title" ? value : value || null });
  };
  const backgroundBindings = mediaBindings.filter(
    (item) => item.binding.role === "background",
  );

  return (
    <div className="grid gap-3 border-b border-book-border bg-book-bg px-4 py-3 text-xs lg:grid-cols-[minmax(0,1fr)_220px]">
      <div className="grid gap-2 md:grid-cols-[1.2fr_0.8fr_0.8fr]">
        <input
          className="pyqt-input text-book-text-main"
          value={draft.title}
          disabled={disabled}
          onChange={(event) =>
            setDraft((current) => ({ ...current, title: event.target.value }))
          }
          onBlur={() => commit("title", draft.title)}
        />
        <input
          className="pyqt-input text-book-text-main"
          value={draft.location}
          disabled={disabled}
          placeholder="地点"
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              location: event.target.value,
            }))
          }
          onBlur={() => commit("location", draft.location)}
        />
        <input
          className="pyqt-input text-book-text-main"
          value={draft.time_of_day}
          disabled={disabled}
          placeholder="时间"
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              time_of_day: event.target.value,
            }))
          }
          onBlur={() => commit("time_of_day", draft.time_of_day)}
        />
        <textarea
          className="pyqt-input min-h-16 text-book-text-main md:col-span-3"
          value={draft.background_note}
          disabled={disabled}
          placeholder="背景需求"
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              background_note: event.target.value,
            }))
          }
          onBlur={() => commit("background_note", draft.background_note)}
        />
        <select
          className="pyqt-input text-book-text-main md:col-span-3"
          value={draft.next_scene_id}
          disabled={disabled}
          onChange={(event) => {
            const value = event.target.value;
            setDraft((current) => ({ ...current, next_scene_id: value }));
            onPatch({ next_scene_id: value ? Number(value) : null });
          }}
        >
          <option value="">
            {scene.entry_kind === "branch" ? "未回收敛" : "无下一场"}
          </option>
          {!selectedNextSceneSelectable && scene.next_scene_id != null ? (
            <option value={scene.next_scene_id}>
              {scenes.some((item) => item.scene.id === scene.next_scene_id)
                ? "当前下一场不是可用主线场"
                : "当前下一场已缺失"}
            </option>
          ) : null}
          {availableNextScenes.map((item) => (
            <option key={item.scene.id} value={item.scene.id}>
              {sceneSelectLabel(item.scene)}
            </option>
          ))}
        </select>
      </div>
      <details className="rounded-xl border border-book-border/40 bg-book-bg-paper/45 p-2.5">
        <summary className="cursor-pointer text-small font-medium text-book-text-main">
          背景素材 ·{" "}
          {backgroundBindings.length
            ? `${backgroundBindings.length} 项`
            : "未绑定"}
        </summary>
        <div className="mt-2">
          <ImageBindingStrip
            title="背景"
            emptyLabel=""
            bindings={backgroundBindings}
            disabled={disabled}
            bindControl={
              <MediaAssetIdBinder
                label="绑定背景素材"
                disabled={disabled}
                onBind={onBindMedia}
              />
            }
            onUnbindMedia={onUnbindMedia}
          />
        </div>
      </details>
    </div>
  );
};

const LineEditor: React.FC<LineEditorProps> = ({
  line,
  speakerProfiles,
  sourceAnchor,
  sourceChapter,
  mediaBindings,
  choices,
  scenes,
  sceneById,
  disabled,
  saving,
  onPatch,
  onChoicesUpdate,
  onMoveUp,
  onMoveDown,
  canMoveUp,
  canMoveDown,
  onInsertAfter,
  onDelete,
  onImportAudio,
  onBindMedia,
  onUnbindMedia,
}) => {
  const [text, setText] = useState(line.text);
  const [pose, setPose] = useState(poseDisplayValue(line.pose ?? ""));
  const isSpeakingLine = SPEAKING_LINE_TYPES.has(line.line_type);
  const isChoiceLine = line.line_type === "choice";
  const expressionValue = selectValue(line.expression, EXPRESSIONS);
  const missingSpeakingExpression =
    isSpeakingLine && expressionValue.length === 0;
  const poseDatalistId = `galgame-pose-suggestions-${line.id}`;
  const speaker = useMemo(
    () =>
      speakerProfiles.find(
        (profile) => profile.id === line.speaker_profile_id,
      ) ?? null,
    [line.speaker_profile_id, speakerProfiles],
  );

  useEffect(() => {
    setText(line.text);
    setPose(poseDisplayValue(line.pose ?? ""));
  }, [line.pose, line.text]);

  const firstSpeakerId = speakerProfiles[0]?.id ?? null;

  return (
    <article className="py-4 first:pt-0 last:pb-0">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <span className="min-w-8 text-small font-semibold text-book-text-muted">
          第 {line.line_index} 行
        </span>
        <select
          className="pyqt-input text-xs text-book-text-main"
          value={line.line_type}
          disabled={disabled}
          onChange={(event) => {
            const nextType = event.target.value as GalgameLineType;
            onPatch({
              line_type: nextType,
              speaker_profile_id: SPEAKING_LINE_TYPES.has(nextType)
                ? (line.speaker_profile_id ?? firstSpeakerId)
                : null,
              expression: SPEAKING_LINE_TYPES.has(nextType)
                ? expressionValue || "neutral"
                : null,
            });
          }}
        >
          {LINE_TYPES.map((item) => (
            <option
              key={item.value}
              value={item.value}
              disabled={
                SPEAKING_LINE_TYPES.has(item.value) &&
                speakerProfiles.length === 0
              }
            >
              {item.label}
            </option>
          ))}
        </select>
        {isSpeakingLine ? (
          <select
            className="pyqt-input text-xs text-book-text-main"
            value={line.speaker_profile_id ?? ""}
            disabled={disabled || speakerProfiles.length === 0}
            onChange={(event) =>
              onPatch({
                speaker_profile_id: event.target.value
                  ? Number(event.target.value)
                  : null,
                expression: event.target.value
                  ? expressionValue || "neutral"
                  : line.expression,
              })
            }
          >
            <option value="">
              {speakerProfiles.length === 0 ? "无角色档案" : "未绑定角色"}
            </option>
            {speakerProfiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </select>
        ) : null}
        {!isChoiceLine ? (
          <select
            className="pyqt-input text-xs text-book-text-main"
            value={line.position ?? ""}
            disabled={disabled}
            onChange={(event) => {
              const position = event.target.value as GalgameStagePositionV2 | "";
              onPatch({ position: position || null });
            }}
          >
            {POSITIONS.map((item) => (
              <option key={item.value} value={item.value}>
                {item.label}
              </option>
            ))}
          </select>
        ) : null}
        {isSpeakingLine ? (
          <span
            className={`max-w-[180px] truncate text-small ${
              speaker && !missingSpeakingExpression
                ? "text-book-text-muted"
                : "text-error"
            }`}
            title={
              speaker
                ? missingSpeakingExpression
                  ? "对白或独白必须设置表情"
                  : speaker.name
                : "未绑定角色"
            }
          >
            {speaker
              ? missingSpeakingExpression
                ? "表情未设置"
                : `角色 ${speaker.name}`
              : "角色未绑定"}
          </span>
        ) : null}
        {line.text_anchor_id ? (
          <span
            className={`max-w-[220px] truncate text-small ${
              sourceAnchor ? "text-book-text-muted" : "text-error"
            }`}
            title={
              sourceAnchor
                ? `${sourceChapter ? `第 ${sourceChapter.chapter_number} 章` : "来源章节"} · P${
                    sourceAnchor.paragraph_index + 1
                  } · ${sourceAnchor.quote}`
                : "来源锚点未随剧本返回"
            }
          >
            {sourceAnchor
              ? `${sourceChapter ? `第 ${sourceChapter.chapter_number} 章` : "源快照"} · ${
                  sourceAnchor.quote
                }`
              : "来源锚点缺失"}
          </span>
        ) : null}
        <div className="ml-auto flex items-center gap-1">
          {saving ? (
            <LoadingSpinner size="sm" className="text-book-text-muted" />
          ) : null}
          <Dropdown
            items={[
              {
                label: "上移一行",
                onClick: onMoveUp,
                disabled: disabled || !canMoveUp,
              },
              {
                label: "下移一行",
                onClick: onMoveDown,
                disabled: disabled || !canMoveDown,
              },
              {
                label: "在下方插入",
                icon: <Plus size={13} />,
                onClick: onInsertAfter,
                disabled,
              },
              { type: "divider" },
              {
                label: "删除此行",
                icon: <Trash2 size={13} />,
                onClick: onDelete,
                disabled,
                danger: true,
              },
            ]}
          />
        </div>
      </div>
      <textarea
        className="pyqt-input min-h-20 w-full resize-y px-3 py-2 text-xs leading-relaxed text-book-text-main"
        value={text}
        disabled={disabled}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => {
          if (text !== line.text) onPatch({ text });
        }}
      />
      {!isChoiceLine ? (
        <div className="mt-2 grid gap-2 md:grid-cols-2">
          <select
            className="pyqt-input text-xs text-book-text-main"
            value={expressionValue}
            disabled={disabled}
            onChange={(event) =>
              onPatch({ expression: event.target.value || null })
            }
          >
            {EXPRESSIONS.map((item) => (
              <option
                key={item.value || "none"}
                value={item.value}
                disabled={isSpeakingLine && item.value === ""}
              >
                {item.label}
              </option>
            ))}
          </select>
          <input
            list={poseDatalistId}
            className="pyqt-input text-xs text-book-text-main"
            value={pose}
            disabled={disabled}
            placeholder="姿势"
            onChange={(event) => setPose(event.target.value)}
            onBlur={() => {
              const storedPose = poseStoredValue(pose);
              if (storedPose !== (line.pose ?? ""))
                onPatch({ pose: storedPose || null });
            }}
          />
          <datalist id={poseDatalistId}>
            {POSE_SUGGESTIONS.map((item) => (
              <option key={item.value} value={item.label} />
            ))}
          </datalist>
          <PresentationEditor
            line={line}
            disabled={disabled}
            onPatch={(presentationJson) =>
              onPatch({ presentation_json: presentationJson })
            }
          />
        </div>
      ) : (
        <ChoiceEditor
          line={line}
          choices={choices}
          scenes={scenes}
          sceneById={sceneById}
          disabled={disabled}
          onUpdate={onChoicesUpdate}
        />
      )}
      <LineMediaBindings
        bindings={mediaBindings}
        disabled={disabled}
        onImportAudio={onImportAudio}
        onBindMedia={onBindMedia}
        onUnbindMedia={onUnbindMedia}
      />
    </article>
  );
};

const PresentationEditor: React.FC<{
  line: GalgameLine;
  disabled: boolean;
  onPatch: (presentationJson: GalgameLinePresentationDocumentV2) => void;
}> = ({ line, disabled, onPatch }) => {
  const presentation = line.presentation_json;
  const currentCgEnabled =
    presentation.cg === true || presentationCgNote(presentation).length > 0;
  const currentCgImportance: GalgameCgImportance | "" =
    presentation.cg_importance ||
    (currentCgEnabled ? DEFAULT_CG_IMPORTANCE : "");
  const currentTransition: GalgameTransition | "" =
    presentation.transition || "";
  const [cgNote, setCgNote] = useState(presentationCgNote(presentation));
  const [cgEnabled, setCgEnabled] = useState(currentCgEnabled);
  const [cgImportance, setCgImportance] =
    useState<GalgameCgImportance | "">(currentCgImportance);
  const [transition, setTransition] =
    useState<GalgameTransition | "">(currentTransition);

  useEffect(() => {
    setCgNote(presentationCgNote(presentation));
    setCgEnabled(currentCgEnabled);
    setCgImportance(currentCgImportance);
    setTransition(currentTransition);
  }, [currentCgEnabled, currentCgImportance, currentTransition, presentation]);

  const update = (patch: PresentationPatch) => {
    onPatch(updatePresentationJson(presentation, patch));
  };

  return (
    <div className="rounded-xl bg-book-bg-paper/45 px-2.5 py-2.5 md:col-span-2">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <button
          type="button"
          role="switch"
          aria-checked={cgEnabled}
          disabled={disabled}
          onClick={() => {
            const checked = !cgEnabled;
            setCgEnabled(checked);
            if (!checked) {
              setCgNote("");
              setCgImportance("");
              update({ cgEnabled: false });
              return;
            }
            const nextImportance = cgImportance || DEFAULT_CG_IMPORTANCE;
            if (!cgImportance) setCgImportance(nextImportance);
            if (cgNote.trim()) {
              update({ cgNote, cgImportance: nextImportance });
            }
          }}
          className="inline-flex items-center gap-1.5 text-small text-book-text-main disabled:opacity-50"
        >
          <span
            className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
              cgEnabled ? "bg-book-primary" : "bg-book-text-muted/25"
            }`}
          >
            <span
              className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                cgEnabled ? "translate-x-4" : "translate-x-[2px]"
              }`}
            />
          </span>
          事件 CG
        </button>
        {cgEnabled ? (
          <select
            className="pyqt-input text-xs text-book-text-main"
            value={cgImportance}
            disabled={disabled}
            onChange={(event) => {
              const importance = event.target.value as GalgameCgImportance | "";
              setCgImportance(importance);
              update({ cgImportance: importance });
            }}
          >
            {CG_IMPORTANCE_OPTIONS.map((item) => (
              <option key={item.value || "none"} value={item.value}>
                {item.label}
              </option>
            ))}
          </select>
        ) : null}
        <select
          className="pyqt-input text-xs text-book-text-main"
          value={transition}
          disabled={disabled}
          onChange={(event) => {
            const nextTransition = event.target.value as GalgameTransition | "";
            setTransition(nextTransition);
            update({ transition: nextTransition });
          }}
        >
          {TRANSITION_OPTIONS.map((item) => (
            <option key={item.value || "none"} value={item.value}>
              {item.label}
            </option>
          ))}
        </select>
      </div>
      {cgEnabled ? (
        <textarea
          className="min-h-16 w-full resize-y rounded-xl border border-book-border/55 bg-book-bg px-2.5 py-2 text-xs leading-relaxed text-book-text-main outline-none focus:border-book-primary"
          value={cgNote}
          disabled={disabled}
          placeholder="事件 CG 画面描述"
          onChange={(event) => setCgNote(event.target.value)}
          onBlur={() => {
            if (cgNote !== presentationCgNote(presentation)) {
              if (cgNote.trim()) {
                setCgEnabled(true);
                if (!cgImportance) setCgImportance(DEFAULT_CG_IMPORTANCE);
                update({
                  cgNote,
                  cgImportance: cgImportance || DEFAULT_CG_IMPORTANCE,
                });
              } else {
                update({ cgNote: "" });
              }
            }
          }}
        />
      ) : null}
    </div>
  );
};

interface ChoiceDraft {
  option_text: string;
  target_scene_id: string;
}

const ChoiceEditor: React.FC<{
  line: GalgameLine;
  choices: GalgameChoice[];
  scenes: GalgameSceneDetail[];
  sceneById: Map<number, GalgameSceneDetail>;
  disabled: boolean;
  onUpdate: (choices: UpdateGalgameChoiceOptionInput[]) => void;
}> = ({ line, choices, scenes, sceneById, disabled, onUpdate }) => {
  const [drafts, setDrafts] = useState<ChoiceDraft[]>(() =>
    buildChoiceDrafts(line, choices, scenes, sceneById),
  );

  useEffect(() => {
    setDrafts(buildChoiceDrafts(line, choices, scenes, sceneById));
  }, [choices, line, sceneById, scenes]);

  const targetInspections = useMemo(
    () =>
      drafts.map((draft) =>
        inspectChoiceTarget(draft.target_scene_id, sceneById),
      ),
    [drafts, sceneById],
  );
  const normalizedChoices = useMemo(() => {
    return drafts
      .map((draft) => {
        const optionText = draft.option_text.trim();
        const targetSceneId = Number(draft.target_scene_id);
        if (
          !optionText ||
          !Number.isInteger(targetSceneId) ||
          !sceneById.has(targetSceneId)
        ) {
          return null;
        }
        return {
          option_text: optionText,
          target_scene_id: targetSceneId,
        };
      })
      .filter((item): item is UpdateGalgameChoiceOptionInput => item != null);
  }, [drafts, sceneById]);

  const structurallyComplete =
    drafts.length >= 2 &&
    drafts.length <= 3 &&
    normalizedChoices.length === drafts.length;
  const targetIssueCount = targetInspections.filter(
    (item) => item.kind === "error",
  ).length;
  const canSave = structurallyComplete && targetIssueCount === 0;
  const validationMessage =
    scenes.length === 0
      ? "没有可用目标场"
      : structurallyComplete
        ? targetIssueCount > 0
          ? `${targetIssueCount} 个目标需要修正`
          : "选择支目标可验收"
        : "选择支需要 2 到 3 个完整选项";

  const updateDraft = (index: number, patch: Partial<ChoiceDraft>) => {
    setDrafts((current) =>
      current.map((draft, itemIndex) =>
        itemIndex === index ? { ...draft, ...patch } : draft,
      ),
    );
  };

  const addDraft = () => {
    if (drafts.length >= 3) return;
    setDrafts((current) => [
      ...current,
      {
        option_text: "",
        target_scene_id:
          defaultChoiceTargetSceneId(line, scenes, sceneById)?.toString() ?? "",
      },
    ]);
  };

  const removeDraft = (index: number) => {
    if (drafts.length <= 2) return;
    setDrafts((current) =>
      current.filter((_, itemIndex) => itemIndex !== index),
    );
  };

  return (
    <div className="mt-2 rounded-xl bg-book-bg-paper/45 p-2.5">
      <div className="space-y-2">
        {drafts.map((draft, index) => {
          const inspection = targetInspections[index];
          return (
            <div key={`${line.id}:${index}`} className="space-y-1">
              <div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_minmax(140px,220px)_auto]">
                <input
                  className="border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
                  value={draft.option_text}
                  disabled={disabled}
                  placeholder={`选项 ${index + 1}`}
                  onChange={(event) =>
                    updateDraft(index, { option_text: event.target.value })
                  }
                />
                <select
                  className="pyqt-input text-xs text-book-text-main"
                  value={draft.target_scene_id}
                  disabled={disabled || scenes.length === 0}
                  onChange={(event) =>
                    updateDraft(index, { target_scene_id: event.target.value })
                  }
                >
                  {scenes.length === 0 ? (
                    <option value="">无目标场</option>
                  ) : null}
                  {scenes.map((item) => (
                    <option key={item.scene.id} value={item.scene.id}>
                      {sceneSelectLabel(item.scene)}
                    </option>
                  ))}
                </select>
                {drafts.length > 2 ? (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => removeDraft(index)}
                    disabled={disabled}
                    aria-label={`删除选项 ${index + 1}`}
                  >
                    <Trash2 size={13} />
                  </BookButton>
                ) : null}
              </div>
              {inspection ? (
                <ChoiceTargetSummary inspection={inspection} />
              ) : null}
            </div>
          );
        })}
      </div>
      {targetInspections.some((item) => item.kind === "error") ? (
        <div className="mt-2 border border-error/50 bg-book-bg px-2 py-1 text-small text-error">
          存在缺失目标或未回收敛支线，导出前需要修正。
        </div>
      ) : null}
      <div className="mt-2 flex flex-col gap-2 border-t border-book-border/35 pt-2 sm:flex-row sm:items-center">
        <span
          className={`min-w-0 flex-1 text-small ${
            canSave && targetIssueCount === 0
              ? "text-book-text-muted"
              : "text-error"
          }`}
        >
          {validationMessage}
        </span>
        <div className="flex items-center justify-end gap-2">
          <BookButton
            variant="secondary"
            size="sm"
            onClick={addDraft}
            disabled={disabled || drafts.length >= 3 || scenes.length === 0}
          >
            <Plus size={13} />
            添加
          </BookButton>
          <BookButton
            variant="primary"
            size="sm"
            onClick={() => onUpdate(normalizedChoices)}
            disabled={disabled || !canSave}
          >
            保存选择支
          </BookButton>
        </div>
      </div>
    </div>
  );
};

type ChoiceTargetInspection =
  | { kind: "empty"; message: string }
  | { kind: "error"; message: string }
  | { kind: "main"; message: string }
  | { kind: "branch"; message: string };

const ChoiceTargetSummary: React.FC<{ inspection: ChoiceTargetInspection }> = ({
  inspection,
}) => {
  const className =
    inspection.kind === "error" ? "text-error" : "text-book-text-muted";
  return (
    <div className={`px-1 text-small ${className}`}>{inspection.message}</div>
  );
};

function inspectChoiceTarget(
  targetSceneId: string,
  sceneById: Map<number, GalgameSceneDetail>,
): ChoiceTargetInspection {
  if (!targetSceneId) {
    return { kind: "empty", message: "未选择目标场" };
  }
  const sceneId = Number(targetSceneId);
  if (!Number.isInteger(sceneId)) {
    return { kind: "error", message: "目标场无效" };
  }
  const target = sceneById.get(sceneId);
  if (!target) {
    return { kind: "error", message: "选择的目标场不存在" };
  }
  if (target.scene.entry_kind !== "branch") {
    return {
      kind: "main",
      message: `跳转到主线场次 ${target.scene.scene_index} · ${target.scene.title}`,
    };
  }
  const returnSceneId = target.scene.next_scene_id;
  if (returnSceneId == null) {
    return {
      kind: "error",
      message: `支线场次 ${target.scene.scene_index} · ${target.scene.title} 未设置回收敛场`,
    };
  }
  const returnScene = sceneById.get(returnSceneId);
  if (!returnScene) {
    return {
      kind: "error",
      message: `支线场次 ${target.scene.scene_index} 的回收敛目标不存在`,
    };
  }
  if (returnScene.scene.entry_kind !== "main") {
    return {
      kind: "error",
      message: `支线场次 ${target.scene.scene_index} 的回收敛目标不是主线场次 ${returnScene.scene.scene_index}`,
    };
  }
  return {
    kind: "branch",
    message: `进入支线场次 ${target.scene.scene_index} · ${target.scene.title}，回到场次 ${returnScene.scene.scene_index} · ${returnScene.scene.title}`,
  };
}

function buildChoiceDrafts(
  line: GalgameLine,
  choices: GalgameChoice[],
  scenes: GalgameSceneDetail[],
  sceneById: Map<number, GalgameSceneDetail>,
): ChoiceDraft[] {
  if (choices.length > 0) {
    return [...choices]
      .sort((left, right) => left.option_index - right.option_index)
      .map((choice) => ({
        option_text: choice.option_text,
        target_scene_id: choice.target_scene_id?.toString() ?? "",
      }));
  }
  const targetSceneId =
    defaultChoiceTargetSceneId(line, scenes, sceneById)?.toString() ?? "";
  return [
    { option_text: "", target_scene_id: targetSceneId },
    { option_text: "", target_scene_id: targetSceneId },
  ];
}

function defaultChoiceTargetSceneId(
  line: GalgameLine,
  scenes: GalgameSceneDetail[],
  sceneById: Map<number, GalgameSceneDetail>,
): number | null {
  const currentScene = sceneById.get(line.scene_id)?.scene ?? null;
  const nextSceneId = currentScene?.next_scene_id ?? null;
  if (nextSceneId != null && sceneById.has(nextSceneId)) {
    return nextSceneId;
  }
  return (
    scenes.find((item) => item.scene.id !== line.scene_id)?.scene.id ??
    scenes[0]?.scene.id ??
    null
  );
}

const LineMediaBindings: React.FC<{
  bindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  onImportAudio: (role: "bgm" | "sfx") => void;
  onBindMedia: (role: "cg" | "bgm" | "sfx", mediaAssetId: number) => void;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({ bindings, disabled, onImportAudio, onBindMedia, onUnbindMedia }) => {
  const audioBindings = bindings.filter(
    (item) => item.binding.role === "bgm" || item.binding.role === "sfx",
  );
  const cgBindings = bindings.filter((item) => item.binding.role === "cg");
  const bindingCount = audioBindings.length + cgBindings.length;
  return (
    <details className="mt-3 border-t border-book-border/40 pt-2.5">
      <summary className="cursor-pointer rounded-lg px-1 py-1 text-small font-medium text-book-text-main hover:bg-book-bg-tertiary/55">
        媒体与音效 · {bindingCount > 0 ? `${bindingCount} 项` : "未绑定"}
      </summary>
      <div className="mt-2.5">
        <ImageBindingStrip
          title="CG"
          emptyLabel=""
          bindings={cgBindings}
          disabled={disabled}
          bindControl={
            <MediaAssetIdBinder
              label="绑定 CG 资产"
              disabled={disabled}
              onBind={(mediaAssetId) => onBindMedia("cg", mediaAssetId)}
            />
          }
          onUnbindMedia={onUnbindMedia}
        />
        <div className="mb-2 divide-y divide-book-border/35 rounded-xl bg-book-bg-paper/45 lg:grid lg:grid-cols-2 lg:divide-x lg:divide-y-0">
          <AudioBindingControl
            label="BGM"
            disabled={disabled}
            onImport={() => onImportAudio("bgm")}
            onBind={(mediaAssetId) => onBindMedia("bgm", mediaAssetId)}
          />
          <AudioBindingControl
            label="SFX"
            disabled={disabled}
            onImport={() => onImportAudio("sfx")}
            onBind={(mediaAssetId) => onBindMedia("sfx", mediaAssetId)}
          />
        </div>
        {audioBindings.length ? (
          <div className="divide-y divide-book-border/35 rounded-xl bg-book-bg-paper/45 px-2.5">
            {audioBindings.map((item) => {
              const src = item.asset.file_path
                ? toMediaSrc(item.asset.file_path)
                : "";
              return (
                <div
                  key={item.binding.id}
                  className="grid gap-2 py-2.5 text-small text-book-text-muted md:grid-cols-[64px_minmax(0,1fr)_auto]"
                >
                  <span className="font-semibold text-book-text-main">
                    {item.binding.role === "bgm" ? "BGM" : "SFX"}
                  </span>
                  {src ? (
                    <audio src={src} controls className="h-8 w-full" />
                  ) : (
                    <span>音频文件缺失</span>
                  )}
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => onUnbindMedia(item)}
                    disabled={disabled}
                  >
                    <X size={13} />
                  </BookButton>
                </div>
              );
            })}
          </div>
        ) : null}
      </div>
    </details>
  );
};

const ImageBindingStrip: React.FC<{
  title: string;
  emptyLabel: string;
  bindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  bindControl?: React.ReactNode;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({
  title,
  emptyLabel,
  bindings,
  disabled,
  bindControl,
  onUnbindMedia,
}) => {
  if (!bindings.length) {
    return (
      <div className="mb-2 space-y-2">
        {emptyLabel ? (
          <div className="border border-dashed border-book-border bg-book-bg-paper px-3 py-4 text-center text-small text-book-text-muted">
            {emptyLabel}
          </div>
        ) : null}
        {bindControl}
      </div>
    );
  }
  return (
    <div className="mb-2 space-y-2">
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-1">
        {bindings.map((item) => {
          const mediaPath = item.asset.thumbnail_path || item.asset.file_path;
          const src = mediaPath ? toMediaSrc(mediaPath) : "";
          return (
            <div
              key={item.binding.id}
              className="overflow-hidden rounded-xl border border-book-border/45 bg-book-bg-paper"
            >
              <div className="flex items-center gap-2 px-2.5 py-2">
                <span className="min-w-0 flex-1 truncate text-small font-semibold text-book-text-main">
                  {title}
                </span>
                <button
                  type="button"
                  onClick={() => onUnbindMedia(item)}
                  disabled={disabled}
                  className="inline-flex h-7 w-7 items-center justify-center rounded-lg text-book-text-muted hover:bg-book-bg-tertiary hover:text-book-text-main disabled:opacity-45"
                  aria-label={`取消绑定${title}`}
                >
                  <X size={13} />
                </button>
              </div>
              <div className="aspect-video w-full overflow-hidden bg-book-bg">
                {src ? (
                  <img
                    src={src}
                    alt={title}
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <div className="flex h-full items-center justify-center text-small text-book-text-muted">
                    图片文件缺失
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
      {bindControl}
    </div>
  );
};

const AudioBindingControl: React.FC<{
  label: string;
  disabled: boolean;
  onImport: () => void;
  onBind: (mediaAssetId: number) => void;
}> = ({ label, disabled, onImport, onBind }) => {
  return (
    <section className="p-2.5">
      <div className="mb-2 text-small font-semibold text-book-text-main">
        {label}
      </div>
      <BookButton
        variant="secondary"
        size="sm"
        className="mb-2 w-full"
        onClick={onImport}
        disabled={disabled}
      >
        <Music size={13} />
        导入 {label}
      </BookButton>
      <MediaAssetIdBinder
        label={`绑定 ${label} 资产`}
        disabled={disabled}
        onBind={onBind}
      />
    </section>
  );
};

function pipelineStartStageLabel(
  stage: GalgamePipelineStartStage | "",
): string {
  return (
    PIPELINE_START_STAGES.find((item) => item.value === stage)?.label ??
    "从头开始"
  );
}

function sceneEntryKindLabel(entryKind: string): string {
  switch (entryKind) {
    case "main":
      return "主线";
    case "branch":
      return "支线";
    default:
      return "其他场次";
  }
}

function sceneSelectLabel(scene: GalgameScene): string {
  const entryKind = scene.entry_kind === "branch" ? "支线" : "主线";
  return `场次 ${scene.scene_index} · ${scene.title} · ${entryKind}`;
}

function assetPlanOutcomeLabel(outcome: string): string {
  switch (outcome) {
    case "created":
      return "新增";
    case "updated":
      return "更新";
    case "reused":
      return "复用";
    default:
      return "其他结果";
  }
}

function assetSpecStatusLabel(status: string): string {
  switch (status) {
    case "draft":
      return "待生成";
    case "completed":
      return "已完成";
    case "failed":
      return "失败";
    case "running":
      return "生成中";
    default:
      return "状态未知";
  }
}

function buildPortraitExpressionCells(
  items: GalgameAssetPlanItem[],
  speakerProfiles: GalgameSpeakerProfile[],
): PortraitExpressionCell[] {
  const speakerNameById = new Map(
    speakerProfiles.map((profile) => [profile.id, profile.name]),
  );
  return items.map((item) => {
    const params = requireRecordValue(
      item.params_json,
      `Galgame 资产规划 ${item.generation_spec_id} params_json`,
    );
    const profileId = item.target_entity_id;
    const profileName = speakerNameById.get(profileId);
    if (!profileName) {
      throw new Error(`Galgame 资产规划引用不存在的角色 ${profileId}`);
    }
    const expression = requireStringValue(
      params.expression,
      `Galgame 资产规划 ${item.generation_spec_id} expression`,
    );
    const pose = optionalStringValue(
      params.pose,
      `Galgame 资产规划 ${item.generation_spec_id} pose`,
    );
    return {
      rowKey: `${profileId}:${pose}`,
      profileId,
      profileName,
      pose,
      expression,
      item,
    };
  });
}

function buildPortraitExpressionMatrixModel(cells: PortraitExpressionCell[]): {
  rows: PortraitExpressionMatrixRow[];
  expressionValues: string[];
  cellsByKey: Map<string, PortraitExpressionCell>;
} {
  const rows: PortraitExpressionMatrixRow[] = [];
  const seenRowKeys = new Set<string>();
  const presentExpressions = new Set<string>();
  const cellsByKey = new Map<string, PortraitExpressionCell>();
  for (const cell of cells) {
    if (!seenRowKeys.has(cell.rowKey)) {
      seenRowKeys.add(cell.rowKey);
      rows.push({ rowKey: cell.rowKey, firstCell: cell });
    }
    presentExpressions.add(cell.expression);
    cellsByKey.set(`${cell.rowKey}:${cell.expression}`, cell);
  }
  return {
    rows,
    expressionValues: portraitExpressionColumns(presentExpressions),
    cellsByKey,
  };
}

function portraitExpressionColumns(present: Set<string>): string[] {
  const known = EXPRESSIONS.map((item) => item.value);
  const columns = known.filter((value) => present.has(value));
  for (const value of present) {
    if (!columns.includes(value)) columns.push(value);
  }
  return columns;
}

function portraitExpressionLabel(expression: string): string {
  return (
    EXPRESSIONS.find((item) => item.value === expression)?.label ?? "其他表情"
  );
}

function poseDisplayValue(pose: string): string {
  return POSE_SUGGESTIONS.find((item) => item.value === pose)?.label ?? pose;
}

function poseStoredValue(pose: string): string {
  const normalized = pose.trim();
  return (
    POSE_SUGGESTIONS.find((item) => item.label === normalized)?.value ??
    normalized
  );
}

function requireRecordValue(
  value: unknown,
  fieldName: string,
): Record<string, unknown> {
  if (value != null && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  throw new Error(`${fieldName} 必须是 JSON 对象`);
}

function requireStringValue(value: unknown, fieldName: string): string {
  if (typeof value === "string" && value.trim()) return value.trim();
  throw new Error(`${fieldName} 必须是非空字符串`);
}

function optionalStringValue(value: unknown, fieldName: string): string {
  if (value == null) return "";
  if (typeof value === "string") return value.trim();
  throw new Error(`${fieldName} 必须是字符串或 null`);
}

function emptyGalgamePresentation(): GalgameLinePresentationDocumentV2 {
  return {
    after_beat_ref: null,
    branch: null,
    branch_id: null,
    cg: null,
    cg_importance: null,
    cg_note: null,
    schema_version: "galgame_line_presentation_v2",
    visible_cast: [],
    enter: [],
    exit: [],
    focus_character_profile_id: null,
    hold: false,
    player_knowledge: [],
    returns_to_scene_index: null,
    transition: null,
  };
}

function presentationForLinePatch(
  line: GalgameLine,
  patch: Partial<GalgameLine>,
): GalgameLinePresentationDocumentV2 {
  const direct = patch.presentation_json ?? line.presentation_json;
  const presentation: GalgameLinePresentationDocumentV2 = {
    ...direct,
    visible_cast: direct.visible_cast.map((member) => ({ ...member })),
    enter: [...direct.enter],
    exit: [...direct.exit],
    player_knowledge: [...direct.player_knowledge],
  };
  const nextType = patch.line_type ?? line.line_type;
  if (!SPEAKING_LINE_TYPES.has(nextType)) return presentation;
  const speakerProfileId =
    "speaker_profile_id" in patch
      ? (patch.speaker_profile_id ?? null)
      : line.speaker_profile_id;
  if (speakerProfileId == null)
    throw new Error(`${nextType} 行必须绑定 speaker_profile_id`);
  const existing = presentation.visible_cast.find(
    (member) => member.character_profile_id === speakerProfileId,
  );
  const expression =
    ("expression" in patch
      ? patch.expression
      : (existing?.expression ?? line.expression)) || "neutral";
  const pose =
    ("pose" in patch ? patch.pose : (existing?.pose ?? line.pose)) ||
    "standing";
  const position = (("position" in patch
    ? patch.position
    : (existing?.position ?? line.position)) ||
    "center") as GalgameStagePositionV2;
  const speaker: GalgameVisibleCastMemberV2 = {
    character_profile_id: speakerProfileId,
    expression,
    pose,
    position,
    facing: existing?.facing ?? "front",
    layer: existing?.layer ?? "front",
  };
  presentation.visible_cast = [
    ...presentation.visible_cast.filter(
      (member) => member.character_profile_id !== speakerProfileId,
    ),
    speaker,
  ];
  if (presentation.focus_character_profile_id == null) {
    presentation.focus_character_profile_id = speakerProfileId;
  }
  return presentation;
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

interface PresentationPatch {
  cgEnabled?: boolean;
  cgNote?: string;
  cgImportance?: GalgameCgImportance | "";
  transition?: GalgameTransition | "";
}

function presentationCgNote(value: GalgameLinePresentationDocumentV2): string {
  return value.cg_note ?? "";
}

function updatePresentationJson(
  current: GalgameLinePresentationDocumentV2,
  patch: PresentationPatch,
): GalgameLinePresentationDocumentV2 {
  const next = { ...current };

  if (patch.cgEnabled === false) {
    clearEventCgPresentation(next);
  } else if (patch.cgEnabled === true) {
    if (presentationCgNote(next)) next.cg = true;
  }

  if (patch.cgNote != null) {
    const note = patch.cgNote.trim();
    if (note) {
      next.cg = true;
      next.cg_note = note;
    } else {
      clearEventCgPresentation(next);
    }
  }

  if (patch.cgImportance != null) {
    next.cg_importance = patch.cgImportance || null;
  }

  if (presentationCgNote(next)) {
    next.cg = true;
    next.cg_importance ??= DEFAULT_CG_IMPORTANCE;
  } else {
    clearEventCgPresentation(next);
  }

  if (patch.transition != null) {
    next.transition = patch.transition || null;
  }

  return next;
}

function clearEventCgPresentation(
  value: GalgameLinePresentationDocumentV2,
): void {
  value.cg = null;
  value.cg_note = null;
  value.cg_importance = null;
}

function selectValue(
  value: unknown,
  options: Array<{ value: string }>,
): string {
  const normalized = stringValue(value);
  return options.some((item) => item.value === normalized) ? normalized : "";
}
