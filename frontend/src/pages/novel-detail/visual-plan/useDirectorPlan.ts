import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { useToast } from "../../../components/feedback/Toast";
import { applyPlanChanges, describePlanConflicts, inverseChanges, planChanges, receiptChanges, type PlanChange } from "./changes";
import { subscribeVisualPlanEdits, VisualPlanInvalidatedError } from "../../../api/visualPlanEdits";
import {
  confirmChapterVisualPlan, editChapterVisualPlanDraft, getChapterVisualPlan,
  saveChapterVisualPlanDraft, validateChapterVisualPlan,
  type ChapterVisualPlanSetView, type ChapterVisualPlanV3, type ChapterVisualPlanView,
  type VisualPlanEditOperation, type VisualPlanDiagnostic,
} from "../../../api/chapterVisualPlan";

export function useDirectorPlan(projectId: number, chapterId: number | null, canEdit: boolean) {
  const addToast = useToast((state) => state.addToast);
  const [set, setSet] = useState<ChapterVisualPlanSetView | null>(null);
  const [plan, setPlan] = useState<ChapterVisualPlanV3 | null>(null);
  const [record, setRecord] = useState<ChapterVisualPlanView | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recovery, setRecovery] = useState(false);
  const [issues, setIssues] = useState<VisualPlanDiagnostic[]>([]);
  const [undoStack, setUndo] = useState<PlanChange[][]>([]);
  const [redoStack, setRedo] = useState<PlanChange[][]>([]);
  const [conflict, setConflict] = useState<string | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);
  const [invalidated, setInvalidated] = useState(false);
  const sourceHash = useRef("");
  const planRef = useRef(plan);
  const recordRef = useRef(record);
  const baseline = useRef("");
  const scope = useRef(0);
  const scopeKey = useRef("");
  scopeKey.current = `${projectId}:${chapterId}`;
  const pending = useRef<Promise<ChapterVisualPlanView> | null>(null);
  const mutationLock = useRef(false);
  const writeQueue = useRef<Promise<unknown>>(Promise.resolve());
  const conflictRef = useRef<string | null>(null);
  const conflictingChanges = useRef<PlanChange[]>([]);
  const editorRevision = useRef(crypto.randomUUID());
  const viewInstanceId = useRef(crypto.randomUUID());
  const storageKey = `a4n:director-draft:${location.origin}:${projectId}:${chapterId}`;
  const dirty = plan != null && JSON.stringify(plan) !== baseline.current;
  const editable = canEdit && !invalidated && (record == null || record.record.status === "draft");

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
    const message = extractErrorMessage(failure, "导演编排操作失败");
    setError(message);
    addToast(message, "error");
  }, [addToast]);

  const serializeWrite = useCallback(<T,>(work: () => Promise<T>): Promise<T> => {
    const currentScope = scope.current;
    const run = async () => {
      if (currentScope !== scope.current) throw new Error("章节已切换，已取消旧视图写入");
      if (conflictRef.current) throw new Error(conflictRef.current);
      return work();
    };
    const request = writeQueue.current.then(run, run);
    writeQueue.current = request;
    return request;
  }, []);

  const applySaved = useCallback((saved: ChapterVisualPlanView, base: ChapterVisualPlanV3, remember = false) => {
    const changes = planChanges(base, saved.plan);
    // The database commit is real even when newer local input conflicts with it.
    acceptRecord(saved);
    try {
      const next = applyPlanChanges(planRef.current ?? base, changes);
      replace(next);
      if (remember && changes.length > 0) {
        const batch = receiptChanges(saved);
        if (batch.length > 0) setUndo((items) => [...items.slice(-39), batch]);
        setRedo([]);
      }
    } catch (failure) {
      const message = `${extractErrorMessage(failure, "并发修改冲突")}。远端已保存，本地输入已保留，自动保存已暂停。`;
      conflictRef.current = message;
      conflictingChanges.current = changes;
      setConflict(message);
      setError(message);
    }
  }, [acceptRecord, replace]);

  const resolveConflict = useCallback((resolution: "local" | "remote") => {
    if (!planRef.current || !conflictRef.current) return;
    try {
      const next = applyPlanChanges(planRef.current, conflictingChanges.current, resolution);
      const changes = planChanges(planRef.current, next);
      if (changes.length > 0) {
        setUndo((items) => [...items.slice(-39), changes]);
        setRedo([]);
      }
      replace(next);
      conflictRef.current = null;
      conflictingChanges.current = [];
      setConflict(null);
      setError(null);
    } catch (failure) { report(failure); }
  }, [replace, report]);

  const reload = useCallback(async (force = false) => {
    if (chapterId == null) { setLoading(false); return; }
    if (!force && (mutationLock.current || pending.current || (planRef.current && JSON.stringify(planRef.current) !== baseline.current))) {
      report(new Error("当前编辑或保存尚未完成，已保留本地内容。完成后可重新加载方案。"));
      return;
    }
    const currentScope = ++scope.current;
    setLoading(true);
    setError(null);
    try {
      const result = await getChapterVisualPlan(projectId, chapterId);
      if (currentScope !== scope.current) return;
      const current = result.draft ?? result.confirmed;
      conflictRef.current = null;
      conflictingChanges.current = [];
      setConflict(null);
      setInvalidated(false);
      setSyncError(null);
      sourceHash.current = result.source.content_sha256;
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

  const reloadWithBackup = useCallback(async () => {
    try {
      if (planRef.current) sessionStorage.setItem(storageKey, JSON.stringify(planRef.current));
      await reload(true);
    } catch (failure) { report(failure); }
  }, [reload, report, storageKey]);

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
    setConflict(null);
    conflictRef.current = null;
    setInvalidated(false);
    pending.current = null;
    mutationLock.current = false;
    setBusy(false);
    setSaving(false);
    viewInstanceId.current = crypto.randomUUID();
    void reload(true);
    return () => { scope.current += 1; };
  }, [reload]);

  const save = useCallback((): Promise<ChapterVisualPlanView> => serializeWrite(async () => {
    const expectedScope = scope.current;
    if (scopeKey.current !== `${projectId}:${chapterId}`) throw new Error("章节已切换，未写入其他章节");
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
        applySaved(saved, snapshot);
        if (JSON.stringify(planRef.current) === baseline.current) {
          sessionStorage.removeItem(storageKey);
          setRecovery(false);
        }
        setError(null);
      }
      return saved;
    } finally {
      if (pending.current === request) pending.current = null;
      if (currentScope === scope.current) setSaving(false);
    }
  }), [applySaved, canEdit, chapterId, projectId, serializeWrite, storageKey]);

  const saveFromAgent = useCallback((persist: (plan: ChapterVisualPlanV3) => Promise<ChapterVisualPlanView>) => serializeWrite(async () => {
    const snapshot = planRef.current;
    if (!snapshot || !canEdit) throw new Error("当前没有可保存的草稿");
    const currentScope = scope.current;
    setSaving(true);
    const request = persist(snapshot);
    pending.current = request;
    try {
      const saved = await request;
      if (currentScope === scope.current) applySaved(saved, snapshot);
      return saved;
    } finally {
      if (pending.current === request) pending.current = null;
      if (currentScope === scope.current) setSaving(false);
    }
  }), [applySaved, canEdit, serializeWrite]);

  useEffect(() => {
    if (loading || chapterId == null || !planRef.current || conflict || invalidated) return;
    const controller = new AbortController();
    const currentScope = scope.current;
    let cursor = 0;
    let retry: ReturnType<typeof setTimeout> | undefined;
    const connect = async () => {
      try {
        await subscribeVisualPlanEdits(projectId, chapterId, cursor, sourceHash.current, recordRef.current?.record.id ?? null, (saved) => {
          if (controller.signal.aborted || currentScope !== scope.current) return;
          setSyncError(null);
          void serializeWrite(async () => {
            const current = recordRef.current;
            if (sourceHash.current !== saved.record.source_content_sha256
              || (current && current.record.id !== saved.record.id && saved.edit_receipt?.before?.record_id !== current.record.id)) {
              throw new VisualPlanInvalidatedError("编排来源或版本链已变化，已保留本地草稿");
            }
            if (!current || current.record.id !== saved.record.id || current.record.revision < saved.record.revision) {
              applySaved(saved, current?.plan ?? JSON.parse(baseline.current), true);
            }
            cursor = saved.edit_receipt!.sequence;
          }).catch((failure) => {
            if (controller.signal.aborted) return;
            if (failure instanceof VisualPlanInvalidatedError) {
              conflictRef.current = failure.message;
              setInvalidated(true);
            }
            setSyncError(extractErrorMessage(failure, "编辑同步失败"));
          });
        }, controller.signal);
        if (!controller.signal.aborted) throw new Error("编辑同步连接已断开，正在重连");
      } catch (failure) {
        if (!controller.signal.aborted) {
          setSyncError(extractErrorMessage(failure, "编辑同步失败"));
          if (failure instanceof VisualPlanInvalidatedError) {
            conflictRef.current = failure.message;
            setInvalidated(true);
          } else retry = setTimeout(() => void connect(), 1500);
        }
      }
    };
    void connect();
    return () => { controller.abort(); clearTimeout(retry); };
  }, [applySaved, chapterId, conflict, invalidated, loading, projectId, serializeWrite]);

  useEffect(() => {
    if (!dirty || !editable || !plan || conflict || invalidated) return;
    try { sessionStorage.setItem(storageKey, JSON.stringify(plan)); }
    catch (failure) { report(new Error(`本地草稿备份失败：${String(failure)}`)); }
    const timer = setTimeout(() => { if (!mutationLock.current) void save().catch(report); }, 800);
    return () => clearTimeout(timer);
  }, [conflict, dirty, editable, invalidated, plan, report, save, storageKey]);

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
    if (!editable || scopeKey.current !== `${projectId}:${chapterId}`) return;
    if (planRef.current) {
      const changes = planChanges(planRef.current, next);
      if (!changes.length) return;
      setUndo((items) => [...items.slice(-39), changes]);
      setRedo([]);
    }
    // Persist the edit buffer immediately; navigation never depends on an unmount request.
    try { sessionStorage.setItem(storageKey, JSON.stringify(next)); }
    catch (failure) { report(failure); }
    replace(next);
  }, [editable, replace, report, storageKey, projectId, chapterId]);

  const mutate = useCallback(async (operation: VisualPlanEditOperation, operationBase?: ChapterVisualPlanView): Promise<boolean> => {
    if (mutationLock.current || !editable || chapterId == null) {
      const failure = new Error("请等待当前操作完成，并在可编辑草稿中修改");
      report(failure);
      return false;
    }
    mutationLock.current = true;
    setBusy(true);
    const currentScope = scope.current;
    try {
      const saved = await save();
      const result = await serializeWrite(() => editChapterVisualPlanDraft(projectId, chapterId, operationBase ?? saved, operation));
      if (currentScope !== scope.current) return false;
      applySaved(result.visual_plan, saved.plan, result.changed);
      setError(null);
      return true;
    } catch (failure) {
      if (currentScope === scope.current) report(failure);
      return false;
    }
    finally { if (currentScope === scope.current) { mutationLock.current = false; setBusy(false); } }
  }, [applySaved, chapterId, editable, projectId, report, save, serializeWrite]);

  const startEditing = useCallback(async () => {
    if (!canEdit || !planRef.current || chapterId == null) return;
    setBusy(true);
    const currentScope = scope.current;
    try {
      await serializeWrite(async () => {
        const snapshot = planRef.current;
        if (!snapshot) throw new Error("编排已关闭");
        const current = recordRef.current?.record.status === "draft" ? recordRef.current : null;
        const saved = await saveChapterVisualPlanDraft(projectId, chapterId, current, snapshot);
        if (currentScope === scope.current) applySaved(saved, snapshot);
      });
    } catch (failure) { if (currentScope === scope.current) report(failure); }
    finally { if (currentScope === scope.current) setBusy(false); }
  }, [applySaved, canEdit, chapterId, projectId, report, serializeWrite]);

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
      const confirmed = await serializeWrite(() => confirmChapterVisualPlan(projectId, chapterId, saved));
      if (currentScope !== scope.current) return;
      applySaved(confirmed.visual_plan, saved.plan);
      setSet((current) => current ? { ...current, draft: null, confirmed: confirmed.visual_plan } : current);
      setUndo([]);
      setRedo([]);
    } catch (failure) { if (currentScope === scope.current) report(failure); }
    finally { if (currentScope === scope.current) { mutationLock.current = false; setBusy(false); } }
  }, [applySaved, chapterId, projectId, report, save, serializeWrite]);

  const restore = useCallback(async () => {
    if (!canEdit || chapterId == null) return;
    const stored = sessionStorage.getItem(storageKey);
    if (!stored) return;
    setBusy(true);
    const currentScope = scope.current;
    try {
      const recovered: ChapterVisualPlanV3 = JSON.parse(stored);
      const current = recordRef.current?.record.status === "draft" ? recordRef.current : null;
      const snapshot = planRef.current;
      const saved = await serializeWrite(() => saveChapterVisualPlanDraft(projectId, chapterId, current, recovered));
      if (currentScope !== scope.current) return;
      if (snapshot) applySaved(saved, snapshot);
      else { acceptRecord(saved); replace(saved.plan); }
      sessionStorage.removeItem(storageKey);
      setRecovery(false);
      setError(null);
    } catch (failure) { if (currentScope === scope.current) report(failure); }
    finally { if (currentScope === scope.current) setBusy(false); }
  }, [acceptRecord, applySaved, canEdit, chapterId, projectId, replace, report, serializeWrite, storageKey]);

  const undo = () => {
    const previous = undoStack.at(-1);
    if (!previous || !planRef.current || busy || !editable) return;
    try {
      const next = applyPlanChanges(planRef.current, inverseChanges(previous));
      setRedo((items) => [...items, previous]);
      setUndo((items) => items.slice(0, -1));
      replace(next);
    } catch (failure) { report(failure); }
  };
  const redo = () => {
    const next = redoStack.at(-1);
    if (!next || !planRef.current || busy || !editable) return;
    try {
      const plan = applyPlanChanges(planRef.current, next);
      setUndo((items) => [...items.slice(-39), next]);
      setRedo((items) => items.slice(0, -1));
      replace(plan);
    } catch (failure) { report(failure); }
  };

  return { plan, record, set, loading, saving, busy, error, dirty, recovery, editable, issues, conflict, syncError, invalidated,
    conflictDetails: plan && conflict ? describePlanConflicts(plan, conflictingChanges.current) : [],
    updateFields, mutate, save, saveFromAgent, resolveConflict, confirm, startEditing, restore, reload, reloadWithBackup, report, undo, redo,
    canUndo: editable && undoStack.length > 0, canRedo: editable && redoStack.length > 0,
    planRef, recordRef, editorRevision, viewInstanceId, acceptRecord, replace };
}
