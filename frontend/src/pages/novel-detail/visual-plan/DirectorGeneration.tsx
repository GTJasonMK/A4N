import { useEffect, useRef, useState } from "react";
import { Check, Sparkles, Square } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  previewVisualPlanPointGeneration, readVisualPlanPromptCandidate, startVisualPlanPromptGeneration,
  type ChapterVisualPlanV3, type ChapterVisualPlanView, type VisualPlanPointGenerationPreview, type VisualPlanPromptCandidate,
} from "../../../api/chapterVisualPlan";
import { listImageConfigs, startImageGeneration, verifyImageGenerationResult, type GeneratedImage, type ImageGenConfig } from "../../../api/image";
import type { GenerationTaskHandle } from "../../../api/generated/generationContracts";
import { useGenerationTask } from "../../../hooks/useGenerationTask";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { sha256Json } from "../../../utils/contentHash";
import { formatTime, segmentTimes, type DirectorSelection } from "./timeline";

interface Props {
  projectId: number;
  chapterId: number;
  plan: ChapterVisualPlanV3;
  target: DirectorSelection;
  save: () => Promise<ChapterVisualPlanView>;
  onAdoptImage: (pointId: string, image: GeneratedImage) => Promise<void>;
  onAdoptPrompt: (candidate: VisualPlanPromptCandidate) => Promise<void>;
  onClose: () => void;
}

