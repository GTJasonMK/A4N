import { useEffect, useRef, useState } from "react";
import {
  adoptMediaAuthoringDocument, generateMediaAdaptationPlan, listVideoAdaptationReferences,
  previewMediaAdaptationPlanRequest, saveMediaAuthoringDocument,
  type AdaptationPlanV1, type AuthoringDocumentKind, type AuthoringDocumentRecord,
  type GalgameBranchDesign, type MediaAuthoringWorkspace, type VideoScriptReferenceCandidate,
} from "../../api/mediaAuthoring";
import type { GenerateAdaptationPlanInput } from "../../api/generated/host";
import { extractErrorMessage } from "../../api";
import { cancelWorkflowTask } from "../../api/streaming";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { BookButton } from "../../components/ui/BookButton";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { confirmDiscardMediaChanges } from "../../components/feedback/UnsavedChangesGuard";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { useProgressTask } from "../../hooks/useProgressTask";
import { useI18n } from "../../i18n";
import { useDirty } from "../../components/navigation/dirtyRegistry";
import { AdaptationPlanEditor, newAdaptationUnit, PlanTextField } from "./AdaptationPlanEditor";
import { GalgameBranchPlanEditor } from "./MediaPlanEditors";

type PlanDocument = AdaptationPlanV1 | GalgameBranchDesign;

interface Props {
  projectId: number;
  kind: Exclude<AuthoringDocumentKind, "manga_page_plan">;
  workspace: MediaAuthoringWorkspace | null;
  loading: boolean;
  error: string | null;
  chapterId?: number | null;
  canEdit: boolean;
  stageRunning?: boolean;
  onReload: () => Promise<void>;
  onGenerateStage?: () => Promise<void>;
  onDirtyChange: (dirty: boolean) => void;
  onBusyChange?: (busy: boolean) => void;
  onNext?: () => void;
}

