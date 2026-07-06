import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  Download,
  FileText,
  FileSearch,
  Images,
  Loader2,
  Music,
  PenLine,
  Plus,
  RefreshCw,
  Trash2,
  X,
} from "lucide-react";
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
  type GalgameLineType,
  type GalgameAssetPlan,
  type GalgameAssetPlanItem,
  type GalgameMediaBindingSummary,
  type GalgamePipelineStatus,
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
import type { SourceSnapshotChapter } from "../../api/project";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";
import { toMediaSrc } from "../../utils/mediaSrc";
import { createProgressTaskId, useProgressTask } from "../../hooks/useProgressTask";
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

const POSITIONS = [
  { value: "", label: "无站位" },
  { value: "left", label: "左" },
  { value: "center", label: "中" },
  { value: "right", label: "右" },
];

const EXPRESSIONS = [
  { value: "", label: "无表情" },
  { value: "neutral", label: "neutral" },
  { value: "smile", label: "smile" },
  { value: "sad", label: "sad" },
  { value: "angry", label: "angry" },
  { value: "surprised", label: "surprised" },
  { value: "shy", label: "shy" },
  { value: "serious", label: "serious" },
];

const POSE_SUGGESTIONS = [
  "natural",
  "standing",
  "sitting",
  "walking",
  "turning",
  "close_up",
];

const DEFAULT_CG_IMPORTANCE = "normal";
const CG_IMPORTANCE_VALUES = ["normal", "high", "critical"];
const CG_IMPORTANCE_OPTIONS = [
  { value: "", label: "CG 优先级" },
  { value: "normal", label: "normal" },
  { value: "high", label: "high" },
  { value: "critical", label: "critical" },
];

const TRANSITION_OPTIONS = [
  { value: "", label: "无转场" },
  { value: "cut", label: "cut" },
  { value: "fade", label: "fade" },
  { value: "dissolve", label: "dissolve" },
];

const PIPELINE_START_STAGES: Array<{ value: GalgamePipelineStartStage | ""; label: string }> = [
  { value: "", label: "从头开始" },
  { value: "scene_segmentation", label: "场景切分" },
  { value: "script_adaptation", label: "剧本改编" },
  { value: "character_staging", label: "角色演出" },
  { value: "visual_planning", label: "视觉规划" },
  { value: "branch_design", label: "分支设计" },
  { value: "persist_script", label: "写入剧本" },
  { value: "asset_generation", label: "资产意图" },
];

const lineBusyTarget = (lineId: number): string => `line:${lineId}`;
const sceneBusyTarget = (sceneId: number): string => `scene:${sceneId}`;
const audioBusyTarget = (lineId: number, role: "bgm" | "sfx"): string => `audio:${lineId}:${role}`;
const bindMediaBusyTarget = (
  targetKind: "galgame_scene" | "galgame_line",
  targetId: number,
  role: "background" | "cg" | "bgm" | "sfx",
): string => `bind-media:${targetKind}:${targetId}:${role}`;
const insertBusyTarget = (sceneId: number, afterLineId: number | null): string =>
  `insert:${sceneId}:${afterLineId ?? "start"}`;

