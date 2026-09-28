import React, { useCallback, useEffect, useId, useRef, useState } from "react";
import { getProjectFactFeedback, previewProjectFactFeedback, runProjectFactFeedback, type ProjectFactFeedbackSummary, type RunProjectFactFeedbackInput } from "../../api/factFeedback";
import { updateProject } from "../../api/project";
import { cancelWorkflowTask } from "../../api/streaming";
import { extractErrorMessage } from "../../api";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { BookButton } from "../../components/ui/BookButton";
import { BookInput } from "../../components/ui/BookInput";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { useProgressTask } from "../../hooks/useProgressTask";
import { useI18n, type MessageKey } from "../../i18n";
import { useNovelDetailDirty } from "./novelDetailDirty";

const MATERIAL_LABELS: Record<string, MessageKey> = {
  world_setting: "novel.factFeedback.world",
  blueprint_synopsis: "novel.factFeedback.synopsis",
  character_voice: "novel.factFeedback.voice",
  character_arc_progress: "novel.factFeedback.arc",
  planning: "novel.factFeedback.planning",
  object_profile: "novel.factFeedback.object",
  scene_profile: "novel.factFeedback.scene",
  foreshadowing: "novel.factFeedback.foreshadowing",
};

interface FactFeedbackPanelProps {
  projectId: number;
  canEdit: boolean;
  refreshKey?: string;
  onChanged?: () => boolean | void | Promise<boolean | void>;
}

