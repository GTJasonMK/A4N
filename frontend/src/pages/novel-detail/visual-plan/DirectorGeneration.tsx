import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Square } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  previewVisualPlanPointGeneration, readVisualPlanPromptCandidate, startVisualPlanPromptGeneration,
  type ChapterVisualPlanV3, type ChapterVisualPlanView, type VisualPlanPromptCandidate,
} from "../../../api/chapterVisualPlan";
import { listImageConfigs, startImageGeneration, verifyImageGenerationResult, type GeneratedImage } from "../../../api/image";
import type { GenerationTaskHandle } from "../../../api/generated/generationContracts";
import { useGenerationTask } from "../../../hooks/useGenerationTask";
import { BookButton } from "../../../components/ui/BookButton";
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
  const [image, setImage] = useState<GeneratedImage | null>(null);
  const [candidate, setCandidate] = useState<VisualPlanPromptCandidate | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [retry, setRetry] = useState(0);
  const scope = useRef(key);
  const started = useRef<string | null>(null);
  const startRef = useRef<() => Promise<void>>(() => Promise.resolve());
  const task = useGenerationTask({ handle });
  const point = target.kind === "point" ? plan.points.find((item) => item.id === target.id) : null;
  const segment = target.kind === "segment" ? plan.segments.find((item) => item.id === target.id) : null;
  const description = point?.description ?? segment?.prompt ?? "";
  const active = handle != null && !["completed", "failed", "cancelled", "interrupted"].includes(task.taskStatus);

  const startGeneration = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      if (!point && !segment) throw new Error("生成目标已不存在");
      let configId: number | null = null;
      if (point) {
        const configs = await listImageConfigs();
        configId = configs.find((config) => config.is_default)?.id ?? configs[0]?.id ?? null;
        if (configId == null) throw new Error("请先在模型设置中配置图片模型");
      }
      const record = await save();
      if (scope.current !== key) return;
      let next: GenerationTaskHandle;
      if (point) {
        await previewVisualPlanPointGeneration(projectId, {
          source_chapter_id: chapterId, record_id: record.record.id, expected_revision: record.record.revision,
          point_id: target.id, user_description: description, config_id: configId,
        });
        if (scope.current !== key) return;
        next = await startImageGeneration({
          project_id: projectId, operation: { kind: "visual_plan_point", options: {
            source_chapter_id: chapterId, record_id: record.record.id, expected_revision: record.record.revision,
            point_id: target.id, user_description: description, config_id: configId,
          } },
        });
      } else {
        next = await startVisualPlanPromptGeneration({
          project_id: projectId, source_chapter_id: chapterId, record_id: record.record.id,
          expected_revision: record.record.revision, segment_id: target.id, user_description: description,
        });
      }
      sessionStorage.setItem(key, JSON.stringify(next));
      if (scope.current !== key) return;
      setHandle(next);
      setImage(null);
      setCandidate(null);
    } catch (failure) {
      if (scope.current === key) setError(extractErrorMessage(failure, "启动生成失败"));
    } finally {
      if (scope.current === key) setBusy(false);
    }
  }, [chapterId, description, key, point, projectId, save, segment, target.id]);

  startRef.current = startGeneration;

  useEffect(() => {
    scope.current = key;
    setImage(null);
    setCandidate(null);
    setError(null);
    let storedHandle: GenerationTaskHandle | null = null;
    try {
      const stored = sessionStorage.getItem(key);
      const value: unknown = stored ? JSON.parse(stored) : null;
      if (value && typeof value === "object" && "task_id" in value && "attempt" in value) storedHandle = value as GenerationTaskHandle;
    } catch (failure) {
      sessionStorage.removeItem(key);
      setError(extractErrorMessage(failure, "生成任务恢复失败"));
    }
    if (storedHandle) {
      started.current = key;
      setHandle(storedHandle);
    } else if (started.current !== key) {
      started.current = key;
      void startRef.current();
    }
    return () => { scope.current = ""; };
  }, [key, retry]);

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
  }, [chapterId, handle, projectId, target.id, target.kind, task.resultRef, task.taskStatus]);

  const retryGeneration = () => {
    sessionStorage.removeItem(key);
    started.current = null;
    setHandle(null);
    setImage(null);
    setCandidate(null);
    setRetry((value) => value + 1);
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

  const canRetry = !busy && !active && !image && !candidate;
  return <div className="director-generation" aria-live="polite">
    <p className="director-context-line">{point ? `${point.label || "图片点"} · ${formatTime(point.at_ms)}` : segment ? `${segment.title} · ${formatTime(segmentTimes(plan, segment)[1] - segmentTimes(plan, segment)[0])}` : "目标已删除"}</p>
    <p className="director-context-line">使用当前描述和项目默认配置生成，完成后可直接采用候选结果。</p>
    {error && <p role="alert" className="director-error">{error}</p>}
    {task.error && <p role="alert" className="director-error">{task.error.message}</p>}
    {busy && !handle && <p role="status">正在准备生成…</p>}
    {active && <p role="status">{task.progress?.message || "正在生成候选…"}</p>}
    {handle && task.taskStatus === "cancelled" && <p role="status">生成已停止。</p>}
    {handle && !active && task.recoveryState === "failed" && <BookButton variant="secondary" onClick={() => void task.recover().catch((failure) => setError(extractErrorMessage(failure, "恢复任务状态失败")))}>恢复任务状态</BookButton>}
    {active && <BookButton variant="secondary" onClick={() => void task.cancel().catch((failure) => setError(extractErrorMessage(failure, "取消失败")))}><Square size={13} />停止生成</BookButton>}
    {image && <div className="director-candidate"><img src={toMediaSrc(image.file_path)} alt="新生成的分镜候选" /><span>{image.model}</span></div>}
    {candidate && <div className="director-candidate"><label>候选描述<textarea rows={8} value={candidate.prompt} onChange={(event) => setCandidate({ ...candidate, prompt: event.target.value })} /></label><span>{candidate.model} · 生成依据为方案 revision {candidate.plan_revision}</span></div>}
    {canRetry && <BookButton variant="secondary" onClick={retryGeneration}>{error || task.error ? "重试生成" : "重新生成"}</BookButton>}
    {(image || candidate) && <BookButton disabled={busy || active || (candidate != null && !candidate.prompt.trim())} onClick={() => void adopt()}><Check size={15} />采用候选</BookButton>}
  </div>;
}