export const GalgameTab: React.FC<GalgameTabProps> = ({
  projectId,
  canEditGalgame,
  snapshotChapters,
}) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const [detail, setDetail] = useState<GalgameScriptDetail | null>(null);
  const [pipelineStatus, setPipelineStatus] = useState<GalgamePipelineStatus | null>(null);
  const [selectedSceneId, setSelectedSceneId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [pipelineCancelling, setPipelineCancelling] = useState(false);
  const [planningAssets, setPlanningAssets] = useState(false);
  const [generatingAssets, setGeneratingAssets] = useState(false);
  const [generatingAssetSpecId, setGeneratingAssetSpecId] = useState<number | null>(null);
  const [exportingWebgal, setExportingWebgal] = useState(false);
  const [assetPlan, setAssetPlan] = useState<GalgameAssetPlan | null>(null);
  const [busyTarget, setBusyTarget] = useState<string | null>(null);
  const [pipelineStartStage, setPipelineStartStage] = useState<GalgamePipelineStartStage | "">("");
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] = useState<ModelRequestPreview | null>(null);
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
        addToast(extractErrorMessage(statusResult.reason, "加载 Galgame 管线状态失败"), "error");
      }
      if (scriptResult.status === "fulfilled") {
        setDetail(scriptResult.value);
        setSelectedSceneId((current) => current ?? scriptResult.value.scenes[0]?.scene.id ?? null);
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
      (selectedSceneId != null ? sceneById.get(selectedSceneId) ?? null : null) ??
      detail?.scenes[0] ??
      null,
    [detail?.scenes, sceneById, selectedSceneId],
  );

  const chapterBySnapshotId = useMemo(() => {
    const map = new Map<number, SourceSnapshotChapter>();
    for (const chapter of snapshotChapters) map.set(chapter.id, chapter);
    return map;
  }, [snapshotChapters]);

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

  const pipelineRunning = generating || pipelineStatus?.run?.status === "running";

  const handleGenerate = async (resumeFromCheckpoint = false) => {
    if (!canEditGalgame || generating) return;
    const taskId = createProgressTaskId("galgame-pipeline", projectId);
    setGenerating(true);
    try {
      const next = await runProgressTask(
        {
          title: "Galgame 管线",
          startMessage: "正在准备 Galgame 管线",
          successMessage: "Galgame 管线已完成",
          errorMessage: "运行 Galgame 管线失败",
          taskId,
          total: 7,
          cancelLabel: "取消管线",
          runningHint: "取消后已写入的检查点会保留，可从检查点续跑。",
          cancelTask: async () => {
            const nextStatus = await cancelGalgamePipeline(projectId);
            setPipelineStatus(nextStatus);
          },
        },
        async ({ setProgress }) => {
          setProgress("正在提交 Galgame 管线...", 0, 7);
          const result = await startGalgamePipeline(projectId, {
            taskId,
            startFromStage: currentPipelineStartStage(),
            resumeFromCheckpoint,
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
      addToast(cancelled ? "Galgame 管线已取消" : message, cancelled ? "info" : "error");
      const nextStatus = await getGalgamePipelineStatus(projectId).catch(() => pipelineStatus);
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
      const preview = await previewGalgamePipelineRequest(projectId, {
        startFromStage: currentPipelineStartStage(),
        resumeFromCheckpoint: false,
      });
      if (previewSeqRef.current !== seq) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (previewSeqRef.current === seq) {
        addToast(extractErrorMessage(err, "预览 Galgame 管线请求失败"), "error");
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
      const result = await runGalgameAssetGeneration(projectId);
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
    if (!canEditGalgame || generatingAssets || generatingAssetSpecId != null || !detail) return;
    setGeneratingAssetSpecId(generationSpecId);
    try {
      const result = await runGalgameAssetGeneration(projectId, {
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

  const handleLinePatch = async (line: GalgameLine, patch: Partial<GalgameLine>) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(lineBusyTarget(line.id));
    try {
      const input: UpdateGalgameLineInput = { line_id: line.id };
      if ("line_type" in patch && patch.line_type != null) input.line_type = patch.line_type;
      if ("speaker_profile_id" in patch) {
        input.speaker_profile_id = patch.speaker_profile_id ?? null;
      }
      if ("text" in patch && patch.text != null) input.text = patch.text;
      if ("position" in patch) input.position = patch.position ?? null;
      if ("expression" in patch) input.expression = patch.expression ?? null;
      if ("pose" in patch) input.pose = patch.pose ?? null;
      if ("presentation_json" in patch) {
        input.presentation_json = requireRecordValue(patch.presentation_json, "presentation_json");
      }
      await updateGalgameLine(projectId, input);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, "保存剧本行失败"), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleScenePatch = async (scene: GalgameScene, patch: Partial<GalgameScene>) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(sceneBusyTarget(scene.id));
    try {
      const input: UpdateGalgameSceneInput = { scene_id: scene.id };
      if ("title" in patch && patch.title != null) input.title = patch.title;
      if ("location" in patch) input.location = patch.location ?? null;
      if ("time_of_day" in patch) input.time_of_day = patch.time_of_day ?? null;
      if ("background_note" in patch) input.background_note = patch.background_note ?? null;
      if ("next_scene_id" in patch) input.next_scene_id = patch.next_scene_id ?? null;
      await updateGalgameScene(projectId, input);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, "保存场信息失败"), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleAddLine = async (scene: GalgameSceneDetail, afterLine: GalgameLine | null) => {
    if (!canEditGalgame || busyTarget != null) return;
    setBusyTarget(insertBusyTarget(scene.scene.id, afterLine?.id ?? null));
    try {
      await insertGalgameLine(projectId, {
        scene_id: scene.scene.id,
        after_line_id: afterLine?.id ?? null,
        line_type: "narration",
        speaker_profile_id: null,
        text: "新的剧本行",
        expression: null,
        pose: null,
        position: null,
        presentation_json: {},
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
      await deleteGalgameLine(projectId, line.id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, "删除剧本行失败"), "error");
    } finally {
      setBusyTarget(null);
    }
  };

  const handleMoveLine = async (scene: GalgameSceneDetail, line: GalgameLine, direction: -1 | 1) => {
    if (!canEditGalgame || busyTarget != null) return;
    const index = scene.lines.findIndex((item) => item.id === line.id);
    const nextIndex = index + direction;
    if (index < 0 || nextIndex < 0 || nextIndex >= scene.lines.length) return;
    const lineIds = scene.lines.map((item) => item.id);
    [lineIds[index], lineIds[nextIndex]] = [lineIds[nextIndex], lineIds[index]];
    setBusyTarget(lineBusyTarget(line.id));
    try {
      await reorderGalgameLines(projectId, scene.scene.id, lineIds);
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
    const file = await pickLocalFile("audio/mpeg,audio/wav,audio/ogg,.mp3,.wav,.ogg");
    if (!file) return;
    setBusyTarget(audioBusyTarget(line.id, role));
    try {
      await importGalgameAudio({
        project_id: projectId,
        target_line_id: line.id,
        role,
        file_name: file.name,
        mime_type: file.type || fallbackAudioMime(file.name),
        data_base64: await readFileAsBase64(file),
      });
      await load();
      addToast(role === "bgm" ? "BGM 已导入并绑定" : "音效已导入并绑定", "success");
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

  const totalLines = detail?.scenes.reduce((sum, scene) => sum + scene.lines.length, 0) ?? 0;

  return (
    <div className="grid min-h-[60vh] grid-cols-1 gap-4 lg:min-h-[640px] lg:grid-cols-[280px_minmax(0,1fr)]">
      <aside className="min-h-0 border border-book-border bg-book-surface p-3">
        <div className="mb-3 flex items-center justify-between gap-2">
          <div className="min-w-0">
            <h2 className="truncate text-sm font-semibold text-book-text-main">剧本场次</h2>
            <p className="text-[11px] text-book-text-muted">
              {detail ? `${detail.scenes.length} 场 / ${totalLines} 行` : "尚未生成"}
            </p>
          </div>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={load}
            disabled={loading || generating || pipelineCancelling}
          >
            <RefreshCw size={13} />
          </BookButton>
        </div>
          <BookButton
            variant="primary"
            size="sm"
            className="mb-3 w-full"
            onClick={() => handleGenerate(false)}
            disabled={!canEditGalgame || generating || pipelineCancelling}
          >
            {generating ? <Loader2 size={14} className="animate-spin" /> : <FileText size={14} />}
            {detail ? "重新运行管线" : "运行 Galgame 管线"}
          </BookButton>
          {pipelineRunning ? (
            <BookButton
              variant="secondary"
              size="sm"
              className="mb-3 w-full"
              onClick={handleCancelPipeline}
              disabled={!canEditGalgame || pipelineCancelling}
            >
              {pipelineCancelling ? <Loader2 size={13} className="animate-spin" /> : <X size={13} />}
              取消管线
            </BookButton>
          ) : null}
          <select
            className="mb-2 w-full rounded-none border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
            value={pipelineStartStage}
            disabled={!canEditGalgame || generating || pipelineCancelling}
            onChange={(event) =>
              setPipelineStartStage(event.target.value as GalgamePipelineStartStage | "")
            }
          >
            {PIPELINE_START_STAGES.map((stage) => (
              <option key={stage.value || "all"} value={stage.value}>
                {stage.label}
              </option>
            ))}
          </select>
          <div className="mb-3 grid grid-cols-1 gap-2 sm:grid-cols-2">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={handlePreviewPipeline}
              disabled={!canEditGalgame || requestPreviewLoading || generating || pipelineCancelling}
            >
              {requestPreviewLoading ? (
                <Loader2 size={13} className="animate-spin" />
              ) : (
                <FileSearch size={13} />
              )}
              预览请求
            </BookButton>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => handleGenerate(true)}
              disabled={!canEditGalgame || generating || pipelineCancelling || !pipelineStatus?.run?.can_resume}
            >
              {generating ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
              续跑
            </BookButton>
          </div>
        <PipelineStatusPanel status={pipelineStatus} loading={loading || generating} />
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
        <ExportPanel
          loading={exportingWebgal}
          disabled={!canEditGalgame || exportingWebgal || !detail}
          onExport={handleExportWebgal}
        />
        <div className="space-y-2">
          {detail?.scenes.map((item) => {
            const chapter = chapterBySnapshotId.get(item.scene.source_snapshot_chapter_id);
            const active = item.scene.id === selectedScene?.scene.id;
            const nextScene =
              item.scene.next_scene_id != null
                ? sceneById.get(item.scene.next_scene_id) ?? null
                : null;
            return (
              <button
                key={item.scene.id}
                type="button"
                className={`w-full border px-3 py-2 text-left text-xs ${
                  active
                    ? "border-book-primary bg-book-primary/10 text-book-text-main"
                    : "border-book-border bg-book-bg text-book-text-muted hover:text-book-text-main"
                }`}
                onClick={() => setSelectedSceneId(item.scene.id)}
              >
                <div className="truncate font-semibold">
                  {item.scene.scene_index}. {item.scene.title}
                </div>
                <div className="mt-1 truncate text-[11px]">
                  {chapter ? `第 ${chapter.chapter_number} 章` : "源快照章节"}
                  {" · "}
                  {item.lines.length} 行
                </div>
                <div className="mt-2 flex flex-wrap items-center gap-1 text-[10px]">
                  <span className="border border-book-border bg-book-surface px-1.5 py-0.5">
                    {sceneEntryKindLabel(item.scene.entry_kind)}
                  </span>
                  {item.scene.next_scene_id != null ? (
                    <span
                      className={`border px-1.5 py-0.5 ${
                        nextScene && !(item.scene.entry_kind === "branch" && nextScene.scene.entry_kind !== "main")
                          ? "border-book-border bg-book-surface text-book-text-muted"
                          : "border-error/50 bg-book-surface text-error"
                      }`}
                    >
                      {nextScene
                        ? item.scene.entry_kind === "branch" && nextScene.scene.entry_kind !== "main"
                          ? `→ 非主线 #${nextScene.scene.scene_index}`
                          : `→ #${nextScene.scene.scene_index}`
                        : `→ 缺失 #${item.scene.next_scene_id}`}
                    </span>
                  ) : item.scene.entry_kind === "branch" ? (
                    <span className="border border-error/50 bg-book-surface px-1.5 py-0.5 text-error">
                      未回收敛
                    </span>
                  ) : null}
                </div>
              </button>
            );
          })}
        </div>
      </aside>

      <section className="min-h-0 border border-book-border bg-book-surface">
        <div className="flex items-center justify-between gap-3 border-b border-book-border px-4 py-3">
          <div className="min-w-0">
            <h2 className="truncate text-sm font-semibold text-book-text-main">
              {selectedScene?.scene.title ?? "Galgame 剧本"}
            </h2>
            <p className="truncate text-[11px] text-book-text-muted">
              {selectedScene
                ? selectedScene.scene.background_note || "Galgame 剧本"
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
            >
              <Plus size={14} />
              行
            </BookButton>
          ) : null}
        </div>

        {selectedScene ? (
          <SceneMetaEditor
            scene={selectedScene.scene}
            scenes={detail?.scenes ?? []}
                        mediaBindings={mediaBindingsByScene.get(selectedScene.scene.id) ?? []}
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

        <div className="min-h-[60vh] overflow-y-auto p-3 lg:max-h-[calc(100vh-250px)] lg:min-h-[520px] lg:p-4 custom-scrollbar">
          {loading ? (
            <div className="inline-flex items-center gap-2 text-xs text-book-text-muted">
              <Loader2 size={14} className="animate-spin" />
              加载 Galgame 剧本
            </div>
          ) : !selectedScene ? (
            <div className="flex h-64 flex-col items-center justify-center gap-3 text-xs text-book-text-muted">
              <PenLine size={22} />
              <span>尚未生成剧本</span>
            </div>
          ) : (
            <div className="space-y-3">
              {selectedScene.lines.map((line, index) => (
                <LineEditor
                  key={line.id}
                  line={line}
                  speakerProfiles={detail?.speaker_profiles ?? []}
                  sourceAnchor={
                    line.text_anchor_id ? textAnchorById.get(line.text_anchor_id) ?? null : null
                  }
                  sourceChapter={
                    line.text_anchor_id
                      ? chapterBySnapshotId.get(
                          textAnchorById.get(line.text_anchor_id)?.source_snapshot_chapter_id ?? -1,
                        ) ?? null
                      : null
                  }
                  disabled={!canEditGalgame || busyTarget != null}
                  saving={busyTarget === lineBusyTarget(line.id)}
                  mediaBindings={mediaBindingsByLine.get(line.id) ?? []}
                  choices={choicesByLine.get(line.id) ?? []}
                  scenes={detail?.scenes ?? []}
                  sceneById={sceneById}
                  onPatch={(patch) => handleLinePatch(line, patch)}
                  onChoicesUpdate={(choices) => handleChoicesUpdate(line, choices)}
                  onMoveUp={() => handleMoveLine(selectedScene, line, -1)}
                  onMoveDown={() => handleMoveLine(selectedScene, line, 1)}
                  canMoveUp={index > 0}
                  canMoveDown={index < selectedScene.lines.length - 1}
                  onInsertAfter={() => handleAddLine(selectedScene, line)}
                            onDelete={() => handleDeleteLine(line)}
                            onImportAudio={(role) => handleImportAudio(line, role)}
                            onBindMedia={(role, mediaAssetId) =>
                              handleBindMedia("galgame_line", line.id, role, mediaAssetId)
                            }
                            onUnbindMedia={handleUnbindMedia}
                          />
              ))}
            </div>
          )}
        </div>
      </section>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="Galgame 管线请求预览"
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

const PipelineStatusPanel: React.FC<{
  status: GalgamePipelineStatus | null;
  loading: boolean;
}> = ({ status, loading }) => {
  const run = status?.run ?? null;

  return (
    <div className="mb-3 border border-book-border bg-book-bg px-3 py-2 text-xs">
      <div className="mb-2 flex items-center justify-between gap-2">
        <span className="font-semibold text-book-text-main">管线状态</span>
        {loading ? <Loader2 size={13} className="animate-spin text-book-text-muted" /> : null}
      </div>
      {!run ? (
        <div className="text-[11px] text-book-text-muted">尚未运行</div>
      ) : (
        <div className="space-y-1 text-[11px] text-book-text-muted">
          <div className="flex items-center justify-between gap-2">
            <span>{pipelineStatusLabel(run.status)}</span>
            <span className="truncate">{formatPipelineTime(run.updated_at)}</span>
          </div>
          {run.current_step_key ? (
            <div className="truncate">步骤：{pipelineStepLabel(run.current_step_key)}</div>
          ) : null}
          {run.error_message ? <div className="text-error">{run.error_message}</div> : null}
          {status?.checkpoints.length ? (
            <div className="mt-2 space-y-1 border-t border-book-border pt-2">
              {status.checkpoints.map((checkpoint) => (
                <div key={checkpoint.id} className="flex items-center justify-between gap-2">
                  <span className="truncate">{pipelineStepLabel(checkpoint.step_key)}</span>
                  <span>{pipelineStatusLabel(checkpoint.status)}</span>
                </div>
              ))}
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
};

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
  return (
    <div className="mb-3 border border-book-border bg-book-bg px-3 py-2 text-xs">
      <div className="mb-2 flex items-center justify-between gap-2">
        <span className="font-semibold text-book-text-main">资产规划</span>
        {loading || generating ? (
          <Loader2 size={13} className="animate-spin text-book-text-muted" />
        ) : null}
      </div>
      <BookButton variant="secondary" size="sm" className="mb-2 w-full" onClick={onPlan} disabled={disabled}>
        {loading ? <Loader2 size={13} className="animate-spin" /> : <Images size={13} />}
        写入资产生成意图
      </BookButton>
      <BookButton
        variant="secondary"
        size="sm"
        className="mb-2 w-full"
        onClick={onGenerate}
        disabled={disabled}
      >
        {generating ? <Loader2 size={13} className="animate-spin" /> : <Images size={13} />}
        生成视觉资产
      </BookButton>
      {!plan ? (
        <div className="text-[11px] text-book-text-muted">尚未规划</div>
      ) : (
        <div className="space-y-2 text-[11px] text-book-text-muted">
          <div className="grid grid-cols-3 gap-1 text-center">
            <AssetPlanMetric label="背景" value={plan.summary.background_specs} />
            <AssetPlanMetric label="差分" value={plan.summary.portrait_expression_specs} />
            <AssetPlanMetric label="CG" value={plan.summary.event_cg_specs} />
          </div>
          <div className="grid grid-cols-3 gap-1 text-center">
            <AssetPlanMetric label="新增" value={plan.summary.created_specs} />
            <AssetPlanMetric label="更新" value={plan.summary.updated_specs} />
            <AssetPlanMetric label="复用" value={plan.summary.reused_specs} />
          </div>
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
  );
};

const ExportPanel: React.FC<{
  loading: boolean;
  disabled: boolean;
  onExport: () => void;
}> = ({ loading, disabled, onExport }) => (
  <div className="mb-3 border border-book-border bg-book-bg px-3 py-2 text-xs">
    <div className="mb-2 flex items-center justify-between gap-2">
      <span className="font-semibold text-book-text-main">导出</span>
      {loading ? <Loader2 size={13} className="animate-spin text-book-text-muted" /> : null}
    </div>
    <BookButton
      variant="secondary"
      size="sm"
      className="w-full"
      onClick={onExport}
      disabled={disabled}
    >
      {loading ? <Loader2 size={13} className="animate-spin" /> : <Download size={13} />}
      WebGAL zip
    </BookButton>
  </div>
);

const AssetPlanMetric: React.FC<{ label: string; value: number }> = ({ label, value }) => (
  <div className="border border-book-border bg-book-surface px-1 py-1">
    <div className="font-semibold text-book-text-main">{value}</div>
    <div>{label}</div>
  </div>
);

const AssetPlanList: React.FC<{ title: string; items: GalgameAssetPlan["backgrounds"] }> = ({
  title,
  items,
}) => {
  if (!items.length) return null;
  return (
    <div className="border-t border-book-border pt-2">
      <div className="mb-1 font-semibold text-book-text-main">{title}</div>
      <div className="space-y-1">
        {items.slice(0, 4).map((item) => (
          <div key={item.canonical_key} className="flex items-center justify-between gap-2">
            <span className="truncate" title={item.label}>
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
}> = ({ items, speakerProfiles, disabled, generatingSpecId, onGenerateSpec }) => {
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
      <div className="mb-1 flex items-center justify-between gap-2">
        <span className="font-semibold text-book-text-main">角色差分</span>
        <span>{cells.length} 项</span>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[360px] border-collapse text-[11px]">
          <thead>
            <tr className="text-left text-book-text-muted">
              <th className="border border-book-border bg-book-surface px-2 py-1 font-medium">
                角色
              </th>
              {expressionValues.map((expression) => (
                <th
                  key={expression || "none"}
                  className="border border-book-border bg-book-surface px-2 py-1 font-medium"
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
                  <th className="max-w-28 border border-book-border bg-book-surface px-2 py-1 text-left font-medium text-book-text-main">
                    <div className="truncate" title={firstCell.profileName}>
                      {firstCell.profileName}
                    </div>
                    <div className="truncate text-book-text-muted" title={firstCell.pose || "默认姿势"}>
                      {firstCell.pose || "默认姿势"}
                    </div>
                  </th>
                  {expressionValues.map((expression) => {
                    const cell = cellsByKey.get(`${rowKey}:${expression}`);
                    const generating = generatingSpecId === cell?.item.generation_spec_id;
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
                              onClick={() => onGenerateSpec(cell.item.generation_spec_id)}
                            >
                              {generating ? (
                                <Loader2 size={12} className="animate-spin" />
                              ) : (
                                <RefreshCw size={12} />
                              )}
                              {cell.item.status === "completed" ? "检查" : "生成"}
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
}> = ({ scene, scenes, mediaBindings, disabled, onPatch, onBindMedia, onUnbindMedia }) => {
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
  }, [scene.background_note, scene.location, scene.next_scene_id, scene.time_of_day, scene.title]);

  const commit = (
    key: "title" | "location" | "time_of_day" | "background_note",
    value: string,
  ) => {
    const current =
      key === "title"
        ? scene.title
        : key === "location"
          ? scene.location ?? ""
          : key === "time_of_day"
            ? scene.time_of_day ?? ""
            : scene.background_note ?? "";
    if (value === current) return;
    onPatch({ [key]: key === "title" ? value : value || null });
  };
  const backgroundBindings = mediaBindings.filter((item) => item.binding.role === "background");

  return (
    <div className="grid gap-3 border-b border-book-border bg-book-bg px-4 py-3 text-xs lg:grid-cols-[minmax(0,1fr)_220px]">
      <div className="grid gap-2 md:grid-cols-[1.2fr_0.8fr_0.8fr]">
        <input
          className="border border-book-border bg-book-surface px-2 py-1 text-book-text-main outline-none focus:border-book-primary"
          value={draft.title}
          disabled={disabled}
          onChange={(event) => setDraft((current) => ({ ...current, title: event.target.value }))}
          onBlur={() => commit("title", draft.title)}
        />
        <input
          className="border border-book-border bg-book-surface px-2 py-1 text-book-text-main outline-none focus:border-book-primary"
          value={draft.location}
          disabled={disabled}
          placeholder="地点"
          onChange={(event) =>
            setDraft((current) => ({ ...current, location: event.target.value }))
          }
          onBlur={() => commit("location", draft.location)}
        />
        <input
          className="border border-book-border bg-book-surface px-2 py-1 text-book-text-main outline-none focus:border-book-primary"
          value={draft.time_of_day}
          disabled={disabled}
          placeholder="时间"
          onChange={(event) =>
            setDraft((current) => ({ ...current, time_of_day: event.target.value }))
          }
          onBlur={() => commit("time_of_day", draft.time_of_day)}
        />
        <textarea
          className="min-h-16 border border-book-border bg-book-surface px-2 py-1 text-book-text-main outline-none focus:border-book-primary md:col-span-3"
          value={draft.background_note}
          disabled={disabled}
          placeholder="背景需求"
          onChange={(event) =>
            setDraft((current) => ({ ...current, background_note: event.target.value }))
          }
          onBlur={() => commit("background_note", draft.background_note)}
        />
        <select
          className="rounded-none border border-book-border bg-book-surface px-2 py-1 text-book-text-main outline-none focus:border-book-primary md:col-span-3"
          value={draft.next_scene_id}
          disabled={disabled}
          onChange={(event) => {
            const value = event.target.value;
            setDraft((current) => ({ ...current, next_scene_id: value }));
            onPatch({ next_scene_id: value ? Number(value) : null });
          }}
        >
          <option value="">{scene.entry_kind === "branch" ? "未回收敛" : "无下一场"}</option>
          {!selectedNextSceneSelectable && scene.next_scene_id != null ? (
            <option value={scene.next_scene_id}>
              {scenes.some((item) => item.scene.id === scene.next_scene_id)
                ? `非主线场 #${scene.next_scene_id}`
                : `缺失场 #${scene.next_scene_id}`}
            </option>
          ) : null}
          {availableNextScenes.map((item) => (
            <option key={item.scene.id} value={item.scene.id}>
              {sceneSelectLabel(item.scene)}
            </option>
          ))}
        </select>
      </div>
      <ImageBindingStrip
        title="背景"
        emptyLabel="未绑定背景"
        bindings={backgroundBindings}
        disabled={disabled}
        bindControl={
          <MediaAssetIdBinder
            label="绑定背景资产"
            disabled={disabled}
            onBind={onBindMedia}
          />
        }
        onUnbindMedia={onUnbindMedia}
      />
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
  const [pose, setPose] = useState(line.pose ?? "");
  const isSpeakingLine = SPEAKING_LINE_TYPES.has(line.line_type);
  const isChoiceLine = line.line_type === "choice";
  const expressionValue = selectValue(line.expression, EXPRESSIONS);
  const missingSpeakingExpression = isSpeakingLine && expressionValue.length === 0;
  const poseDatalistId = `galgame-pose-suggestions-${line.id}`;
  const speaker = useMemo(
    () => speakerProfiles.find((profile) => profile.id === line.speaker_profile_id) ?? null,
    [line.speaker_profile_id, speakerProfiles],
  );

  useEffect(() => {
    setText(line.text);
    setPose(line.pose ?? "");
  }, [line.pose, line.text]);

  const firstSpeakerId = speakerProfiles[0]?.id ?? null;

  return (
    <div className="border border-book-border bg-book-bg px-3 py-3">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <span className="min-w-8 text-[11px] font-semibold text-book-text-muted">
          #{line.line_index}
        </span>
        <select
          className="rounded-none border border-book-border bg-book-surface px-2 py-1 text-xs text-book-text-main"
          value={line.line_type}
          disabled={disabled}
          onChange={(event) => {
            const nextType = event.target.value as GalgameLineType;
            onPatch({
              line_type: nextType,
              speaker_profile_id: SPEAKING_LINE_TYPES.has(nextType)
                ? line.speaker_profile_id ?? firstSpeakerId
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
              disabled={SPEAKING_LINE_TYPES.has(item.value) && speakerProfiles.length === 0}
            >
              {item.label}
            </option>
          ))}
        </select>
        {isSpeakingLine ? (
          <select
            className="rounded-none border border-book-border bg-book-surface px-2 py-1 text-xs text-book-text-main"
            value={line.speaker_profile_id ?? ""}
            disabled={disabled || speakerProfiles.length === 0}
            onChange={(event) =>
              onPatch({
                speaker_profile_id: event.target.value ? Number(event.target.value) : null,
                expression: event.target.value ? expressionValue || "neutral" : line.expression,
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
            className="rounded-none border border-book-border bg-book-surface px-2 py-1 text-xs text-book-text-main"
            value={line.position ?? ""}
            disabled={disabled}
            onChange={(event) => onPatch({ position: event.target.value || null })}
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
            className={`max-w-[180px] truncate border px-2 py-1 text-[11px] ${
              speaker && !missingSpeakingExpression
                ? "border-book-border bg-book-surface text-book-text-muted"
                : "border-error/50 bg-book-surface text-error"
            }`}
            title={
              speaker
                ? missingSpeakingExpression
                  ? "对白/独白必须设置 expression"
                  : `profile_id=${speaker.id}`
                : "未绑定 speaker_profile_id"
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
            className={`max-w-[220px] truncate text-[11px] ${
              sourceAnchor ? "text-book-text-muted" : "text-error"
            }`}
            title={
              sourceAnchor
                ? `${sourceChapter ? `第 ${sourceChapter.chapter_number} 章` : "源快照章节"} · P${
                    sourceAnchor.paragraph_index + 1
                  } · ${sourceAnchor.quote}`
                : `text_anchor ${line.text_anchor_id} 未随剧本返回`
            }
          >
            {sourceAnchor
              ? `${sourceChapter ? `第 ${sourceChapter.chapter_number} 章` : "源快照"} · ${
                  sourceAnchor.quote
                }`
              : `锚点缺失 ${line.text_anchor_id}`}
          </span>
        ) : null}
        <div className="ml-auto flex items-center gap-1">
          {saving ? <Loader2 size={13} className="animate-spin text-book-text-muted" /> : null}
          <BookButton variant="secondary" size="sm" onClick={onMoveUp} disabled={disabled || !canMoveUp}>
            <ArrowUp size={13} />
          </BookButton>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={onMoveDown}
            disabled={disabled || !canMoveDown}
          >
            <ArrowDown size={13} />
          </BookButton>
          <BookButton variant="secondary" size="sm" onClick={onInsertAfter} disabled={disabled}>
            <Plus size={13} />
          </BookButton>
          <BookButton variant="secondary" size="sm" onClick={onDelete} disabled={disabled}>
            <Trash2 size={13} />
          </BookButton>
        </div>
      </div>
      <textarea
        className="min-h-20 w-full resize-y border border-book-border bg-book-surface px-3 py-2 text-xs leading-relaxed text-book-text-main outline-none focus:border-book-primary"
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
            className="rounded-none border border-book-border bg-book-surface px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
            value={expressionValue}
            disabled={disabled}
            onChange={(event) => onPatch({ expression: event.target.value || null })}
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
            className="border border-book-border bg-book-surface px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
            value={pose}
            disabled={disabled}
            placeholder="姿势"
            onChange={(event) => setPose(event.target.value)}
            onBlur={() => {
              if (pose !== (line.pose ?? "")) onPatch({ pose: pose || null });
            }}
          />
          <datalist id={poseDatalistId}>
            {POSE_SUGGESTIONS.map((item) => (
              <option key={item} value={item} />
            ))}
          </datalist>
          <PresentationEditor
            line={line}
            disabled={disabled}
            onPatch={(presentationJson) => onPatch({ presentation_json: presentationJson })}
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
    </div>
  );
};

const PresentationEditor: React.FC<{
  line: GalgameLine;
  disabled: boolean;
  onPatch: (presentationJson: Record<string, unknown>) => void;
}> = ({ line, disabled, onPatch }) => {
  const presentation = useMemo(() => recordValue(line.presentation_json), [line.presentation_json]);
  const currentCgEnabled = presentation["cg"] === true || presentationCgNote(presentation).length > 0;
  const currentCgImportance =
    selectValue(presentation["cg_importance"], CG_IMPORTANCE_OPTIONS) ||
    (currentCgEnabled ? DEFAULT_CG_IMPORTANCE : "");
  const currentTransition = selectValue(presentation["transition"], TRANSITION_OPTIONS);
  const [cgNote, setCgNote] = useState(presentationCgNote(presentation));
  const [cgEnabled, setCgEnabled] = useState(currentCgEnabled);
  const [cgImportance, setCgImportance] = useState(currentCgImportance);
  const [transition, setTransition] = useState(currentTransition);

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
    <div className="md:col-span-2 border border-book-border bg-book-surface px-2 py-2">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-1 text-[11px] text-book-text-main">
          <input
            type="checkbox"
            checked={cgEnabled}
            disabled={disabled}
            onChange={(event) => {
              const checked = event.target.checked;
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
          />
          事件 CG
        </label>
        <select
          className="rounded-none border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
          value={cgImportance}
          disabled={disabled || !cgEnabled}
          onChange={(event) => {
            setCgImportance(event.target.value);
            update({ cgImportance: event.target.value });
          }}
        >
          {CG_IMPORTANCE_OPTIONS.map((item) => (
            <option key={item.value || "none"} value={item.value}>
              {item.label}
            </option>
          ))}
        </select>
        <select
          className="rounded-none border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
          value={transition}
          disabled={disabled}
          onChange={(event) => {
            setTransition(event.target.value);
            update({ transition: event.target.value });
          }}
        >
          {TRANSITION_OPTIONS.map((item) => (
            <option key={item.value || "none"} value={item.value}>
              {item.label}
            </option>
          ))}
        </select>
      </div>
      <textarea
        className="min-h-16 w-full resize-y border border-book-border bg-book-bg px-2 py-1 text-xs leading-relaxed text-book-text-main outline-none focus:border-book-primary"
        value={cgNote}
        disabled={disabled}
        placeholder="事件 CG 画面描述"
        onChange={(event) => setCgNote(event.target.value)}
        onBlur={() => {
          if (cgNote !== presentationCgNote(presentation)) {
            if (cgNote.trim()) {
              setCgEnabled(true);
              if (!cgImportance) setCgImportance(DEFAULT_CG_IMPORTANCE);
              update({ cgNote, cgImportance: cgImportance || DEFAULT_CG_IMPORTANCE });
            } else {
              update({ cgNote: "" });
            }
          }
        }}
      />
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
    () => drafts.map((draft) => inspectChoiceTarget(draft.target_scene_id, sceneById)),
    [drafts, sceneById],
  );
  const normalizedChoices = useMemo(() => {
    return drafts
      .map((draft) => {
        const optionText = draft.option_text.trim();
        const targetSceneId = Number(draft.target_scene_id);
        if (!optionText || !Number.isInteger(targetSceneId) || !sceneById.has(targetSceneId)) {
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
    drafts.length >= 2 && drafts.length <= 3 && normalizedChoices.length === drafts.length;
  const targetIssueCount = targetInspections.filter((item) => item.kind === "error").length;
  const canSave =
    structurallyComplete && targetIssueCount === 0;
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
      current.map((draft, itemIndex) => (itemIndex === index ? { ...draft, ...patch } : draft)),
    );
  };

  const addDraft = () => {
    if (drafts.length >= 3) return;
    setDrafts((current) => [
      ...current,
      {
        option_text: "",
        target_scene_id: defaultChoiceTargetSceneId(line, scenes, sceneById)?.toString() ?? "",
      },
    ]);
  };

  const removeDraft = (index: number) => {
    if (drafts.length <= 2) return;
    setDrafts((current) => current.filter((_, itemIndex) => itemIndex !== index));
  };

  return (
    <div className="mt-2 border border-book-border bg-book-surface p-2">
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
                  onChange={(event) => updateDraft(index, { option_text: event.target.value })}
                />
                <select
                  className="rounded-none border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main"
                  value={draft.target_scene_id}
                  disabled={disabled || scenes.length === 0}
                  onChange={(event) => updateDraft(index, { target_scene_id: event.target.value })}
                >
                  {scenes.length === 0 ? <option value="">无目标场</option> : null}
                  {scenes.map((item) => (
                    <option key={item.scene.id} value={item.scene.id}>
                      {sceneSelectLabel(item.scene)}
                    </option>
                  ))}
                </select>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => removeDraft(index)}
                  disabled={disabled || drafts.length <= 2}
                >
                  <Trash2 size={13} />
                </BookButton>
              </div>
              {inspection ? <ChoiceTargetSummary inspection={inspection} /> : null}
            </div>
          );
        })}
      </div>
      {targetInspections.some((item) => item.kind === "error") ? (
        <div className="mt-2 border border-error/50 bg-book-bg px-2 py-1 text-[11px] text-error">
          存在缺失目标或未回收敛支线，导出前需要修正。
        </div>
      ) : null}
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2">
        <span
          className={`text-[11px] ${
            canSave && targetIssueCount === 0 ? "text-book-text-muted" : "text-error"
          }`}
        >
          {validationMessage}
        </span>
        <div className="flex items-center gap-2">
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

const ChoiceTargetSummary: React.FC<{ inspection: ChoiceTargetInspection }> = ({ inspection }) => {
  const className =
    inspection.kind === "error"
      ? "border-error/50 text-error"
      : "border-book-border text-book-text-muted";
  return (
    <div className={`border bg-book-bg px-2 py-1 text-[11px] ${className}`}>
      {inspection.message}
    </div>
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
    return { kind: "error", message: "目标场 ID 无效" };
  }
  const target = sceneById.get(sceneId);
  if (!target) {
    return { kind: "error", message: `目标场不存在 #${sceneId}` };
  }
  if (target.scene.entry_kind !== "branch") {
    return {
      kind: "main",
      message: `跳转到主线 #${target.scene.scene_index} ${target.scene.title}`,
    };
  }
  const returnSceneId = target.scene.next_scene_id;
  if (returnSceneId == null) {
    return {
      kind: "error",
      message: `支线 #${target.scene.scene_index} ${target.scene.title} 未设置回收敛场`,
    };
  }
  const returnScene = sceneById.get(returnSceneId);
  if (!returnScene) {
    return {
      kind: "error",
      message: `支线 #${target.scene.scene_index} 回收敛目标缺失 #${returnSceneId}`,
    };
  }
  if (returnScene.scene.entry_kind !== "main") {
    return {
      kind: "error",
      message: `支线 #${target.scene.scene_index} 回收敛目标不是主线 #${returnScene.scene.scene_index}`,
    };
  }
  return {
    kind: "branch",
    message: `进入支线 #${target.scene.scene_index} ${target.scene.title}，回到 #${returnScene.scene.scene_index} ${returnScene.scene.title}`,
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
  const targetSceneId = defaultChoiceTargetSceneId(line, scenes, sceneById)?.toString() ?? "";
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
    scenes.find((item) => item.scene.id !== line.scene_id)?.scene.id ?? scenes[0]?.scene.id ?? null
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
  return (
    <div className="mt-2 border-t border-book-border pt-2">
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
      <div className="mb-2 grid gap-2 lg:grid-cols-2">
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
        <div className="space-y-2">
          {audioBindings.map((item) => {
            const src = item.asset.file_path ? toMediaSrc(item.asset.file_path) : "";
            return (
              <div
                key={item.binding.id}
                className="grid gap-2 border border-book-border bg-book-surface p-2 text-[11px] text-book-text-muted md:grid-cols-[64px_minmax(0,1fr)_auto]"
              >
                <span className="font-semibold text-book-text-main">
                  {item.binding.role === "bgm" ? "BGM" : "SFX"}
                </span>
                {src ? (
                  <audio src={src} controls className="h-8 w-full" />
                ) : (
                  <span>音频文件缺失</span>
                )}
                <BookButton variant="secondary" size="sm" onClick={() => onUnbindMedia(item)} disabled={disabled}>
                  <X size={13} />
                </BookButton>
              </div>
            );
          })}
        </div>
      ) : null}
    </div>
  );
};

const ImageBindingStrip: React.FC<{
  title: string;
  emptyLabel: string;
  bindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  bindControl?: React.ReactNode;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({ title, emptyLabel, bindings, disabled, bindControl, onUnbindMedia }) => {
  if (!bindings.length) {
    return (
      <div className="mb-2 space-y-2">
        {emptyLabel ? (
          <div className="border border-dashed border-book-border bg-book-surface px-3 py-4 text-center text-[11px] text-book-text-muted">
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
            <div key={item.binding.id} className="border border-book-border bg-book-surface">
              <div className="flex items-center justify-between gap-2 border-b border-book-border px-2 py-1">
                <span className="truncate text-[11px] font-semibold text-book-text-main">
                  {title} #{item.asset.id}
                </span>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => onUnbindMedia(item)}
                  disabled={disabled}
                >
                  <X size={13} />
                </BookButton>
              </div>
              <div className="aspect-video w-full overflow-hidden bg-book-bg">
                {src ? (
                  <img
                    src={src}
                    alt={`${title} #${item.asset.id}`}
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <div className="flex h-full items-center justify-center text-[11px] text-book-text-muted">
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
    <div className="border border-book-border bg-book-surface p-2">
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
      <MediaAssetIdBinder label={`绑定 ${label} 资产`} disabled={disabled} onBind={onBind} />
    </div>
  );
};

const MediaAssetIdBinder: React.FC<{
  label: string;
  disabled: boolean;
  onBind: (mediaAssetId: number) => void;
}> = ({ label, disabled, onBind }) => {
  const [draft, setDraft] = useState("");
  const [error, setError] = useState("");
  const mediaAssetId = parsePositiveInteger(draft);

  const submit = () => {
    if (mediaAssetId == null) {
      setError("请输入正整数媒体资产 ID");
      return;
    }
    setError("");
    onBind(mediaAssetId);
  };

  return (
    <div className="space-y-1">
      <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
        <input
          className="min-w-0 border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
          value={draft}
          disabled={disabled}
          inputMode="numeric"
          placeholder="media_asset_id"
          onChange={(event) => {
            setDraft(event.target.value);
            if (error) setError("");
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") submit();
          }}
        />
        <BookButton
          variant="secondary"
          size="sm"
          onClick={submit}
          disabled={disabled || mediaAssetId == null}
        >
          <Images size={13} />
          {label}
        </BookButton>
      </div>
      {error ? <div className="text-[11px] text-error">{error}</div> : null}
    </div>
  );
};

function pipelineStatusLabel(status: string): string {
  switch (status) {
    case "running":
      return "运行中";
    case "completed":
      return "已完成";
    case "failed":
      return "失败";
    case "cancelled":
      return "已取消";
    case "interrupted":
      return "已中断";
    case "superseded":
      return "已取代";
    default:
      return status;
  }
}

function pipelineStepLabel(stepKey: string): string {
  switch (stepKey) {
    case "persist_script":
      return "写入剧本";
    default:
      return stepKey;
  }
}

function sceneEntryKindLabel(entryKind: string): string {
  switch (entryKind) {
    case "main":
      return "主线";
    case "branch":
      return "支线";
    default:
      return entryKind;
  }
}

function sceneSelectLabel(scene: GalgameScene): string {
  const entryKind = scene.entry_kind === "branch" ? "支线" : "主线";
  return `#${scene.scene_index} ${scene.title} · ${entryKind}`;
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
      return outcome;
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
      return status;
  }
}

function buildPortraitExpressionCells(
  items: GalgameAssetPlanItem[],
  speakerProfiles: GalgameSpeakerProfile[],
): PortraitExpressionCell[] {
  const speakerNameById = new Map(speakerProfiles.map((profile) => [profile.id, profile.name]));
  return items.map((item) => {
    const params = recordValue(item.params_json);
    const profileId = numberValue(params.character_profile_id) ?? item.target_entity_id;
    const profileName = speakerNameById.get(profileId) ?? `角色 #${profileId}`;
    const expression = stringValue(params.expression);
    const pose = stringValue(params.pose);
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
  return EXPRESSIONS.find((item) => item.value === expression)?.label ?? expression;
}

function recordValue(value: unknown): Record<string, unknown> {
  return value != null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function requireRecordValue(value: unknown, fieldName: string): Record<string, unknown> {
  if (value != null && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  throw new Error(`${fieldName} 必须是 JSON 对象`);
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function numberValue(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function parsePositiveInteger(value: string): number | null {
  const trimmed = value.trim();
  if (!/^[1-9]\d*$/.test(trimmed)) return null;
  const parsed = Number(trimmed);
  return Number.isSafeInteger(parsed) ? parsed : null;
}

interface PresentationPatch {
  cgEnabled?: boolean;
  cgNote?: string;
  cgImportance?: string;
  transition?: string;
}

function presentationCgNote(value: Record<string, unknown>): string {
  return (
    stringValue(value.cg_note) ||
    stringValue(value.cg_prompt) ||
    stringValue(value.event_cg_note)
  );
}

function updatePresentationJson(
  current: Record<string, unknown>,
  patch: PresentationPatch,
): Record<string, unknown> {
  const next = { ...current };

  if (patch.cgEnabled === false) {
    clearEventCgPresentation(next);
  } else if (patch.cgEnabled === true) {
    if (presentationCgNote(next)) next.cg = true;
  }

  if (patch.cgNote != null) {
    const note = patch.cgNote.trim();
    delete next.cg_prompt;
    delete next.event_cg_note;
    if (note) {
      next.cg = true;
      next.cg_note = note;
    } else {
      clearEventCgPresentation(next);
    }
  }

  if (patch.cgImportance != null) {
    const importance = patch.cgImportance.trim();
    if (isValidCgImportance(importance)) {
      next.cg_importance = importance;
    } else {
      delete next.cg_importance;
    }
  }

  if (presentationCgNote(next)) {
    next.cg = true;
    if (!isValidCgImportance(stringValue(next.cg_importance))) {
      next.cg_importance = DEFAULT_CG_IMPORTANCE;
    }
  } else {
    clearEventCgPresentation(next);
  }

  if (patch.transition != null) {
    if (patch.transition) {
      next.transition = patch.transition;
    } else {
      delete next.transition;
    }
  }

  return next;
}

function clearEventCgPresentation(value: Record<string, unknown>): void {
  delete value.cg;
  delete value.cg_note;
  delete value.cg_prompt;
  delete value.event_cg_note;
  delete value.cg_importance;
}

function isValidCgImportance(value: string): boolean {
  return CG_IMPORTANCE_VALUES.includes(value);
}

function selectValue(value: unknown, options: Array<{ value: string }>): string {
  const normalized = stringValue(value);
  return options.some((item) => item.value === normalized) ? normalized : "";
}

function fallbackAudioMime(fileName: string): string {
  const lower = fileName.toLowerCase();
  if (lower.endsWith(".mp3")) return "audio/mpeg";
  if (lower.endsWith(".wav")) return "audio/wav";
  if (lower.endsWith(".ogg")) return "audio/ogg";
  return "application/octet-stream";
}

function formatPipelineTime(value: string): string {
  const time = new Date(value);
  if (Number.isNaN(time.getTime())) return value;
  return time.toLocaleString();
}