export const FactFeedbackPanel = ({ projectId, canEdit, refreshKey, onChanged }: FactFeedbackPanelProps) => {
  const { t } = useI18n();
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const headingId = useId();
  const [data, setData] = useState<ProjectFactFeedbackSummary | null>(null);
  const [interval, setIntervalDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<ModelRequestPreview | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewLoading, setPreviewLoading] = useState(false);
  const currentProject = useRef(projectId);
  const mounted = useRef(false);
  const operationBusy = useRef(false);
  const intervalDirty = useRef(false);
  const loadSequence = useRef(0);
  currentProject.current = projectId;

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; loadSequence.current += 1; };
  }, []);

  const load = useCallback(async (): Promise<boolean> => {
    const sequence = ++loadSequence.current;
    try {
      const result = await getProjectFactFeedback(projectId);
      if (!mounted.current || currentProject.current !== projectId || loadSequence.current !== sequence) return false;
      setData(result);
      if (!intervalDirty.current) setIntervalDraft(String(result.interval_chapters));
      setError(null);
      return true;
    } catch (cause) {
      if (mounted.current && currentProject.current === projectId && loadSequence.current === sequence) {
        setError(extractErrorMessage(cause, t("novel.factFeedback.loadFailed")));
      }
      return false;
    } finally {
      if (mounted.current && currentProject.current === projectId && loadSequence.current === sequence) setLoading(false);
    }
  }, [projectId, t]);

  useEffect(() => {
    setData(null);
    setLoading(true);
    intervalDirty.current = false;
    setPreviewOpen(false);
  }, [projectId]);

  useEffect(() => { void load(); }, [load, refreshKey]);

  useEffect(() => {
    if (data?.workflow_status !== "running") return;
    const timer = globalThis.setInterval(() => void load(), 1500);
    return () => globalThis.clearInterval(timer);
  }, [data?.workflow_status, load]);

  const intervalHasDraft =
    intervalDirty.current && interval.trim() !== String(data?.interval_chapters ?? "");
  const requireSavedInterval = () => {
    if (!intervalHasDraft) return true;
    addToast(t("novel.factFeedback.saveIntervalBeforeRun"), "info");
    return false;
  };

  const run = async (mode: RunProjectFactFeedbackInput["mode"], resume = false) => {
    if (!canEdit || operationBusy.current) return;
    if (!requireSavedInterval()) return;
    operationBusy.current = true;
    setBusy(true);
    const requestProject = projectId;
    const taskId = crypto.randomUUID();
    let refreshAttempted = false;
    try {
      const result = await runProgressTask({
        title: t(mode === "rebuild" ? "novel.factFeedback.rebuild" : "novel.factFeedback.consolidate"),
        startMessage: t("novel.factFeedback.running"), successMessage: t("novel.factFeedback.finished"),
        errorMessage: t("novel.factFeedback.runFailed"), total: 100,
        cancelTaskId: taskId, cancelTask: cancelWorkflowTask,
      }, async () => {
        const next = await runProjectFactFeedback({ project_id: requestProject, mode, workflow_run_id: resume ? data?.workflow_run_id ?? null : null }, taskId);
        if (!mounted.current || currentProject.current !== requestProject) return next;
        refreshAttempted = onChanged != null;
        if (await onChanged?.() === false) throw new Error(t("novel.factFeedback.loadFailed"));
        return next;
      });
      if (!mounted.current || currentProject.current !== requestProject) return;
      setData(result);
      setError(null);
    } catch (cause) {
      if (mounted.current && currentProject.current === requestProject) {
        setError(extractErrorMessage(cause, t("novel.factFeedback.runFailed")));
        if (!refreshAttempted) await load();
      }
    } finally {
      operationBusy.current = false;
      if (mounted.current && currentProject.current === requestProject) setBusy(false);
    }
  };

  const saveInterval = async (event: React.FormEvent) => {
    event.preventDefault();
    const value = Number(interval);
    if (!canEdit || operationBusy.current || !Number.isSafeInteger(value) || value <= 0 || value > 2147483647) return;
    operationBusy.current = true;
    setBusy(true);
    const requestProject = projectId;
    try {
      await updateProject(requestProject, { title: null, description: null, fact_feedback_interval_chapters: value });
      if (!mounted.current || currentProject.current !== requestProject) return;
      intervalDirty.current = false;
      await load();
      await onChanged?.();
      addToast(t("novel.factFeedback.intervalSaved"), "success");
    } catch (cause) {
      if (mounted.current && currentProject.current === requestProject) setError(extractErrorMessage(cause, t("novel.factFeedback.saveFailed")));
    } finally {
      operationBusy.current = false;
      if (mounted.current && currentProject.current === requestProject) setBusy(false);
    }
  };

  const showPreview = async () => {
    if (operationBusy.current || !requireSavedInterval()) return;
    setPreviewOpen(true);
    setPreviewLoading(true);
    setPreview(null);
    const requestProject = projectId;
    try {
      const result = await previewProjectFactFeedback({ project_id: projectId, mode: "consolidate", workflow_run_id: null });
      if (mounted.current && currentProject.current === requestProject) setPreview(result);
    } catch (cause) {
      if (mounted.current && currentProject.current === requestProject) {
        setPreviewOpen(false);
        addToast(extractErrorMessage(cause, t("novel.factFeedback.previewFailed")), "error");
      }
    } finally {
      if (mounted.current && currentProject.current === requestProject) setPreviewLoading(false);
    }
  };

  const running = busy || data?.workflow_status === "running";
  useNovelDetailDirty(
    `fact-feedback:${projectId}`,
    intervalHasDraft,
    undefined,
    () => !operationBusy.current && !busy && !previewLoading,
  );
  return (
    <section aria-labelledby={headingId} className="a4n-flat-surface min-w-0 p-3 sm:p-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <h3 id={headingId} className="text-small font-semibold text-book-text-main">{t("novel.factFeedback.title")}</h3>
        <BookButton size="sm" variant="ghost" onClick={() => void load()} disabled={loading}>{t("novel.factFeedback.refresh")}</BookButton>
      </header>
      <p className="mt-1 text-caption leading-relaxed text-book-text-muted">{t("novel.factFeedback.description")}</p>
      {loading ? <div className="mt-3"><LoadingSpinner size="sm" /></div> : null}
      {error ? <p role="alert" className="mt-2 break-words text-small text-error">{error}</p> : null}
      {data ? <>
        <p role="status" className="mt-3 text-small text-book-text-main">
          {t("novel.factFeedback.coverage", { covered: data.consolidated_chapters.length, pending: data.pending_chapters.length, interval: data.interval_chapters })}
        </p>
        {data.legacy_chapters.length > 0 ? <p className="mt-1 text-caption text-book-text-muted">{t("novel.factFeedback.legacy", { count: data.legacy_chapters.length })}</p> : null}
        {data.error ? <p role="alert" className="mt-2 break-words text-small text-error">{data.error}</p> : null}
        {canEdit ? <div className="mt-3 space-y-3">
          <form onSubmit={(event) => void saveInterval(event)} className="flex flex-wrap items-end gap-2">
            <div className="w-36 max-w-full"><BookInput label={t("novel.factFeedback.interval")} type="number" inputMode="numeric" min={1} max={2147483647} step={1} required value={interval} onChange={(event) => { intervalDirty.current = true; setIntervalDraft(event.target.value); }} disabled={busy} /></div>
            <BookButton size="sm" variant="secondary" type="submit" disabled={busy || interval === String(data.interval_chapters)}>{t("novel.factFeedback.save")}</BookButton>
          </form>
          <div className="flex flex-wrap items-center gap-2">
            <BookButton size="sm" onClick={() => void run("consolidate")} disabled={running}>{t("novel.factFeedback.consolidate")}</BookButton>
            <BookButton size="sm" variant="secondary" onClick={() => void run("rebuild")} disabled={running}>{t("novel.factFeedback.rebuild")}</BookButton>
            {data.can_resume ? <BookButton size="sm" variant="secondary" onClick={() => void run(data.workflow_kind === "project_fact_rebuild" ? "rebuild" : "consolidate", true)} disabled={running}>{t("novel.factFeedback.resume")}</BookButton> : null}
            <BookButton size="sm" variant="ghost" onClick={() => void showPreview()} disabled={running}>{t("novel.factFeedback.preview")}</BookButton>
          </div>
          <p className="text-caption text-book-text-muted">{t("novel.factFeedback.rebuildHint")}</p>
        </div> : null}
        {data.changes.length > 0 ? <details className="mt-4 border-t border-book-border/40 pt-3">
          <summary className="cursor-pointer text-small font-medium text-book-text-main">{t("novel.factFeedback.changes", { count: data.changes.length })}</summary>
          <ol className="mt-2 divide-y divide-book-border/35">
            {data.changes.map((change) => <li key={change.id} className="min-w-0 py-3">
              <p className="text-small font-medium text-book-text-main">{t(MATERIAL_LABELS[change.material_kind] ?? "novel.factFeedback.material")} · {t(change.status === "proposal" ? "novel.factFeedback.proposal" : change.status === "retracted" ? "novel.factFeedback.retracted" : "novel.factFeedback.applied")}</p>
              <p className="mt-1 break-words text-small text-book-text-sub">{change.reason}</p>
              <div className="mt-2 grid min-w-0 grid-cols-1 gap-2 md:grid-cols-2">
                <div className="min-w-0"><p className="text-caption text-book-text-muted">{t("novel.factFeedback.before")}</p><pre className="mt-1 whitespace-pre-wrap break-all text-caption text-book-text-sub">{displayValue(change.before)}</pre></div>
                <div className="min-w-0"><p className="text-caption text-book-text-muted">{t("novel.factFeedback.after")}</p><pre className="mt-1 whitespace-pre-wrap break-all text-caption text-book-text-main">{displayValue(change.after)}</pre></div>
              </div>
              {Array.isArray(change.evidence) ? change.evidence.map((item, index) => {
                if (!item || typeof item !== "object") return null;
                const evidence = item as { chapter_number?: number; quote?: string };
                return evidence.quote ? <blockquote key={index} className="mt-2 break-words border-l-2 border-book-border pl-2 text-caption text-book-text-muted">{t("novel.factFeedback.evidence", { chapter: evidence.chapter_number ?? "" })}{evidence.quote}</blockquote> : null;
              }) : null}
            </li>)}
          </ol>
        </details> : <p className="mt-3 text-caption text-book-text-muted">{t("novel.factFeedback.empty")}</p>}
      </> : null}
      <RequestPreviewModal open={previewOpen} loading={previewLoading} data={preview} onClose={() => setPreviewOpen(false)} />
    </section>
  );
};

function displayValue(value: unknown): string {
  if (value == null) return "—";
  return typeof value === "string" ? value : JSON.stringify(value, null, 2);
}
