import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import {
  confirmChapterVisualPlan, editChapterVisualPlanDraft, getChapterVisualPlan,
  saveChapterVisualPlanDraft, validateChapterVisualPlan,
  type ChapterVisualPlanSetView, type ChapterVisualPlanV3, type ChapterVisualPlanView,
  type VisualPlanEditOperation, type VisualPlanDiagnostic,
} from "../../../api/chapterVisualPlan";

export function useDirectorPlan(projectId: number, chapterId: number | null, canEdit: boolean) {
  const [set, setSet] = useState<ChapterVisualPlanSetView | null>(null);
  const [plan, setPlan] = useState<ChapterVisualPlanV3 | null>(null);
  const [record, setRecord] = useState<ChapterVisualPlanView | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recovery, setRecovery] = useState(false);
  const [issues, setIssues] = useState<VisualPlanDiagnostic[]>([]);
  const [undoStack, setUndo] = useState<ChapterVisualPlanV3[]>([]);
  const [redoStack, setRedo] = useState<ChapterVisualPlanV3[]>([]);
  const planRef = useRef(plan);
  const recordRef = useRef(record);
  const baseline = useRef("");
  const scope = useRef(0);
  const scopeKey = useRef("");
  scopeKey.current = `${projectId}:${chapterId}`;
  const pending = useRef<Promise<ChapterVisualPlanView> | null>(null);
  const mutationLock = useRef(false);
  const editorRevision = useRef(crypto.randomUUID());
  const viewInstanceId = useRef(crypto.randomUUID());
  const storageKey = `a4n:director-draft:${location.origin}:${projectId}:${chapterId}`;
  const dirty = plan != null && JSON.stringify(plan) !== baseline.current;
  const editable = canEdit && (record == null || record.record.status === "draft");

  const replace = useCallback((next: ChapterVisualPlanV3) => {
    if (JSON.stringify(planRef.current) !== JSON.stringify(next)) editorRevision.current = crypto.randomUUID();
    planRef.current = next;
    setPlan(next);
    setIssues([]);
  }, []);

  const acceptRecord = useCallback((next: ChapterVisualPlanView) => {
    recordRef.current = next;
    baseline.current = JSON.stringify(next.plan);
    setRecord(next);
  }, []);

  const report = useCallback((failure: unknown) => {
    setError(extractErrorMessage(failure, "导演编排操作失败"));
  }, []);

  const reload = useCallback(async (force = false) => {
    if (chapterId == null) { setLoading(false); return; }
    if (!force && (mutationLock.current || pending.current || (planRef.current && JSON.stringify(planRef.current) !== baseline.current))) {
      setError("当前编辑或保存尚未完成，已保留本地内容。完成后可重新加载方案。");
      return;
    }
    const currentScope = ++scope.current;
    setLoading(true);
    setError(null);
    try {
      const result = await getChapterVisualPlan(projectId, chapterId);
      if (currentScope !== scope.current) return;
      const current = result.draft ?? result.confirmed;
      setSet(result);
      recordRef.current = current;
      setRecord(current);
      const next = current?.plan ?? result.starter_plan;
      baseline.current = JSON.stringify(next);
      replace(next);
      setUndo([]);
      setRedo([]);
      setRecovery(sessionStorage.getItem(storageKey) != null);
    } catch (failure) {
      if (currentScope === scope.current) report(failure);
    } finally {
      if (currentScope === scope.current) setLoading(false);
    }
  }, [chapterId, projectId, replace, report, storageKey]);

  useEffect(() => {
    planRef.current = null;
    recordRef.current = null;
    setPlan(null);
    setRecord(null);
    setSet(null);
    setUndo([]);
    setRedo([]);
    setIssues([]);
    setRecovery(false);
    pending.current = null;
    mutationLock.current = false;
    setBusy(false);
    setSaving(false);
    viewInstanceId.current = crypto.randomUUID();
    void reload(true);
    return () => { scope.current += 1; };
  }, [reload]);

  const save = useCallback(async (): Promise<ChapterVisualPlanView> => {
    const expectedScope = scope.current;
    if (scopeKey.current !== `${projectId}:${chapterId}`) throw new Error("章节已切换，未写入其他章节");
    if (pending.current) await pending.current;
    if (scope.current !== expectedScope || scopeKey.current !== `${projectId}:${chapterId}`) throw new Error("章节已切换，请在原章节重新保存");
    const snapshot = planRef.current;
    const current = recordRef.current;
    if (!canEdit || chapterId == null || !snapshot || current?.record.status === "confirmed") {
      throw new Error("请先进入草稿编辑模式");
    }
    if (current && JSON.stringify(snapshot) === baseline.current) return current;
    const currentScope = scope.current;
    setSaving(true);
    const request = saveChapterVisualPlanDraft(projectId, chapterId, current, snapshot);
    pending.current = request;
    try {
      const saved = await request;
      if (currentScope === scope.current) {
        acceptRecord(saved);
        if (planRef.current === snapshot) {
          replace(saved.plan);
          sessionStorage.removeItem(storageKey);
          setRecovery(false);
        }
        setError(null);
      }
      return saved;
    } catch (failure) {
      if (currentScope === scope.current) report(failure);
      throw failure;
    } finally {
      if (pending.current === request) pending.current = null;
      if (currentScope === scope.current) setSaving(false);
    }
  }, [acceptRecord, canEdit, chapterId, projectId, replace, report, storageKey]);

  useEffect(() => {
    if (!dirty || !editable || !plan) return;
    try { sessionStorage.setItem(storageKey, JSON.stringify(plan)); }
    catch (failure) { report(new Error(`本地草稿备份失败：${String(failure)}`)); }
    const timer = setTimeout(() => { if (!mutationLock.current) void save().catch(report); }, 800);
    return () => clearTimeout(timer);
  }, [dirty, editable, plan, report, save, storageKey]);

  useEffect(() => {
    const guard = (event: BeforeUnloadEvent) => {
      if (planRef.current && JSON.stringify(planRef.current) !== baseline.current) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", guard);
    return () => window.removeEventListener("beforeunload", guard);
  }, []);

  const updateFields = useCallback((next: ChapterVisualPlanV3) => {
    if (!editable || mutationLock.current || scopeKey.current !== `${projectId}:${chapterId}`) return;
    if (planRef.current && JSON.stringify(planRef.current) === baseline.current) {
      setUndo((items) => [...items.slice(-39), planRef.current!]);
      setRedo([]);
    }
    // Persist the edit buffer immediately; navigation never depends on an unmount request.
    try { sessionStorage.setItem(storageKey, JSON.stringify(next)); }
    catch (failure) { report(failure); }
    replace(next);
  }, [editable, replace, report, storageKey, projectId, chapterId]);

  const mutate = useCallback(async (operation: VisualPlanEditOperation) => {
    if (mutationLock.current || !editable || chapterId == null) throw new Error("请等待当前操作完成，并在可编辑草稿中修改");
    mutationLock.current = true;
    setBusy(true);
    const currentScope = scope.current;
    try {
      const saved = await save();
      const result = await editChapterVisualPlanDraft(projectId, chapterId, saved, operation);
      if (currentScope !== scope.current) return;
      if (result.changed) {
        setUndo((items) => [...items.slice(-39), saved.plan]);
        setRedo([]);
      }
      acceptRecord(result.visual_plan);
      replace(result.visual_plan.plan);
      setError(null);
    } catch (failure) { if (currentScope === scope.current) report(failure); throw failure; }
    finally { if (currentScope === scope.current) { mutationLock.current = false; setBusy(false); } }
  }, [acceptRecord, chapterId, editable, projectId, replace, report, save]);

  const startEditing = useCallback(async () => {
    if (!canEdit || !planRef.current || chapterId == null) return;
    setBusy(true);
    const currentScope = scope.current;
    try {
      const saved = await saveChapterVisualPlanDraft(projectId, chapterId, null, planRef.current);
      if (currentScope !== scope.current) return;
      acceptRecord(saved);
      replace(saved.plan);
    } catch (failure) { if (currentScope === scope.current) report(failure); }
    finally { if (currentScope === scope.current) setBusy(false); }
  }, [acceptRecord, canEdit, chapterId, projectId, replace, report]);

  const confirm = useCallback(async () => {
    if (chapterId == null || mutationLock.current) return;
    mutationLock.current = true;
    setBusy(true);
    const currentScope = scope.current;
    try {
      const saved = await save();
      const result = await validateChapterVisualPlan(projectId, chapterId, saved);
      if (currentScope !== scope.current) return;
      setIssues(result.validation.diagnostics);
      if (!result.validation.valid) return;
      const confirmed = await confirmChapterVisualPlan(projectId, chapterId, saved);
      if (currentScope !== scope.current) return;
      acceptRecord(confirmed.visual_plan);
      replace(confirmed.visual_plan.plan);
      setSet((current) => current ? { ...current, draft: null, confirmed: confirmed.visual_plan } : current);
      setUndo([]);
      setRedo([]);
    } catch (failure) { if (currentScope === scope.current) report(failure); }
    finally { if (currentScope === scope.current) { mutationLock.current = false; setBusy(false); } }
  }, [acceptRecord, chapterId, projectId, replace, report, save]);

  const restore = useCallback(async () => {
    if (!canEdit || chapterId == null) return;
    const stored = sessionStorage.getItem(storageKey);
    if (!stored) return;
    setBusy(true);
    const currentScope = scope.current;
    try {
      const recovered: ChapterVisualPlanV3 = JSON.parse(stored);
      const current = recordRef.current?.record.status === "draft" ? recordRef.current : null;
      const saved = await saveChapterVisualPlanDraft(projectId, chapterId, current, recovered);
      if (currentScope !== scope.current) return;
      acceptRecord(saved);
      replace(saved.plan);
      sessionStorage.removeItem(storageKey);
      setRecovery(false);
      setError(null);
    } catch (failure) { if (currentScope === scope.current) report(failure); }
    finally { if (currentScope === scope.current) setBusy(false); }
  }, [acceptRecord, canEdit, chapterId, projectId, replace, report, storageKey]);

  const undo = () => {
    const previous = undoStack.at(-1);
    if (!previous || !planRef.current || busy || !editable) return;
    setRedo((items) => [...items, planRef.current!]);
    setUndo((items) => items.slice(0, -1));
    replace(previous);
  };
  const redo = () => {
    const next = redoStack.at(-1);
    if (!next || !planRef.current || busy || !editable) return;
    setUndo((items) => [...items, planRef.current!]);
    setRedo((items) => items.slice(0, -1));
    replace(next);
  };

  return { plan, record, set, loading, saving, busy, error, dirty, recovery, editable, issues,
    updateFields, mutate, save, confirm, startEditing, restore, reload, report, undo, redo,
    canUndo: editable && undoStack.length > 0, canRedo: editable && redoStack.length > 0,
    planRef, recordRef, editorRevision, viewInstanceId, acceptRecord, replace };
}
