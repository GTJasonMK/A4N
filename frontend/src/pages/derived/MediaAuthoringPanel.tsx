import { useEffect, useRef, useState } from "react";
import { Pencil, Save, Trash2, X } from "lucide-react";
import {
  adoptMediaAuthoringDocument, generateMediaAdaptationPlan,
  deleteMediaAuthoringDocument, previewDeleteMediaAuthoringDocument,
  previewMediaAdaptationPlanRequest, saveMediaAuthoringDocument,
  type AdaptationPlanV1, type AuthoringDocumentKind, type AuthoringDocumentRecord,
  type GalgameBranchDesign, type MediaAuthoringWorkspace,
} from "../../api/mediaAuthoring";
import type { GenerateAdaptationPlanInput } from "../../api/generated/host";
import { extractErrorMessage } from "../../api";
import { cancelWorkflowTask } from "../../api/streaming";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { BookButton } from "../../components/ui/BookButton";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { confirmDiscardMediaChanges } from "../../components/feedback/UnsavedChangesGuard";
import { promptDialog } from "../../components/feedback/PromptDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { useProgressTask } from "../../hooks/useProgressTask";
import { useI18n } from "../../i18n";
import { useDirty } from "../../components/navigation/dirtyRegistry";
import { mangaStageLabel } from "../novel-detail/manga/mangaTabUtils";
import { AdaptationPlanEditor, AdaptationPlanView, isValidMangaPageCount, newAdaptationUnit } from "./AdaptationPlanEditor";
import { GalgameBranchPlanEditor } from "./MediaPlanEditors";

type PlanDocument = AdaptationPlanV1 | GalgameBranchDesign;
type BusyAction = "generate" | "preview" | "save" | "adopt" | "delete";

interface Props {
  projectId: number;
  kind: Exclude<AuthoringDocumentKind, "manga_page_plan">;
  workspace: MediaAuthoringWorkspace | null;
  loading: boolean;
  error: string | null;
  chapterId?: number | null;
  canEdit: boolean;
  stageRunning?: boolean;
  stageBlocked?: boolean;
  preconditionError?: string | null;
  onReload: () => Promise<void>;
  onGenerateStage?: () => Promise<void>;
  onDirtyChange: (dirty: boolean) => void;
  onBusyChange?: (busy: boolean) => void;
}

