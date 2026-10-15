import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Sparkles, Square } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  previewVisualPlanPointGeneration, readVisualPlanPromptCandidate, startVisualPlanPromptGeneration,
  type ChapterVisualPlanV3, type ChapterVisualPlanView, type VisualPlanPromptCandidate,
} from "../../../api/chapterVisualPlan";
import {
  listImageConfigs, startImageGeneration, verifyImageGenerationResult,
  type GeneratedImage, type ImageSize,
} from "../../../api/image";
import { generationClient } from "../../../api/generationTasks";
import type { GenerationTaskHandle } from "../../../api/generated/generationContracts";
import { useGenerationTask } from "../../../hooks/useGenerationTask";
import { BookButton } from "../../../components/ui/BookButton";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { sha256Json } from "../../../utils/contentHash";
import type { DirectorSelection } from "./timeline";
import { useI18n } from "../../../i18n";

interface Props {
  projectId: number;
  chapterId: number;
  plan: ChapterVisualPlanV3;
  target: DirectorSelection;
  save: () => Promise<ChapterVisualPlanView>;
  selectedSize: ImageSize | null;
  availableSizes: readonly ImageSize[];
  autoStart?: boolean;
  hasCandidates?: boolean;
  onImageGenerated?: (image: GeneratedImage) => void;
  onAdoptPrompt: (candidate: VisualPlanPromptCandidate) => Promise<boolean>;
  onCandidateDirty?: (dirty: boolean) => void;
}