export function DirectorGeneration({ projectId, chapterId, plan, target, save, onAdoptImage, onAdoptPrompt, onClose }: Props) {
  const key = `a4n:director-task:${location.origin}:${projectId}:${chapterId}:${target.kind}:${target.id}`;
  const [handle, setHandle] = useState<GenerationTaskHandle | null>(null);
  const [configs, setConfigs] = useState<ImageGenConfig[]>([]);
  const [configId, setConfigId] = useState<number | null>(null);
  const [description, setDescription] = useState("");
  const [preview, setPreview] = useState<VisualPlanPointGenerationPreview | null>(null);
  const [previewRecord, setPreviewRecord] = useState<ChapterVisualPlanView | null>(null);
  const [image, setImage] = useState<GeneratedImage | null>(null);
  const [candidate, setCandidate] = useState<VisualPlanPromptCandidate | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const scope = useRef(key);
  const task = useGenerationTask({ handle });
  const active = handle != null && !["completed", "failed", "cancelled", "interrupted"].includes(task.taskStatus);
  const point = target.kind === "point" ? plan.points.find((point) => point.id === target.id) : null;
  const segment = target.kind === "segment" ? plan.segments.find((segment) => segment.id === target.id) : null;

  useEffect(() => {
    scope.current = key;
    setImage(null);
    setCandidate(null);
    setPreview(null);
    setPreviewRecord(null);
    try {
      const stored = sessionStorage.getItem(key);
      const value: unknown = stored ? JSON.parse(stored) : null;
      if (value && typeof value === "object" && "task_id" in value && "attempt" in value) setHandle(value as GenerationTaskHandle);
      else setHandle(null);
    } catch (failure) { setError(extractErrorMessage(failure, "生成任务恢复失败")); }
    if (target.kind === "point") {
      void listImageConfigs().then((items) => {
        if (scope.current !== key) return;
        setConfigs(items);
        setConfigId(items.find((config) => config.is_default)?.id ?? items[0]?.id ?? null);
      }).catch((failure) => { if (scope.current === key) setError(extractErrorMessage(failure, "图片模型加载失败")); });
    }
    return () => { scope.current = ""; };
  }, [key, target.kind]);

  useEffect(() => {
    if (!handle || task.taskStatus !== "completed" || !task.resultRef) return;
    let mounted = true;
    const result = task.resultRef;
    const recover = async () => {
      if (target.kind === "point") {
        const [generated] = await verifyImageGenerationResult(task, projectId);
        const metadata = generated.metadata_json as { director_candidate?: { point_id?: string } } | null;
        if (metadata?.director_candidate?.point_id !== target.id) throw new Error("生成图片的目标与当前图片点不一致");
        if (mounted) setImage(generated);
      } else {
        if (result.kind !== "visual_plan_prompt_candidate") throw new Error("生成结果类型不正确");
        const value = await readVisualPlanPromptCandidate(projectId, Number(result.id));
        if (value.segment_id !== target.id || value.source_chapter_id !== chapterId || await sha256Json(value) !== result.revision) throw new Error("生成候选的目标或内容校验失败");
        if (mounted) setCandidate(value);
      }
    };
    void recover().catch((failure) => { if (mounted) setError(extractErrorMessage(failure, "读取生成候选失败")); });
    return () => { mounted = false; };
  }, [handle, task.taskStatus, task.resultRef, projectId, chapterId, target.kind, target.id]);

  const prepare = async () => {
    setBusy(true);
    setError(null);
    try {
      const record = await save();
      const result = await previewVisualPlanPointGeneration(projectId, {
        source_chapter_id: chapterId, record_id: record.record.id, expected_revision: record.record.revision,
        point_id: target.id, user_description: description, config_id: configId,
      });
      if (scope.current !== key) return;
      setPreview(result);
      setPreviewRecord(record);
    } catch (failure) { if (scope.current === key) setError(extractErrorMessage(failure, "生成请求预览失败")); }
    finally { if (scope.current === key) setBusy(false); }
  };

  const generate = async () => {
    setBusy(true);
    setError(null);
    try {
      const record = await save();
      if (target.kind === "point" && (!preview || previewRecord?.record.revision !== record.record.revision)) {
        throw new Error("编排已改变，请重新预览生图请求");
      }
      const next = target.kind === "point" ? await startImageGeneration({
        project_id: projectId, operation: { kind: "visual_plan_point", options: {
          source_chapter_id: chapterId, record_id: record.record.id, expected_revision: record.record.revision,
          point_id: target.id, user_description: description, config_id: configId,
        } },
      }) : await startVisualPlanPromptGeneration({
        project_id: projectId, source_chapter_id: chapterId, record_id: record.record.id,
        expected_revision: record.record.revision, segment_id: target.id, user_description: description,
      });
      sessionStorage.setItem(key, JSON.stringify(next));
      if (scope.current !== key) return;
      setHandle(next);
      setImage(null);
      setCandidate(null);
    } catch (failure) { if (scope.current === key) setError(extractErrorMessage(failure, "启动生成失败")); }
    finally { if (scope.current === key) setBusy(false); }
  };

  const adopt = async () => {
    setBusy(true);
    try {
      if (image) await onAdoptImage(target.id, image);
      else if (candidate) await onAdoptPrompt(candidate);
      else throw new Error("没有可采用的候选");
      onClose();
    } catch (failure) { setError(extractErrorMessage(failure, "采用候选失败")); }
    finally { setBusy(false); }
  };

  return <Modal isOpen title={target.kind === "point" ? "生成分镜图片" : "生成镜头变化描述"} onClose={onClose} maxWidthClassName="max-w-3xl">
    <div className="director-generation">
      <p className="director-context-line">{point ? `${point.label || "图片点"} · ${formatTime(point.at_ms)}` : segment ? `${segment.title} · ${formatTime(segmentTimes(plan, segment)[1] - segmentTimes(plan, segment)[0])}` : "目标已删除"}</p>
      {error && <p role="alert" className="director-error">{error}</p>}
      {task.error && <p role="alert" className="director-error">{task.error.message}</p>}
      {target.kind === "point" && <label>图片模型<select value={configId ?? ""} disabled={active || busy} onChange={(event) => { setConfigId(Number(event.target.value)); setPreview(null); }}>
        {configs.length === 0 && <option value="">请先在模型设置中配置图片模型</option>}
        {configs.map((config) => <option key={config.id} value={config.id}>{config.name} · {config.model}</option>)}
      </select></label>}
      <label>这次希望怎样生成？<textarea rows={4} value={description} disabled={active || busy} placeholder={point ? "例如：沿用参考图人物，雨夜街口的近景，人物回头看向右侧。" : "例如：镜头缓缓推近，人物从犹豫到下定决心，最后握紧门把手。"}
        onChange={(event) => { setDescription(event.target.value); setPreview(null); }} /></label>
      {point && <p className="director-context-line">将使用本点描述、所属镜头、章节与场景资料，以及 {point.reference_image_ids.length} 张已选参考图。</p>}
      {segment && <details className="director-request-preview"><summary>本次生成依据</summary><p>{segment.prompt || "尚无已有描述"}</p><p>{segment.use_end_point_image ? "已明确关联结束点图片作为目标尾帧。" : "结束点图片不参与当前镜头约束。"}</p><p>按时间顺序使用本段起点和内部关键帧，文本模型沿用当前项目配置。</p></details>}
      {preview && <details className="director-request-preview" open><summary>生图请求 · {preview.aspect_ratio} · {preview.reference_image_ids.length} 张参考图</summary><pre>{preview.prompt}</pre></details>}
      <div className="director-inline-actions">
        {point && <BookButton variant="secondary" disabled={busy || active || configId == null} onClick={() => void prepare()}>预览生图请求</BookButton>}
        <BookButton disabled={busy || active || (!point && !segment) || (target.kind === "point" && (!preview || configId == null))} onClick={() => void generate()}><Sparkles size={15} />{handle ? "重新生成候选" : "生成候选"}</BookButton>
        {active && <BookButton variant="secondary" onClick={() => void task.cancel().catch((failure) => setError(extractErrorMessage(failure, "取消失败")))}><Square size={13} />停止生成</BookButton>}
      </div>
      {active && <p role="status">{task.progress?.message || "正在生成候选…"}</p>}
      {handle && task.recoveryState === "failed" && <BookButton variant="secondary" onClick={() => void task.recover().catch((failure) => setError(String(failure)))}>恢复任务状态</BookButton>}
      {image && <div className="director-candidate"><img src={toMediaSrc(image.file_path)} alt="新生成的分镜候选" /><span>{image.model}</span></div>}
      {candidate && <div className="director-candidate"><label>候选描述<textarea rows={8} value={candidate.prompt} onChange={(event) => setCandidate({ ...candidate, prompt: event.target.value })} /></label><span>{candidate.model} · 生成依据为方案 revision {candidate.plan_revision}</span></div>}
      {(image || candidate) && <BookButton disabled={busy || active || (candidate != null && !candidate.prompt.trim())} onClick={() => void adopt()}><Check size={15} />采用候选</BookButton>}
    </div>
  </Modal>;
}