export function MediaAuthoringPanel({ projectId, kind, workspace, loading, error, chapterId: fixedChapterId, canEdit, stageRunning = false, stageBlocked = false, preconditionError = null, onReload, onGenerateStage, onDirtyChange, onBusyChange }: Props) {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask } = useProgressTask();
  const [selectedChapterId, setSelectedChapterId] = useState<number | null>(null);
  const [editor, setEditor] = useState<{ record: AuthoringDocumentRecord | null; document: PlanDocument; baseline: string } | null>(null);
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [busyAction, setBusyAction] = useState<BusyAction | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [preview, setPreview] = useState<ModelRequestPreview | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const busyRef = useRef(false);
  const editingRef = useRef(editing);
  editingRef.current = editing;
  const sourceChapterId = fixedChapterId !== undefined
    ? fixedChapterId
    : selectedChapterId ?? workspace?.chapters[0]?.source.source_chapter_id ?? null;
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
  const selected = current;
  const dirty = editor != null && JSON.stringify(editor.document) !== editor.baseline;
  const isAdaptationPlan = kind === "adaptation_plan";
  const adaptationDocument = editor?.document as AdaptationPlanV1 | undefined;
  const pageCountValid = !isAdaptationPlan || workspace?.target !== "manga"
    || isValidMangaPageCount(adaptationDocument?.page_count);
  const disabled = !canEdit || busy || stageRunning || stageBlocked;
  const branchPlanPrerequisiteMissing = kind === "galgame_branch_plan"
    && workspace != null
    && workspace.chapters.some((chapter) => !workspace.documents.some(({ record, stale }) =>
      !stale
        && record.kind === "adaptation_plan"
        && record.status === "adopted"
        && record.source_snapshot_id === workspace.source_snapshot_id
        && record.source_chapter_id === chapter.source.source_chapter_id,
    ));
  const title = kind === "adaptation_plan"
    ? t("common.mediaAuthoring.adaptation")
    : t("common.mediaAuthoring.branches");

  const discard = () => {
    if (busy) return false;
    setEditor(null);
    setEditing(false);
    onDirtyChange(false);
    return true;
  };
  useDirty(dirtyKey, dirty || busy, discard, () => !busy);
  useEffect(() => { onDirtyChange(dirty || busy); }, [dirty, busy, onDirtyChange]);
  useEffect(() => () => onDirtyChange(false), [onDirtyChange]);
  useEffect(() => { onBusyChange?.(busy); }, [busy, onBusyChange]);
  useEffect(() => () => onBusyChange?.(false), [onBusyChange]);
  useEffect(() => {
    setEditor(null);
    setEditing(false);
    setActionError(null);
    busyRef.current = false;
    setBusy(false);
    setBusyAction(null);
  }, [scope]);
  useEffect(() => {
    if (dirty || editingRef.current) return;
    if (!selected) { setEditor(null); return; }
    setEditor({ record: selected.record, document: selected.record.document as PlanDocument, baseline: JSON.stringify(selected.record.document) });
    // Background refreshes must never overwrite an active or unsaved editor.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scope, selected?.record.id, selected?.record.revision, selected?.record.status]);
  const confirmDiscardDrafts = async () => {
    return confirmDiscardMediaChanges(dirty, t);
  };
  const commitEditor = (record: AuthoringDocumentRecord) => {
    if (scopeRef.current !== scope) return;
    setEditor({ record, document: record.document as PlanDocument, baseline: JSON.stringify(record.document) });
    onDirtyChange(false);
  };
  const run = async (actionKind: BusyAction, action: () => Promise<void>) => {
    if (busyRef.current || disabled) return;
    const requestScope = scope;
    busyRef.current = true;
    setBusyAction(actionKind);
    setBusy(true);
    setActionError(null);
    try { await action(); }
    catch (error) {
      if (scopeRef.current === requestScope) {
        const message = extractErrorMessage(error, t("common.mediaAuthoring.requestFailed"));
        setActionError(message);
      }
    }
    finally {
      if (scopeRef.current === requestScope) {
        busyRef.current = false;
        setBusyAction(null);
        setBusy(false);
      }
    }
  };
  const reloadAfterWrite = async (requestScope: string) => {
    if (scopeRef.current !== requestScope) return;
    try {
      await onReload();
    } catch (error) {
      if (scopeRef.current !== requestScope) return;
      const message = extractErrorMessage(error, t("common.mediaAuthoring.readbackFailed"));
      setActionError(message);
    }
  };
  const save = async () => {
    if (!editor) throw new Error(t("common.mediaAuthoring.noPlan"));
    if (isAdaptationPlan && workspace?.target === "manga" && !pageCountValid) {
      throw new Error(t("common.mediaAuthoring.pageCountRequired"));
    }
    const requestScope = scope;
    const saved = await saveMediaAuthoringDocument({ project_id: projectId, chapter_id: chapterId, kind,
      record_id: editor.record?.id ?? null, expected_revision: editor.record?.revision ?? null, document: editor.document });
    commitEditor(saved);
    void reloadAfterWrite(requestScope);
    return saved;
  };
  const adopt = async () => {
    const requestScope = scope;
    const saved = dirty || !editor?.record ? await save() : editor.record;
    if (scopeRef.current !== requestScope) return;
    const adopted = await adoptMediaAuthoringDocument({ project_id: projectId, record_id: saved.id, expected_revision: saved.revision });
    if (scopeRef.current !== requestScope) return;
    commitEditor(adopted);
    void reloadAfterWrite(requestScope);
  };
  const saveAndAdopt = async () => {
    const requestScope = scope;
    const saved = dirty || !editor?.record ? await save() : editor.record;
    if (scopeRef.current !== requestScope) return;
    const adopted = saved.status === "adopted"
      ? saved
      : await adoptMediaAuthoringDocument({ project_id: projectId, record_id: saved.id, expected_revision: saved.revision });
    if (scopeRef.current !== requestScope) return;
    commitEditor(adopted);
    setEditing(false);
    void reloadAfterWrite(requestScope);
  };
  const enterEdit = () => {
    if (!editor || disabled || selected?.stale) return;
    setEditing(true);
  };
  const cancelEdit = async () => {
    if (!editor || !editing || busy) return;
    const requestScope = scope;
    if (!await confirmDiscardMediaChanges(dirty, t) || scopeRef.current !== requestScope) return;
    setEditor(selected
      ? { record: selected.record, document: selected.record.document as PlanDocument, baseline: JSON.stringify(selected.record.document) }
      : null);
    setEditing(false);
    onDirtyChange(false);
  };
  const remove = async () => {
    if (!isAdaptationPlan || !editor?.record) return;
    const requestScope = scope;
    const record = editor.record;
    const deleteInput = {
      project_id: projectId,
      record_id: record.id,
      kind,
      expected_revision: record.revision,
    };
    const impact = await previewDeleteMediaAuthoringDocument(deleteInput);
    if (scopeRef.current !== requestScope) return;

    const impactLines: string[] = [];
    const chapterLabel = (chapterId: number) => {
      const source = workspace?.chapters.find((item) => item.source.source_chapter_id === chapterId);
      return source ? source.title?.trim() || t("common.mediaAuthoring.chapterNumber", { number: source.chapter_number }) : `#${chapterId}`;
    };
    if (impact.manga_chapter_artifacts.length > 0) {
      impactLines.push(t("common.mediaAuthoring.deleteMangaChapterArtifacts", {
        details: impact.manga_chapter_artifacts.map((artifact) => `${chapterLabel(artifact.chapter_id)} / ${mangaStageLabel(artifact.stage, t)}`).join("、"),
      }));
    }
    if (impact.dependent_documents.length > 0) {
      const labels: Record<AuthoringDocumentKind, string> = {
        adaptation_plan: t("common.mediaAuthoring.adaptation"),
        galgame_branch_plan: t("common.mediaAuthoring.branches"),
        manga_page_plan: t("common.mediaAuthoring.pagePlanning"),
      };
      const details = impact.dependent_documents
        .map(({ id, kind: dependencyKind }) => `${labels[dependencyKind]} #${id}`)
        .join("、");
      impactLines.push(t("common.mediaAuthoring.deleteDependentDocuments", { details }));
    }
    if (impact.manga_pages.length > 0) {
      const pageRefs = impact.manga_pages.map(({ chapter_id, page_number, image_count }) => {
        return t("common.mediaAuthoring.deletePageRef", {
          chapter: chapterLabel(chapter_id),
          page: page_number,
          images: image_count,
        });
      });
      const visiblePageRefs = pageRefs.slice(0, 8);
      if (pageRefs.length > visiblePageRefs.length) {
        visiblePageRefs.push(t("common.mediaAuthoring.deleteMorePages", {
          count: pageRefs.length - visiblePageRefs.length,
        }));
      }
      impactLines.push(t("common.mediaAuthoring.deleteMangaPages", {
        details: visiblePageRefs.join("、"),
      }));
    }
    if (impact.galgame_script) {
      impactLines.push(t("common.mediaAuthoring.deleteGalgameScript", {
        graphs: impact.galgame_script.story_graph_revision_count,
        scenes: impact.galgame_script.scene_count,
        lines: impact.galgame_script.line_count,
        choices: impact.galgame_script.choice_count,
      }));
    }
    const hasDownstream = impactLines.length > 0;
    const confirmed = await confirmDialog({
      title: t(hasDownstream
        ? "common.mediaAuthoring.deleteTitleWithDownstream"
        : "common.mediaAuthoring.deleteTitle"),
      message: [
        t("common.mediaAuthoring.deleteMessage"),
        ...(hasDownstream
          ? [t("common.mediaAuthoring.deleteCascadeHeading"), ...impactLines]
          : []),
      ].join("\n\n"),
      confirmText: t(hasDownstream
        ? "common.mediaAuthoring.deleteCascadeConfirm"
        : "common.delete"),
      dialogType: "danger",
    });
    if (!confirmed || scopeRef.current !== requestScope) return;
    await deleteMediaAuthoringDocument({
      ...deleteInput,
      expected_impact_fingerprint: impact.fingerprint,
    });
    if (scopeRef.current !== requestScope) return;
    setEditor(null);
    setEditing(false);
    onDirtyChange(false);
    addToast(t("common.mediaAuthoring.deleted"), "success");
    void reloadAfterWrite(requestScope);
  };
  const generationInput = (instruction: string): GenerateAdaptationPlanInput => {
    if (!sourceChapterId) throw new Error(t("common.mediaAuthoring.chooseChapter"));
    return {
      project_id: projectId, chapter_id: sourceChapterId, task_id: `media-adaptation-${projectId}-${crypto.randomUUID()}`,
      instruction: instruction.trim() || t("common.mediaAuthoring.defaultInstruction"),
      video_reference: null,
      clear_video_reference: true, record_id: editor?.record?.id ?? null,
      expected_revision: editor?.record?.revision ?? null, resume_from_checkpoint: true,
    };
  };
  const requestGenerationInstruction = () => promptDialog({
    title: t("common.mediaAuthoring.instruction"),
    placeholder: t("common.mediaAuthoring.defaultInstruction"),
    multiline: true,
  });
  const createManualPlan = async () => {
    if (kind === "galgame_branch_plan") {
      setEditor({ record: null, document: { choices: [] }, baseline: "" });
      return;
    }
    const target = workspace?.target;
    if (!target || !sourceChapterId) return;
    const document = {
      schema_version: "adaptation_plan_v1" as const,
      target,
      page_count: null,
      intention: t("common.mediaAuthoring.defaultInstruction"),
      decisions: [],
      units: [newAdaptationUnit()],
    };
    setEditor({ record: null, document, baseline: "" });
    setEditing(true);
  };
  const generate = async () => {
    if (!await confirmDiscardMediaChanges(dirty, t)) return;
    const requestScope = scope;
    if (scopeRef.current !== requestScope) return;
    if (kind !== "adaptation_plan") {
      setEditor(null); onDirtyChange(false);
      await onGenerateStage?.();
      if (scopeRef.current !== requestScope) return;
      return;
    }
    const requestedInstruction = await requestGenerationInstruction();
    if (requestedInstruction == null || scopeRef.current !== requestScope) return;
    const input = generationInput(requestedInstruction);
    const result = await runProgressTask({ title, startMessage: t("common.mediaAuthoring.generating"), successMessage: t("common.mediaAuthoring.draftSaved"), errorMessage: t("common.mediaAuthoring.generationFailed"), total: 1, streaming: true,
      cancelTaskId: input.task_id, cancelTask: cancelWorkflowTask, pollTaskProgress: true,
    }, () => generateMediaAdaptationPlan(input));
    if (scopeRef.current === requestScope) {
      commitEditor(result);
      setEditing(false);
    }
    void reloadAfterWrite(requestScope);
  };

  return <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-1 custom-scrollbar">
    <header className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-book-border/60 pb-3">
      <div className="flex min-w-0 flex-1 basis-48 flex-wrap items-baseline gap-x-2 gap-y-1">
        <h2 className="text-base font-semibold">{title}</h2>
        {chapter && kind !== "galgame_branch_plan" && <span className="min-w-0 break-words text-xs font-medium text-book-text-sub">{t("writing.chapterLabel", { number: chapter.chapter_number })}{chapter.title ? ` · ${chapter.title}` : ""}</span>}
      </div>
      {dirty && <span className="text-xs text-book-text-muted" role="status">{t("common.unsavedChangesTitle")}</span>}
      <div className="flex flex-wrap items-center justify-end gap-2">
        {kind !== "galgame_branch_plan" && fixedChapterId === undefined && <label className="flex items-center gap-2 text-xs text-book-text-sub">
          <span>{t("common.mediaAuthoring.chooseChapter")}</span>
          <ThemedSelect aria-label={t("common.mediaAuthoring.chooseChapter")} className="max-w-full" value={sourceChapterId ?? ""} disabled={busy || stageRunning || stageBlocked} onChange={(value) => { const next = Number(value); void (async () => { if (await confirmDiscardDrafts()) { setEditor(null); setEditing(false); onDirtyChange(false); setSelectedChapterId(next); } })(); }}>
            {workspace?.chapters.map((item) => <option key={item.source.source_chapter_id} value={item.source.source_chapter_id}>{item.chapter_number} · {item.title}</option>)}
          </ThemedSelect>
        </label>}
        <BookButton variant="ghost" size="sm" disabled={loading || busy} onClick={() => void onReload().catch(() => {})}>{t("common.refresh")}</BookButton>
        {canEdit && <>
          {(!isAdaptationPlan || !editing) && <>
            <BookButton size="sm" variant={editor ? "secondary" : "primary"} disabled={disabled || !!preconditionError || !workspace || (kind !== "galgame_branch_plan" && !chapter)} onClick={() => void run("generate", generate)}>{busyAction === "generate" || stageRunning ? <LoadingSpinner size="sm" /> : null}{editor ? t("common.mediaAuthoring.regeneratePlan") : t("common.mediaAuthoring.generatePlan")}</BookButton>
            {isAdaptationPlan && <BookButton size="sm" variant="ghost" disabled={disabled || !!preconditionError || !chapter} onClick={() => void run("preview", async () => {
              const requestScope = scope;
              const instruction = await requestGenerationInstruction();
              if (instruction == null || scopeRef.current !== requestScope) return;
              const value = await previewMediaAdaptationPlanRequest(generationInput(instruction));
              if (scopeRef.current !== requestScope) return;
              setPreview(value);
              setPreviewOpen(true);
            })}>{busyAction === "preview" ? <LoadingSpinner size="sm" /> : null}{t("common.mediaAuthoring.previewRequest")}</BookButton>}
            {!editor && <BookButton size="sm" variant="secondary" disabled={disabled || !!preconditionError || branchPlanPrerequisiteMissing || !workspace || (isAdaptationPlan && !chapter)} onClick={() => void createManualPlan()}>{t("common.mediaAuthoring.manualPlan")}</BookButton>}
          </>}
          {isAdaptationPlan && editor && !editing && <BookButton size="sm" onClick={enterEdit} disabled={disabled || selected?.stale}><Pencil size={13} />{t("shootingScript.edit")}</BookButton>}
          {isAdaptationPlan && editor && editing && <>
            <BookButton size="sm" variant="ghost" onClick={() => void cancelEdit()} disabled={disabled}><X size={13} />{t("shootingScript.cancelEdit")}</BookButton>
            <BookButton size="sm" onClick={() => void run("save", saveAndAdopt)} disabled={disabled || selected?.stale || !pageCountValid || (!dirty && editor.record?.status === "adopted")}>
              {busyAction === "save" ? <LoadingSpinner size="sm" /> : <Save size={13} />}{busyAction === "save" ? t("shootingScript.saving") : t("common.mediaAuthoring.saveDraft")}
            </BookButton>
          </>}
          {isAdaptationPlan && editor?.record && <BookButton size="sm" variant="danger" onClick={() => void run("delete", remove)} disabled={disabled}>
            {busyAction === "delete" ? <LoadingSpinner size="sm" /> : <Trash2 size={13} />}{t("common.delete")}
          </BookButton>}
          {!isAdaptationPlan && editor && <BookButton size="sm" variant="secondary" disabled={disabled || branchPlanPrerequisiteMissing || !dirty || selected?.stale} onClick={() => void run("save", async () => { await save(); })}>{busyAction === "save" ? <LoadingSpinner size="sm" /> : null}{t("common.mediaAuthoring.saveDraft")}</BookButton>}
          {!isAdaptationPlan && editor && <BookButton size="sm" disabled={disabled || branchPlanPrerequisiteMissing || selected?.stale || (!dirty && editor.record?.status === "adopted")} onClick={() => void run("adopt", adopt)}>{busyAction === "adopt" ? <LoadingSpinner size="sm" /> : null}{t("common.mediaAuthoring.adopt")}</BookButton>}
        </>}
      </div>
    </header>
    {error || actionError ? <p role="alert" className="text-sm text-error">{actionError || error}</p> : null}
    {preconditionError ? <p role="status" className="text-sm text-warning">{preconditionError}</p> : null}
    {branchPlanPrerequisiteMissing ? <p role="status" className="text-sm text-warning">{t("common.mediaAuthoring.branchPlanRequiresAdaptations")}</p> : null}
    {selected?.stale && <p role="status" className="text-xs text-warning">{t("common.mediaAuthoring.stale")}</p>}
    {loading && !workspace ? <LoadingSpinner size="sm" /> : editor && workspace ? isAdaptationPlan
      ? chapter ? editing
        ? <fieldset disabled={disabled || selected?.stale} className="min-w-0 space-y-3">
            <legend className="sr-only">{title}</legend>
            <AdaptationPlanEditor value={editor.document as AdaptationPlanV1} chapter={chapter} onChange={(document) => setEditor({ ...editor, document })} />
          </fieldset>
        : <AdaptationPlanView value={editor.document as AdaptationPlanV1} chapter={chapter} />
        : <p className="py-6 text-sm text-book-text-muted">{t("common.mediaAuthoring.chooseChapter")}</p>
      : <fieldset disabled={disabled || selected?.stale} className="min-w-0 space-y-3 disabled:opacity-70">
          <legend className="sr-only">{title}</legend>
          <GalgameBranchPlanEditor value={editor.document as GalgameBranchDesign} workspace={workspace} onChange={(document) => setEditor({ ...editor, document })} />
        </fieldset>
      : <p className="py-6 text-sm text-book-text-muted">{t("common.mediaAuthoring.noPlan")}</p>}
    <RequestPreviewModal open={previewOpen} loading={false} data={preview} fallbackTitle={t("common.mediaAuthoring.previewRequest")} onClose={() => setPreviewOpen(false)} />
  </div>;
}