export function DirectorGeneration({ projectId, chapterId, plan, target, save, selectedSize, availableSizes, autoStart = false, hasCandidates = false, onImageGenerated, onAdoptPrompt, onCandidateDirty }: Props) {
  const { t } = useI18n();
  const key = `a4n:director-task:${location.origin}:${projectId}:${chapterId}:${target.kind}:${target.id}`;
  const [handle, setHandle] = useState<GenerationTaskHandle | null>(null);
  const [candidate, setCandidate] = useState<VisualPlanPromptCandidate | null>(null);
  const candidateBaseline = useRef<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const scope = useRef(key);
  const started = useRef<string | null>(null);
  const cancelRequested = useRef(false);
  const generationRef = useRef<HTMLDivElement>(null);
  const startRef = useRef<() => Promise<void>>(() => Promise.resolve());
  const task = useGenerationTask({ handle });
  const point = target.kind === "point" ? plan.points.find((item) => item.id === target.id) : null;
  const segment = target.kind === "segment" ? plan.segments.find((item) => item.id === target.id) : null;
  const description = point?.description ?? segment?.prompt ?? "";
  const active = handle != null && !["completed", "failed", "cancelled", "interrupted"].includes(task.taskStatus);
  const report = useCallback((failure: unknown, fallback: string) => {
    const message = extractErrorMessage(failure, fallback);
    setError(message);
  }, []);

  const startGeneration = useCallback(async () => {
    cancelRequested.current = false;
    setCancelling(false);
    setBusy(true);
    setError(null);
    let taskStarted = false;
    try {
      if (!point && !segment) throw new Error(t("visualPlan.generation.targetMissing"));
      let configId: number | null = null;
      if (point) {
        if (!selectedSize || !availableSizes.some((size) => size.width === selectedSize.width && size.height === selectedSize.height)) {
          throw new Error(t("visualPlan.generation.matchingSizeRequired"));
        }
        const configs = await listImageConfigs();
        configId = configs.find((config) => config.is_default)?.id ?? configs[0]?.id ?? null;
        if (configId == null) throw new Error(t("visualPlan.generation.imageConfigRequired"));
      }
      const record = await save();
      if (scope.current !== key || cancelRequested.current) return;
      let next: GenerationTaskHandle;
      if (point) {
        await previewVisualPlanPointGeneration(projectId, {
          source_chapter_id: chapterId, record_id: record.record.id, expected_revision: record.record.revision,
          point_id: target.id, user_description: description, config_id: configId, size: selectedSize,
        });
        if (scope.current !== key) return;
        next = await startImageGeneration({
          project_id: projectId, operation: { kind: "visual_plan_point", options: {
            source_chapter_id: chapterId, record_id: record.record.id, expected_revision: record.record.revision,
            point_id: target.id, user_description: description, config_id: configId, size: selectedSize,
          } },
        });
      } else {
        next = await startVisualPlanPromptGeneration({
          project_id: projectId, source_chapter_id: chapterId, record_id: record.record.id,
          expected_revision: record.record.revision, segment_id: target.id, user_description: description,
        });
      }
      if (scope.current !== key) return;
      sessionStorage.setItem(key, JSON.stringify(next));
      setHandle(next);
      setCandidate(null);
      candidateBaseline.current = null;
      onCandidateDirty?.(false);
      taskStarted = true;
      if (cancelRequested.current) {
        try {
          await generationClient.cancel(next);
        } catch (failure) {
          cancelRequested.current = false;
          setCancelling(false);
          report(failure, t("visualPlan.generation.cancelFailed"));
        }
      }
    } catch (failure) {
      if (scope.current === key) report(failure, t("visualPlan.generation.startFailed"));
    } finally {
      if (scope.current === key) {
        setBusy(false);
        if (!taskStarted) {
          cancelRequested.current = false;
          setCancelling(false);
        }
      }
    }
  }, [availableSizes, chapterId, description, key, onCandidateDirty, point, projectId, report, save, segment, selectedSize, t, target.id]);

  startRef.current = startGeneration;

  useEffect(() => {
    if (active) generationRef.current?.querySelector(".director-generation-action")?.scrollIntoView({ block: "nearest" });
  }, [active]);

  useEffect(() => {
    if (!active && !busy && cancelling) {
      cancelRequested.current = false;
      setCancelling(false);
    }
  }, [active, busy, cancelling]);

  useEffect(() => {
    scope.current = key;
    cancelRequested.current = false;
    setCancelling(false);
    setCandidate(null);
    candidateBaseline.current = null;
    onCandidateDirty?.(false);
    setError(null);
    let storedHandle: GenerationTaskHandle | null = null;
    try {
      const stored = sessionStorage.getItem(key);
      const value: unknown = stored ? JSON.parse(stored) : null;
      if (value && typeof value === "object" && "task_id" in value && "attempt" in value) storedHandle = value as GenerationTaskHandle;
    } catch (failure) {
      sessionStorage.removeItem(key);
      report(failure, t("visualPlan.generation.recoveryFailed"));
    }
    if (storedHandle) {
      started.current = key;
      setHandle(storedHandle);
    } else if (autoStart && (segment || point) && started.current !== key) {
      started.current = key;
      void startRef.current();
    }
    return () => { scope.current = ""; };
  }, [autoStart, key, point, report, segment]);

  useEffect(() => {
    if (!handle || task.taskStatus !== "completed" || !task.resultRef) return;
    let mounted = true;
    const result = task.resultRef;
    const recover = async () => {
      if (target.kind === "point") {
        const [generated] = await verifyImageGenerationResult(task, projectId);
        const metadata = generated.metadata_json as { director_candidate?: { point_id?: string } } | null;
        if (metadata?.director_candidate?.point_id !== target.id) throw new Error(t("visualPlan.generation.imageTargetMismatch"));
        if (mounted) {
          onImageGenerated?.(generated);
          sessionStorage.removeItem(key);
        }
      } else {
        if (result.kind !== "visual_plan_prompt_candidate") throw new Error(t("visualPlan.generation.resultTypeInvalid"));
        const value = await readVisualPlanPromptCandidate(projectId, Number(result.id));
        if (value.segment_id !== target.id || value.source_chapter_id !== chapterId || await sha256Json(value) !== result.revision) throw new Error(t("visualPlan.generation.candidateTargetMismatch"));
        if (mounted) {
          candidateBaseline.current = value.prompt;
          setCandidate(value);
          onCandidateDirty?.(false);
        }
      }
    };
    void recover().catch((failure) => { if (mounted) report(failure, t("visualPlan.generation.readCandidateFailed")); });
    return () => { mounted = false; };
  }, [chapterId, handle, onCandidateDirty, onImageGenerated, projectId, report, t, target.id, target.kind, task.resultRef, task.taskStatus]);

  const retryGeneration = () => {
    cancelRequested.current = false;
    setCancelling(false);
    sessionStorage.removeItem(key);
    started.current = null;
    setHandle(null);
    setCandidate(null);
    candidateBaseline.current = null;
    if (candidate != null) onCandidateDirty?.(false);
    void startRef.current();
  };

  const discardCandidateBeforeRegeneration = async () => {
    if (candidate == null || candidate.prompt === candidateBaseline.current) {
      retryGeneration();
      return;
    }
    const discard = await confirmDialog({
      title: t("visualPlan.candidateEditDiscardTitle"),
      message: t("visualPlan.candidateEditDiscardMessage"),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    });
    if (discard) retryGeneration();
  };

  const adopt = async () => {
    setBusy(true);
    setError(null);
    try {
      if (!candidate) throw new Error(t("visualPlan.generation.candidateMissing"));
      const accepted = await onAdoptPrompt(candidate);
      if (accepted === false) return;
      sessionStorage.removeItem(key);
      started.current = null;
      setHandle(null);
      setCandidate(null);
      candidateBaseline.current = null;
      onCandidateDirty?.(false);
    } catch (failure) { report(failure, t("visualPlan.generation.adoptFailed")); }
    finally { setBusy(false); }
  };

  const cancelGeneration = async () => {
    if (cancelling) return;
    cancelRequested.current = true;
    setCancelling(true);
    setError(null);
    if (!handle) return;
    try { await task.cancel(); }
    catch (failure) {
      cancelRequested.current = false;
      setCancelling(false);
      report(failure, t("visualPlan.generation.cancelFailed"));
    }
  };
  const preparing = busy && !handle;
  const stopping = preparing || active;
  const triggerGeneration = () => {
    if (handle || candidate || error || task.error || hasCandidates) void discardCandidateBeforeRegeneration();
    else void startRef.current();
  };
  const action = <BookButton
    className="director-generation-action"
    variant="secondary"
    disabled={cancelling || (!stopping && busy)}
    onClick={() => { if (stopping) void cancelGeneration(); else triggerGeneration(); }}
  >
    {stopping ? <Square size={13} /> : <Sparkles size={15} />}
    {cancelling ? t("visualPlan.generation.action.stopping")
      : stopping ? t("visualPlan.generation.action.stop")
        : error || task.error ? t("visualPlan.generation.action.retry")
          : handle || candidate || hasCandidates ? t("visualPlan.generation.action.regenerate")
            : point ? t("visualPlan.generation.action.generateImage")
              : t("visualPlan.generation.action.generateCandidate")}
  </BookButton>;

  if (point) return <div ref={generationRef} className="director-generation director-generation-compact" aria-live="polite">
    {busy && !handle && <p role="status">{t("visualPlan.generation.status.preparing")}</p>}
    {active && <p role="status">{task.progress?.message || t("visualPlan.generation.status.generating")}</p>}
    {handle && ["cancelled", "interrupted"].includes(task.taskStatus) && <p role="status">{t("visualPlan.generation.status.stopped")}</p>}
    {handle && !active && task.recoveryState === "failed" && <BookButton variant="secondary" onClick={() => void task.recover().catch((failure) => report(failure, t("visualPlan.generation.recoveryFailed")))}>{t("visualPlan.generation.status.restoreTask")}</BookButton>}
    {!active && !busy && (error || task.error?.message) && <p role="alert" className="director-context-line">{error || task.error?.message}</p>}
    {action}
  </div>;
  return <div ref={generationRef} className="director-generation" aria-live="polite">
    {busy && !handle && <p role="status">{t("visualPlan.generation.status.preparing")}</p>}
    {active && <p role="status">{task.progress?.message || t("visualPlan.generation.status.generating")}</p>}
    {handle && ["cancelled", "interrupted"].includes(task.taskStatus) && <p role="status">{t("visualPlan.generation.status.stopped")}</p>}
    {handle && !active && task.recoveryState === "failed" && <BookButton variant="secondary" onClick={() => void task.recover().catch((failure) => report(failure, t("visualPlan.generation.recoveryFailed")))}>{t("visualPlan.generation.status.restoreTask")}</BookButton>}
    {candidate && <div className="director-candidate"><label>{t("visualPlan.generation.status.candidateDescription")}<textarea rows={8} value={candidate.prompt} onChange={(event) => { const prompt = event.target.value; onCandidateDirty?.(prompt !== candidateBaseline.current); setCandidate({ ...candidate, prompt }); }} /></label><span>{t("visualPlan.generation.status.generatedFromRevision", { model: candidate.model, revision: candidate.plan_revision })}</span></div>}
    {candidate && <BookButton disabled={busy || active || !candidate.prompt.trim()} onClick={() => void adopt()}><Check size={15} />{t("visualPlan.generation.action.adopt")}</BookButton>}
    {action}
  </div>;
}