export function MediaAuthoringPanel({ projectId, kind, workspace, loading, error, chapterId: fixedChapterId, canEdit, stageRunning = false, onReload, onGenerateStage, onDirtyChange, onBusyChange, onNext }: Props) {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask } = useProgressTask();
  const [selectedChapterId, setSelectedChapterId] = useState<number | null>(null);
  const [versionId, setVersionId] = useState<number | null>(null);
  const [editor, setEditor] = useState<{ record: AuthoringDocumentRecord | null; document: PlanDocument; baseline: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [instruction, setInstruction] = useState("");
  const [references, setReferences] = useState<VideoScriptReferenceCandidate[]>([]);
  const [referenceError, setReferenceError] = useState<string | null>(null);
  const [referenceChoice, setReferenceChoice] = useState("saved");
  const [submittedOptions, setSubmittedOptions] = useState({ instruction: "", referenceChoice: "saved" });
  const [preview, setPreview] = useState<ModelRequestPreview | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const busyRef = useRef(false);
  const sourceChapterId = fixedChapterId ?? selectedChapterId ?? workspace?.chapters[0]?.source.source_chapter_id ?? null;
  const scope = `${projectId}:${kind}:${sourceChapterId}`;
  const dirtyKey = `media-authoring:${scope}`;
  const scopeRef = useRef(scope);
  scopeRef.current = scope;
  useEffect(() => { scopeRef.current = scope; return () => { scopeRef.current = ""; }; }, [scope]);
  const chapter = workspace?.chapters.find((item) => item.source.source_chapter_id === sourceChapterId);
  const chapterId = kind === "galgame_branch_plan" ? null : sourceChapterId;
  const versions = workspace?.documents.filter(({ record }) => record.kind === kind && record.source_chapter_id === chapterId) ?? [];
  const current = versions.find(({ record }) => record.source_snapshot_id === workspace?.source_snapshot_id && record.status === "draft")
    ?? versions.find(({ record }) => record.source_snapshot_id === workspace?.source_snapshot_id && record.status === "adopted");
  const selected = versions.find(({ record }) => record.id === versionId) ?? current;
  const dirty = editor != null && JSON.stringify(editor.document) !== editor.baseline;
  const generationOptionsDirty = kind === "adaptation_plan" && (
    instruction.trim() !== submittedOptions.instruction.trim() ||
    referenceChoice !== submittedOptions.referenceChoice
  );
  const draftDirty = dirty || generationOptionsDirty;
  const disabled = !canEdit || busy || stageRunning;
  const title = kind === "adaptation_plan"
    ? t("common.mediaAuthoring.adaptation")
    : t("common.mediaAuthoring.branches");

  const discard = () => {
    if (busy) return false;
    setEditor(null);
    setVersionId(null);
    setInstruction(submittedOptions.instruction);
    setReferenceChoice(submittedOptions.referenceChoice);
    onDirtyChange(false);
    return true;
  };
  useDirty(dirtyKey, draftDirty || busy, discard, () => !busy);
  useEffect(() => { onDirtyChange(draftDirty || busy); }, [draftDirty, busy, onDirtyChange]);
  useEffect(() => () => onDirtyChange(false), [onDirtyChange]);
  useEffect(() => { onBusyChange?.(busy); }, [busy, onBusyChange]);
  useEffect(() => () => onBusyChange?.(false), [onBusyChange]);
  useEffect(() => {
    setVersionId(null);
    setEditor(null);
    setActionError(null);
    busyRef.current = false;
    setBusy(false);
    setReferenceChoice("saved");
    setSubmittedOptions((current) => ({ ...current, referenceChoice: "saved" }));
  }, [scope]);
  useEffect(() => {
    if (dirty) return;
    if (!selected) { setEditor(null); return; }
    setEditor({ record: selected.record, document: selected.record.document as PlanDocument, baseline: JSON.stringify(selected.record.document) });
    // A background refresh must never overwrite an unsaved editor.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scope, selected?.record.id, selected?.record.revision, selected?.record.status]);
  useEffect(() => {
    if (kind !== "adaptation_plan" || !sourceChapterId) return;
    let active = true;
    setReferenceError(null);
    setReferences([]);
    void listVideoAdaptationReferences({ project_id: projectId, chapter_id: sourceChapterId })
      .then((items) => { if (active) setReferences(items); })
      .catch((error) => { if (active) setReferenceError(extractErrorMessage(error, t("common.mediaAuthoring.requestFailed"))); });
    return () => { active = false; };
  }, [kind, projectId, sourceChapterId, t]);

  const confirmDiscardDrafts = async () => {
    if (!await confirmDiscardMediaChanges(draftDirty, t)) return false;
    setInstruction(submittedOptions.instruction);
    setReferenceChoice(submittedOptions.referenceChoice);
    return true;
  };
  const commitEditor = (record: AuthoringDocumentRecord) => {
    if (scopeRef.current !== scope) return;
    setVersionId(record.id);
    setEditor({ record, document: record.document as PlanDocument, baseline: JSON.stringify(record.document) });
    onDirtyChange(false);
  };
  const run = async (action: () => Promise<void>) => {
    if (busyRef.current || disabled) return;
    const requestScope = scope;
    busyRef.current = true;
    setBusy(true);
    setActionError(null);
    try { await action(); }
    catch (error) {
      if (scopeRef.current === requestScope) {
        const message = extractErrorMessage(error, t("common.mediaAuthoring.requestFailed"));
        setActionError(message);
        addToast(message, "error");
      }
    }
    finally {
      if (scopeRef.current === requestScope) {
        busyRef.current = false;
        setBusy(false);
      }
    }
  };
  const save = async () => {
    if (!editor) throw new Error(t("common.mediaAuthoring.noPlan"));
    const saved = await saveMediaAuthoringDocument({ project_id: projectId, chapter_id: chapterId, kind,
      record_id: editor.record?.id ?? null, expected_revision: editor.record?.revision ?? null, document: editor.document });
    commitEditor(saved);
    await onReload();
    return saved;
  };
  const adopt = async () => {
    const requestScope = scope;
    const saved = dirty || !editor?.record ? await save() : editor.record;
    if (scopeRef.current !== requestScope) return;
    const adopted = await adoptMediaAuthoringDocument({ project_id: projectId, record_id: saved.id, expected_revision: saved.revision });
    if (scopeRef.current !== requestScope) return;
    commitEditor(adopted);
    await onReload();
  };
  const generationInput = (): GenerateAdaptationPlanInput => {
    if (!sourceChapterId) throw new Error(t("common.mediaAuthoring.chooseChapter"));
    return {
      project_id: projectId, chapter_id: sourceChapterId, task_id: `media-adaptation-${projectId}-${crypto.randomUUID()}`,
      instruction: instruction.trim() || t("common.mediaAuthoring.defaultInstruction"),
      video_reference: references.find((item) => String(item.selection.script_id) === referenceChoice)?.selection ?? null,
      clear_video_reference: referenceChoice === "independent", record_id: editor?.record?.id ?? null,
      expected_revision: editor?.record?.revision ?? null, resume_from_checkpoint: true,
    };
  };
  const generate = async () => {
    if (!await confirmDiscardMediaChanges(dirty, t)) return;
    const requestScope = scope;
    if (kind !== "adaptation_plan") {
      setEditor(null); setVersionId(null); onDirtyChange(false);
      await onGenerateStage?.();
      if (scopeRef.current !== requestScope) return;
      return;
    }
    const input = generationInput();
    const result = await runProgressTask({ title, startMessage: t("common.mediaAuthoring.generating"), successMessage: t("common.mediaAuthoring.draftSaved"), errorMessage: t("common.mediaAuthoring.generationFailed"), total: 1, streaming: true,
      cancelTaskId: input.task_id, cancelTask: cancelWorkflowTask, pollTaskProgress: true,
    }, () => generateMediaAdaptationPlan(input));
    if (scopeRef.current === requestScope) {
      setSubmittedOptions({ instruction, referenceChoice });
      commitEditor(result);
    }
    await onReload();
  };

  return <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-1 custom-scrollbar">
    <header className="flex flex-wrap items-start gap-3 border-b border-book-border/60 pb-3">
      <div className="min-w-0 flex-1 basis-64"><h2 className="text-base font-semibold">{title}</h2>
        {chapter && kind !== "galgame_branch_plan" && <p className="mt-1 text-xs font-medium text-book-text-sub">{t("writing.chapterLabel", { number: chapter.chapter_number })}{chapter.title ? ` · ${chapter.title}` : ""}</p>}
        <p className="mt-1 text-xs leading-relaxed text-book-text-muted">{kind === "adaptation_plan" ? t("common.mediaAuthoring.adaptationHint") : t("common.mediaAuthoring.planHint")}</p>
      </div>
      <div className="flex flex-wrap items-end gap-2">
        {kind !== "galgame_branch_plan" && fixedChapterId === undefined && <label className="grid gap-1 text-xs text-book-text-sub"><span>{t("common.mediaAuthoring.chooseChapter")}</span>
          <ThemedSelect aria-label={t("common.mediaAuthoring.chooseChapter")} className="max-w-full" value={sourceChapterId ?? ""} disabled={busy || stageRunning} onChange={(value) => { const next = Number(value); void (async () => { if (await confirmDiscardDrafts()) { setEditor(null); onDirtyChange(false); setSelectedChapterId(next); } })(); }}>
            {workspace?.chapters.map((item) => <option key={item.source.source_chapter_id} value={item.source.source_chapter_id}>{item.chapter_number} · {item.title}</option>)}
          </ThemedSelect>
        </label>}
        {versions.length > 0 && <label className="grid gap-1 text-xs text-book-text-sub"><span>{t("common.mediaAuthoring.version")}</span>
          <ThemedSelect aria-label={t("common.mediaAuthoring.version")} value={selected?.record.id ?? ""} disabled={busy || stageRunning} onChange={(value) => { const next = Number(value); void (async () => { if (await confirmDiscardDrafts()) { setEditor(null); onDirtyChange(false); setVersionId(next); } })(); }}>
            {versions.map(({ record }) => <option key={record.id} value={record.id}>{record.version} · {record.status === "draft" ? t("common.mediaAuthoring.draft") : record.status === "adopted" ? t("common.mediaAuthoring.adopted") : t("common.mediaAuthoring.history")}</option>)}
          </ThemedSelect>
        </label>}
        <BookButton variant="ghost" size="sm" disabled={loading || busy} onClick={() => void onReload().catch(() => {})}>{t("common.refresh")}</BookButton>
      </div>
    </header>
    {error || actionError ? <p role="alert" className="text-sm text-error">{actionError || error}</p> : null}
    {selected?.stale && <p role="status" className="text-xs text-warning">{t("common.mediaAuthoring.stale")}</p>}
    {kind === "adaptation_plan" && <details className="border-b border-book-border/60 pb-3" open={!editor}>
      <summary className="cursor-pointer text-sm font-medium">{t("common.mediaAuthoring.generationOptions")}</summary>
      <div className="mt-3 grid gap-3 lg:grid-cols-2">
        <PlanTextField label={t("common.mediaAuthoring.instruction")} value={instruction} onChange={setInstruction} />
        <div className="space-y-2">
          <label className="grid gap-1 text-xs text-book-text-sub"><span>{t("common.mediaAuthoring.optionalVideo")}</span>
            <ThemedSelect aria-label={t("common.mediaAuthoring.optionalVideo")} className="w-full min-w-0" value={referenceChoice} disabled={disabled} onChange={setReferenceChoice}>
              <option value="saved">{editor?.record?.inputs.video_reference ? t("common.mediaAuthoring.keepReference") : t("common.mediaAuthoring.independent")}</option>
              {editor?.record?.inputs.video_reference && <option value="independent">{t("common.mediaAuthoring.independent")}</option>}
              {references.map((item) => <option key={item.selection.script_id} value={item.selection.script_id}>{item.project_title} · v{item.selection.revision}{item.same_source_content ? "" : ` · ${t("common.mediaAuthoring.differentSource")}`}</option>)}
            </ThemedSelect>
          </label>
          <p className="text-xs leading-relaxed text-book-text-muted">{t("common.mediaAuthoring.frozenReferenceHint")}</p>
          {referenceError && <p role="alert" className="text-xs text-error">{referenceError}</p>}
        </div>
      </div>
    </details>}
    {editor?.record?.inputs.video_reference && <p className="text-xs text-book-text-muted">{t("common.mediaAuthoring.reference")} · {editor.record.inputs.video_reference.project_title} · v{editor.record.inputs.video_reference.script_revision}
      {selected?.video_reference_status === "updated" && <span className="ml-2 text-warning">{t("common.mediaAuthoring.referenceUpdated")}</span>}
      {selected?.video_reference_status === "unavailable" && <span className="ml-2">{t("common.mediaAuthoring.referenceUnavailable")}</span>}
    </p>}
    <div className="flex flex-wrap items-center gap-2">
      {canEdit && <>
        <BookButton size="sm" variant={editor ? "secondary" : "primary"} disabled={disabled || !workspace || (kind !== "galgame_branch_plan" && !chapter)} onClick={() => void run(generate)}>{busy || stageRunning ? <LoadingSpinner size="sm" /> : null}{editor ? t("common.mediaAuthoring.regeneratePlan") : t("common.mediaAuthoring.generatePlan")}</BookButton>
        {kind === "adaptation_plan" && <BookButton size="sm" variant="ghost" disabled={disabled || !chapter} onClick={() => void run(async () => { const value = await previewMediaAdaptationPlanRequest(generationInput()); setPreview(value); setPreviewOpen(true); })}>{t("common.mediaAuthoring.previewRequest")}</BookButton>}
        {!editor && <BookButton size="sm" variant="secondary" disabled={disabled || !workspace} onClick={() => {
          const document = kind === "galgame_branch_plan" ? { choices: [] } : { schema_version: "adaptation_plan_v1" as const, target: workspace!.target, intention: "", decisions: [], units: [newAdaptationUnit()] };
          setEditor({ record: null, document, baseline: "" });
        }}>{t("common.mediaAuthoring.manualPlan")}</BookButton>}
      </>}
    </div>
    {loading && !workspace ? <LoadingSpinner size="sm" /> : editor && workspace ? <fieldset disabled={disabled || selected?.stale} className="min-w-0 space-y-3 disabled:opacity-70">
      <legend className="sr-only">{title}</legend>
      {kind === "adaptation_plan" && chapter ? <AdaptationPlanEditor value={editor.document as AdaptationPlanV1} chapter={chapter} onChange={(document) => setEditor({ ...editor, document })} />
        : <GalgameBranchPlanEditor value={editor.document as GalgameBranchDesign} workspace={workspace} onChange={(document) => setEditor({ ...editor, document })} />}
    </fieldset> : <p className="py-6 text-sm text-book-text-muted">{t("common.mediaAuthoring.noPlan")}</p>}
    {editor && canEdit && <footer className="sticky bottom-0 z-10 flex flex-wrap items-center justify-end gap-2 border-t border-book-border/60 bg-[rgb(var(--color-bg-primary))] py-3">
      <span className="mr-auto text-xs text-book-text-muted" role="status">{dirty ? t("common.unsavedChangesTitle") : editor.record?.status === "adopted" ? t("common.mediaAuthoring.adopted") : editor.record ? t("common.mediaAuthoring.draftSaved") : ""}</span>
      <BookButton size="sm" variant="secondary" disabled={disabled || !dirty || selected?.stale} onClick={() => void run(async () => { await save(); })}>{t("common.mediaAuthoring.saveDraft")}</BookButton>
      <BookButton size="sm" disabled={disabled || selected?.stale || (!dirty && editor.record?.status === "adopted")} onClick={() => void run(adopt)}>{t("common.mediaAuthoring.adopt")}</BookButton>
      {onNext && <BookButton size="sm" variant="secondary" disabled={disabled || dirty || editor.record?.status !== "adopted" || selected?.stale} onClick={() => { void (async () => { if (await confirmDiscardDrafts()) { onDirtyChange(false); onNext(); } })(); }}>{t("common.mediaAuthoring.nextStage")}</BookButton>}
    </footer>}
    <RequestPreviewModal open={previewOpen} loading={false} data={preview} fallbackTitle={t("common.mediaAuthoring.previewRequest")} onClose={() => setPreviewOpen(false)} />
  </div>;
}
