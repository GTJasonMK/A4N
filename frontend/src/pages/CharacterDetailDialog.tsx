import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, LocateFixed, Pencil, RotateCcw, Sparkles } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { Modal } from "../components/ui/Modal";
import { BookButton } from "../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../components/ui/Dropdown";
import { useToast } from "../components/feedback/Toast";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { RequestPreviewModal } from "../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../api";
import {
  checkCharacterConflict,
  getCharacterArcStatus,
  getCharacterVoiceFingerprint,
  previewCheckCharacterConflictRequest,
  previewSyncCharacterFromChaptersRequest,
  syncCharacterFromChapters,
  type CharacterConflictSummary,
  type CharacterConflictScope,
  type CharacterProfile,
} from "../api/character";
import type { ModelRequestPreview } from "../api/requestPreview";
import { cancelWorkflowTask } from "../api/streaming";
import { createWorkflowTaskId, useProgressTask } from "../hooks/useProgressTask";
import { isCancelledOperation } from "../utils/cancellation";
import { CharacterDetailContent } from "./character-detail-dialog/CharacterDetailContent";
import { CharacterDetailTabs } from "./character-detail-dialog/CharacterDetailTabs";
import { CharacterSummaryCard } from "./character-detail-dialog/CharacterSummaryCard";
import {
  CHARACTER_DETAIL_DRAFT_SECTIONS,
  characterDetailDraftKey,
} from "./character-detail-dialog/manageViewUtils";
import type {
  CharacterDetailInitialEventFilter,
  CharacterDetailTab,
} from "./character-detail-dialog/characterDetailUtils";
import { useI18n, type MessageTranslator } from "../i18n";
import { useDirtyRegistry, useDirtyRegistryVersion } from "../components/navigation/dirtyRegistry";

export type { CharacterDetailInitialEventFilter } from "./character-detail-dialog/characterDetailUtils";

/**
 * 角色详情对话框
 *
 * 暴露 character 域的深度分析能力：
 *  - getCharacterArcStatus(project_id, name) —— 角色弧光状态
 *  - getCharacterVoiceFingerprint(project_id, name) —— 声音指纹
 *  - checkCharacterConflict(project_id, name, task_id, scope) —— LLM 检测档案与章节的矛盾
 *  - syncCharacterFromChapters(project_id, name, chapterNumber) —— 从章节同步角色变化到档案
 *
 * 各子视图实现在 `./character-detail-dialog/`。
 */

interface CharacterDetailDialogProps {
  open: boolean;
  onClose: () => void;
  projectId: number;
  character: CharacterProfile | null;
  currentChapterNumber?: number | null;
  initialTab?: CharacterDetailTab;
  initialEventFilter?: CharacterDetailInitialEventFilter | null;
  onChanged?: () => boolean | void | Promise<boolean | void>;
  onEditInitial?: (character: CharacterProfile) => void;
}

interface CharacterCacheValue<T> {
  key: string;
  value: T;
}

interface ConflictScopeDraft {
  startChapter: string;
  endChapter: string;
}

const emptyConflictScopeDraft = (): ConflictScopeDraft => ({
  startChapter: "",
  endChapter: "",
});

const conflictScopeKey = (draft: ConflictScopeDraft): string => {
  const start = draft.startChapter.trim() || "*";
  const end = draft.endChapter.trim() || "*";
  return `${start}:${end}`;
};

const conflictScopeFromDraft = (
  draft: ConflictScopeDraft,
  t: MessageTranslator,
): CharacterConflictScope => ({
  start_chapter: optionalChapterInput(draft.startChapter, t("characterDetail.startChapter"), t),
  end_chapter: optionalChapterInput(draft.endChapter, t("characterDetail.endChapter"), t),
});

const optionalChapterInput = (
  value: string,
  label: string,
  t: MessageTranslator,
): number | null => {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const numeric = Number(trimmed);
  if (!Number.isInteger(numeric)) {
    throw new Error(t("characterDetail.invalidChapterInteger", { label }));
  }
  return numeric;
};

const ConflictScopeControls: React.FC<{
  draft: ConflictScopeDraft;
  activeChapterNumber: number | null;
  disabled: boolean;
  onChange: (draft: ConflictScopeDraft) => void;
  onUseCurrentChapter: () => void;
  onClear: () => void;
}> = ({
  draft,
  activeChapterNumber,
  disabled,
  onChange,
  onUseCurrentChapter,
  onClear,
}) => {
  const { t } = useI18n();
  return (
  <div className="flex flex-col gap-2 border-b border-book-border/45 pb-3">
    <div className="flex flex-wrap items-end gap-2">
      <span className="text-small font-semibold text-book-text-muted">{t("characterDetail.checkScope")}</span>
      <label className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
        <span>{t("characterDetail.startChapter")}</span>
        <input
          type="number"
          min={1}
          step={1}
          inputMode="numeric"
          value={draft.startChapter}
          disabled={disabled}
          onChange={(event) =>
            onChange({ ...draft, startChapter: event.currentTarget.value })
          }
          className="pyqt-input h-7 w-20 !px-2 !py-1 text-xs"
        />
      </label>
      <label className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
        <span>{t("characterDetail.endChapter")}</span>
        <input
          type="number"
          min={1}
          step={1}
          inputMode="numeric"
          value={draft.endChapter}
          disabled={disabled}
          onChange={(event) =>
            onChange({ ...draft, endChapter: event.currentTarget.value })
          }
          className="pyqt-input h-7 w-20 !px-2 !py-1 text-xs"
        />
      </label>
      <BookButton
        variant="secondary"
        size="sm"
        onClick={onUseCurrentChapter}
        disabled={disabled || activeChapterNumber == null}
      >
        <span className="inline-flex items-center gap-1.5">
          <LocateFixed size={13} /> {t("characterDetail.currentChapter")}
        </span>
      </BookButton>
      <BookButton variant="secondary" size="sm" onClick={onClear} disabled={disabled}>
        <span className="inline-flex items-center gap-1.5">
          <RotateCcw size={13} /> {t("characterDetail.wholeProject")}
        </span>
      </BookButton>
    </div>
  </div>
  );
};

export const CharacterDetailDialog: React.FC<CharacterDetailDialogProps> = ({
  open,
  onClose,
  projectId,
  character,
  currentChapterNumber,
  initialTab,
  initialEventFilter,
  onChanged,
  onEditInitial,
}) => {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const registry = useDirtyRegistry();
  useDirtyRegistryVersion();
  const [tab, setTab] = useState<CharacterDetailTab>("arc");
  const [arc, setArc] = useState<CharacterCacheValue<unknown> | null>(null);
  const [voice, setVoice] = useState<CharacterCacheValue<unknown> | null>(null);
  const [arcLoadError, setArcLoadError] = useState<CharacterCacheValue<string> | null>(null);
  const [voiceLoadError, setVoiceLoadError] = useState<CharacterCacheValue<string> | null>(null);
  const [conflicts, setConflicts] =
    useState<CharacterCacheValue<CharacterConflictSummary[]> | null>(null);
  const [conflictScopeDraft, setConflictScopeDraft] = useState<ConflictScopeDraft>(
    emptyConflictScopeDraft,
  );
  const [loadingTab, setLoadingTab] = useState<CharacterDetailTab | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const { runProgressTask } = useProgressTask();

  const name = character?.name;
  const characterCacheKey =
    character && name ? `${projectId}:${character.id}:${name}` : null;
  const currentArcCache = characterCacheKey && arc?.key === characterCacheKey ? arc : null;
  const currentVoiceCache = characterCacheKey && voice?.key === characterCacheKey ? voice : null;
  const currentArc = currentArcCache?.value ?? null;
  const currentVoice = currentVoiceCache?.value ?? null;
  const currentArcLoadError =
    characterCacheKey && arcLoadError?.key === characterCacheKey ? arcLoadError.value : null;
  const currentVoiceLoadError =
    characterCacheKey && voiceLoadError?.key === characterCacheKey ? voiceLoadError.value : null;
  const activeConflictScopeKey = conflictScopeKey(conflictScopeDraft);
  const conflictCacheKey = characterCacheKey
    ? `${characterCacheKey}:conflict:${activeConflictScopeKey}`
    : null;
  const currentConflicts =
    conflictCacheKey && conflicts?.key === conflictCacheKey ? conflicts.value : null;
  const activeChapterNumber =
    currentChapterNumber && currentChapterNumber > 0 ? currentChapterNumber : null;
  const activeCharacterScope = characterCacheKey;
  const activeDialogScope = open && characterCacheKey ? characterCacheKey : null;
  const mountedRef = useRef(true);
  const activeCharacterScopeRef = useRef<string | null>(activeCharacterScope);
  const syncIdentityRef = useRef<string | null>(activeCharacterScope);
  const activeDialogScopeRef = useRef<string | null>(activeDialogScope);
  const activeTabRef = useRef<CharacterDetailTab>(tab);
  const activeChapterNumberRef = useRef<number | null>(activeChapterNumber);
  const refreshSeqRef = useRef(0);
  const syncSeqRef = useRef(0);
  const requestPreviewSeqRef = useRef(0);
  const refreshingRef = useRef(false);
  const syncingRef = useRef(false);

  const refreshParentAfterMutation = useCallback(async (): Promise<string | null> => {
    try {
      return await onChanged?.() === false ? t("characterDetail.refreshFailed") : null;
    } catch (error) {
      return extractErrorMessage(error, t("characterDetail.refreshFailed"));
    }
  }, [onChanged, t]);

  const dirtyManageKeys = useCallback(() => {
    if (!character) return [];
    return CHARACTER_DETAIL_DRAFT_SECTIONS
      .map((section) => characterDetailDraftKey(character.id, section))
      .filter((key) => registry.hasDirty(key));
  }, [character, registry]);

  const confirmManageDraftAction = useCallback(async (action: string): Promise<boolean> => {
    const keys = dirtyManageKeys();
    if (keys.length === 0) return true;
    if (!keys.every((key) => registry.canDiscardDirty(key))) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    const ok = await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("characterDetail.manage"),
        action,
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    });
    if (!ok) return false;
    for (const key of keys) {
      if (!await registry.discardDirty(key)) return false;
    }
    return true;
  }, [addToast, dirtyManageKeys, registry, t]);

  const handleDialogClose = async () => {
    if (await confirmManageDraftAction(t("common.close"))) onClose();
  };

  const handleTabChange = async (nextTab: CharacterDetailTab) => {
    if (nextTab === tab) return;
    if (!(await confirmManageDraftAction(t("common.switchTab")))) return;
    setTab(nextTab);
  };

  if (activeCharacterScope) {
    activeCharacterScopeRef.current = activeCharacterScope;
  }
  activeDialogScopeRef.current = activeDialogScope;
  activeTabRef.current = tab;
  activeChapterNumberRef.current = activeChapterNumber;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      syncSeqRef.current += 1;
      requestPreviewSeqRef.current += 1;
      refreshingRef.current = false;
      syncingRef.current = false;
    };
  }, []);

  const isCurrentDialogScope = useCallback((scope: string | null) => {
    return mountedRef.current && scope != null && activeDialogScopeRef.current === scope;
  }, []);

  const isCurrentCharacterScope = useCallback((scope: string | null) => {
    return mountedRef.current && scope != null && activeCharacterScopeRef.current === scope;
  }, []);

  useEffect(() => {
    refreshSeqRef.current += 1;
    requestPreviewSeqRef.current += 1;
    setArc(null);
    setVoice(null);
    setArcLoadError(null);
    setVoiceLoadError(null);
    setConflicts(null);
    setConflictScopeDraft(emptyConflictScopeDraft());
    setLoadingTab(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    refreshingRef.current = false;
    if (!open || !characterCacheKey) {
      setTab("arc");
      return;
    }
    setTab(initialTab ?? "arc");
  }, [open, characterCacheKey, initialTab]);

  useEffect(() => {
    if (!characterCacheKey || syncIdentityRef.current === characterCacheKey) return;
    syncIdentityRef.current = characterCacheKey;
    syncSeqRef.current += 1;
    setSyncing(false);
    syncingRef.current = false;
  }, [characterCacheKey]);

  useEffect(() => {
    if (open && characterCacheKey && initialTab) {
      setTab(initialTab);
    }
  }, [open, characterCacheKey, initialTab, initialEventFilter?.category, initialEventFilter?.key]);

  useEffect(() => {
    if (!open || !name || !characterCacheKey) return;
    if (tab === "arc" && currentArcCache != null) return;
    if (tab === "voice" && currentVoiceCache != null) return;
    if (tab !== "arc" && tab !== "voice") return;

    let cancelled = false;
    const cacheKey = characterCacheKey;

    const loadCurrentTab = async () => {
      setLoadingTab(tab);
      try {
        if (tab === "arc") {
          const data = await getCharacterArcStatus(projectId, name);
          if (!cancelled) {
            setArc({ key: cacheKey, value: data });
            setArcLoadError(null);
          }
        } else {
          const data = await getCharacterVoiceFingerprint(projectId, name);
          if (!cancelled) {
            setVoice({ key: cacheKey, value: data });
            setVoiceLoadError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          const message = extractErrorMessage(err, t("characterDetail.loadFailed"));
          if (tab === "arc") setArcLoadError({ key: cacheKey, value: message });
          else setVoiceLoadError({ key: cacheKey, value: message });
        }
      } finally {
        if (!cancelled) setLoadingTab(null);
      }
    };

    void loadCurrentTab();

    return () => {
      cancelled = true;
      if (mountedRef.current) {
        setLoadingTab((current) => (current === tab ? null : current));
      }
    };
  }, [addToast, characterCacheKey, currentArcCache, currentVoiceCache, name, open, projectId, tab, t]);

  const handleRefresh = async () => {
    if (!name || !characterCacheKey || tab === "manage") return;
    if (refreshingRef.current) return;
    refreshingRef.current = true;
    const requestScope = characterCacheKey;
    const requestTab = tab;
    const requestName = name;
    const requestProjectId = projectId;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    if (tab === "conflict") {
      if (!conflictCacheKey) {
        refreshingRef.current = false;
        return;
      }
      let requestConflictScope: CharacterConflictScope;
      try {
        requestConflictScope = conflictScopeFromDraft(conflictScopeDraft, t);
      } catch (err) {
        refreshingRef.current = false;
        addToast(extractErrorMessage(err, t("characterDetail.invalidChapterRange")), "error");
        return;
      }
      const cacheKey = conflictCacheKey;
      const taskId = createWorkflowTaskId("character-conflict", requestProjectId);
      setLoadingTab("conflict");
      try {
        const rows = await runProgressTask(
          {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
            title: t("characterDetail.consistencyCheck"),
            startMessage: t("characterDetail.preparingConsistencyCheck"),
            successMessage: t("characterDetail.consistencyCheckComplete"),
            errorMessage: t("characterDetail.consistencyCheckFailed"),
            total: 4,
            cancelLabel: t("characterDetail.stopCheck"),
            runningHint: t("characterDetail.taskDataKept"),
          },
          () => {
            if (!isCurrentDialogScope(requestScope)) {
              return Promise.reject(new Error(t("characterDetail.taskStopped")));
            }
            return checkCharacterConflict(
              requestProjectId,
              requestName,
              taskId,
              requestConflictScope,
            );
          },
        );
        if (
          !isCurrentDialogScope(requestScope) ||
          refreshSeqRef.current !== requestSeq ||
          activeTabRef.current !== requestTab
        ) {
          return;
        }
        setConflicts({ key: cacheKey, value: rows });
      } catch (err) {
        if (!isCurrentDialogScope(requestScope) || refreshSeqRef.current !== requestSeq) return;
        const message = extractErrorMessage(err, t("characterDetail.consistencyCheckFailed"));
        addToast(message, isCancelledOperation(message) ? "info" : "error");
      } finally {
        if (isCurrentDialogScope(requestScope) && refreshSeqRef.current === requestSeq) {
          setLoadingTab((current) => (current === requestTab ? null : current));
        }
        if (refreshSeqRef.current === requestSeq) {
          refreshingRef.current = false;
        }
      }
      return;
    }
    setLoadingTab(tab);
    try {
      const cacheKey = characterCacheKey;
      switch (tab) {
        case "arc":
          {
            const data = await getCharacterArcStatus(requestProjectId, requestName);
            if (
              !isCurrentDialogScope(requestScope) ||
              refreshSeqRef.current !== requestSeq ||
              activeTabRef.current !== requestTab
            ) {
              return;
            }
            setArc({ key: cacheKey, value: data });
            setArcLoadError(null);
          }
          break;
        case "voice":
          {
            const data = await getCharacterVoiceFingerprint(requestProjectId, requestName);
            if (
              !isCurrentDialogScope(requestScope) ||
              refreshSeqRef.current !== requestSeq ||
              activeTabRef.current !== requestTab
            ) {
              return;
            }
            setVoice({
              key: cacheKey,
              value: data,
            });
            setVoiceLoadError(null);
          }
          break;
      }
    } catch (err) {
      if (!isCurrentDialogScope(requestScope) || refreshSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("characterDetail.refreshFailed"));
      if (requestTab === "arc") setArcLoadError({ key: requestScope, value: message });
      else if (requestTab === "voice") setVoiceLoadError({ key: requestScope, value: message });
    } finally {
      if (isCurrentDialogScope(requestScope) && refreshSeqRef.current === requestSeq) {
        setLoadingTab((current) => (current === requestTab ? null : current));
      }
      if (refreshSeqRef.current === requestSeq) {
        refreshingRef.current = false;
      }
    }
  };

  const handleSync = async () => {
    if (!name || !characterCacheKey) return;
    if (syncingRef.current) return;
    if (tab === "manage" && !(await confirmManageDraftAction(t("characterDetail.syncFromCurrentChapter")))) return;
    syncingRef.current = true;
    const requestScope = characterCacheKey;
    const requestName = name;
    const requestProjectId = projectId;
    const requestChapterNumber = activeChapterNumber;
    const cacheKey = characterCacheKey;
    if (!requestChapterNumber) {
      addToast(t("characterDetail.chooseChapterToSync"), "error");
      syncingRef.current = false;
      return;
    }
    const ok = await confirmDialog({
      title: t("characterDetail.syncTitle"),
      message: t("characterDetail.syncMessage", {
        name: requestName,
        chapter: requestChapterNumber,
      }),
      confirmText: t("characterDetail.startSync"),
      dialogType: "warning",
    });
    if (!ok) {
      syncingRef.current = false;
      return;
    }
    if (
      !isCurrentCharacterScope(requestScope) ||
      activeChapterNumberRef.current !== requestChapterNumber
    ) {
      syncingRef.current = false;
      return;
    }
    const requestSeq = syncSeqRef.current + 1;
    syncSeqRef.current = requestSeq;
    const taskId = createWorkflowTaskId("character-sync", requestProjectId);
    setSyncing(true);
    try {
      const result = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: t("characterDetail.syncTitleWithName", { name: requestName }),
          startMessage: t("characterDetail.preparingSync"),
          successMessage: t("characterDetail.syncComplete"),
          errorMessage: t("characterDetail.syncFailed"),
          cancelLabel: t("characterDetail.stopSync"),
          runningHint: t("characterDetail.taskDataKept"),
        },
        () => {
          if (!isCurrentCharacterScope(requestScope)) {
            return Promise.reject(new Error(t("characterDetail.taskStopped")));
          }
          return syncCharacterFromChapters(
            requestProjectId,
            requestName,
            requestChapterNumber,
            taskId,
          );
        },
      );
      if (!isCurrentCharacterScope(requestScope) || syncSeqRef.current !== requestSeq) return;
      const parentRefreshError = await refreshParentAfterMutation();
      if (result.status === "skipped") {
        addToast(t("characterDetail.noSyncNeeded"), "info");
      } else {
        const ragIssues: string[] = [];
        const stateRagError = result.state_rag_error?.trim();
        if (stateRagError) {
          ragIssues.push(t("characterDetail.stateRagIssue", { error: stateRagError }));
        } else if (!result.state_rag_indexed) {
          ragIssues.push(t("characterDetail.stateRagNotIndexed"));
        }
        const attributeEventRagError = result.attribute_event_rag_error?.trim();
        if (attributeEventRagError) {
          ragIssues.push(t("characterDetail.attributeEventRagIssue", { error: attributeEventRagError }));
        } else if (!result.attribute_event_rag_indexed) {
          ragIssues.push(t("characterDetail.attributeEventRagNotIndexed"));
        }
        const ragIssue = ragIssues.length > 0 ? ragIssues.join(t("common.summarySeparator")) : null;
        const rewindNotice =
          result.cleared_through_chapter > result.synced_chapter
            ? t("characterDetail.rewindNotice", {
                from: result.cleared_through_chapter,
                to: result.synced_chapter,
                count:
                  result.cleared_attribute_events +
                  result.cleared_behaviors +
                  result.cleared_state_indexes +
                  result.cleared_snapshots,
              })
            : null;
        if (ragIssue) {
          addToast(
            t("characterDetail.syncCompleteWithRagIssue", {
              issue: ragIssue,
              notice: rewindNotice ? ` ${rewindNotice}` : "",
            }),
            "error",
          );
        } else if (rewindNotice) {
          addToast(t("characterDetail.syncCompleteWithNotice", { notice: rewindNotice }), "info");
        } else {
          addToast(t("characterDetail.syncSuccess"), "success");
        }
      }
      if (parentRefreshError) addToast(parentRefreshError, "error");
      if (tab !== "arc") {
        setArc(null);
        setArcLoadError(null);
      }
      if (tab !== "voice") {
        setVoice(null);
        setVoiceLoadError(null);
      }
      setConflicts(null);
      if (!isCurrentCharacterScope(requestScope) || syncSeqRef.current !== requestSeq) return;
      if (!isCurrentDialogScope(requestScope)) return;
      if (tab === "arc") {
        try {
          const data = await getCharacterArcStatus(requestProjectId, requestName);
          if (!isCurrentDialogScope(requestScope) || syncSeqRef.current !== requestSeq) return;
          setArc({
            key: cacheKey,
            value: data,
          });
          setArcLoadError(null);
        } catch (err) {
          if (!isCurrentDialogScope(requestScope) || syncSeqRef.current !== requestSeq) return;
          const message = extractErrorMessage(err, t("characterDetail.refreshArcFailed"));
          setArcLoadError({ key: cacheKey, value: message });
          addToast(message, "error");
        }
      } else if (tab === "voice") {
        try {
          const data = await getCharacterVoiceFingerprint(requestProjectId, requestName);
          if (!isCurrentDialogScope(requestScope) || syncSeqRef.current !== requestSeq) return;
          setVoice({
            key: cacheKey,
            value: data,
          });
          setVoiceLoadError(null);
        } catch (err) {
          if (!isCurrentDialogScope(requestScope) || syncSeqRef.current !== requestSeq) return;
          const message = extractErrorMessage(err, t("characterDetail.refreshVoiceFailed"));
          setVoiceLoadError({ key: cacheKey, value: message });
          addToast(message, "error");
        }
      }
    } catch (err) {
      if (!isCurrentCharacterScope(requestScope) || syncSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("characterDetail.syncFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
      const parentRefreshError = await refreshParentAfterMutation();
      if (parentRefreshError) addToast(parentRefreshError, "error");
    } finally {
      if (isCurrentCharacterScope(requestScope) && syncSeqRef.current === requestSeq) {
        setSyncing(false);
      }
      if (syncSeqRef.current === requestSeq) {
        syncingRef.current = false;
      }
    }
  };

  const openRequestPreview = async (
    scope: string,
    loader: () => Promise<ModelRequestPreview>,
  ) => {
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const data = await loader();
      if (!isCurrentDialogScope(scope) || requestPreviewSeqRef.current !== requestSeq) return;
      setRequestPreviewData(data);
    } catch (err) {
      if (!isCurrentDialogScope(scope) || requestPreviewSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.previewRequestFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentDialogScope(scope) && requestPreviewSeqRef.current === requestSeq) {
        setRequestPreviewLoading(false);
      }
    }
  };

  const handlePreviewConflict = async () => {
    if (!name || !characterCacheKey) return;
    const requestScope = characterCacheKey;
    const requestName = name;
    const requestProjectId = projectId;
    let requestConflictScope: CharacterConflictScope;
    try {
      requestConflictScope = conflictScopeFromDraft(conflictScopeDraft, t);
    } catch (err) {
      addToast(extractErrorMessage(err, t("characterDetail.invalidChapterRange")), "error");
      return;
    }
    await openRequestPreview(requestScope, () =>
      previewCheckCharacterConflictRequest(
        requestProjectId,
        requestName,
        requestConflictScope,
      ),
    );
  };

  const setConflictScopeToCurrentChapter = () => {
    if (!activeChapterNumber) {
      addToast(t("characterDetail.chooseCurrentChapter"), "error");
      return;
    }
    const value = String(activeChapterNumber);
    setConflicts(null);
    setConflictScopeDraft({ startChapter: value, endChapter: value });
  };

  const clearConflictScope = () => {
    setConflicts(null);
    setConflictScopeDraft(emptyConflictScopeDraft());
  };

  const handlePreviewSync = async () => {
    if (!name || !characterCacheKey) return;
    const requestScope = characterCacheKey;
    const requestName = name;
    const requestProjectId = projectId;
    const requestChapterNumber = activeChapterNumber;
    if (!requestChapterNumber) {
      addToast(t("characterDetail.chooseChapterToSync"), "error");
      return;
    }
    await openRequestPreview(requestScope, () =>
      previewSyncCharacterFromChaptersRequest(
        requestProjectId,
        requestName,
        requestChapterNumber,
      ),
    );
  };

  if (!character) {
    return null;
  }

  const tabActionItems: DropdownItem[] = [];
  if (onEditInitial && character.initial_profile_editable) {
    tabActionItems.push({
      label: t("characterDetail.editInitialProfile"),
      onClick: () => {
        void (async () => {
          if (await confirmManageDraftAction(t("common.close"))) onEditInitial(character);
        })();
      },
      icon: <Pencil size={12} />,
    });
  }
  if (tab === "manage" && activeChapterNumber != null) {
    tabActionItems.push({
      label: syncing ? t("characterDetail.syncingFromChapter") : t("characterDetail.syncFromCurrentChapter"),
      onClick: () => void handleSync(),
      icon: syncing ? <LoadingSpinner size="sm" /> : <Sparkles size={12} />,
      disabled: syncing || syncingRef.current,
    });
  }
  const requestPreviewItems: DropdownItem[] = [];
  if (tab === "conflict") {
    requestPreviewItems.push({
      label: t("characterDetail.consistencyRequest"),
      onClick: handlePreviewConflict,
      icon: <Eye size={12} />,
    });
  }
  if (activeChapterNumber != null) {
    requestPreviewItems.push({
      label: t("characterDetail.chapterSyncRequest"),
      onClick: handlePreviewSync,
      icon: <Eye size={12} />,
    });
  }
  if (tabActionItems.length > 0 && requestPreviewItems.length > 0) {
    tabActionItems.push({ type: "divider" });
  }
  tabActionItems.push(...requestPreviewItems);

  return (
    <>
      <Modal
        isOpen={open}
        onClose={() => void handleDialogClose()}
        title={character.name}
        maxWidthClassName="max-w-3xl"
        footer={activeChapterNumber != null && tab !== "manage" ? (
          <BookButton
            onClick={handleSync}
            disabled={syncing || syncingRef.current}
          >
            {syncing ? (
              <span className="inline-flex items-center gap-2">
                <LoadingSpinner size="sm" /> {t("characterDetail.syncing")}
              </span>
            ) : (
              <span className="inline-flex items-center gap-2">
                <Sparkles size={14} /> {t("characterDetail.syncFromChapter")}
              </span>
            )}
          </BookButton>
        ) : undefined}
      >
        <div className="flex flex-col gap-2.5">
          <CharacterSummaryCard character={character} />

          <CharacterDetailTabs
            activeTab={tab}
            loadingTab={loadingTab}
            conflictCount={currentConflicts?.length ?? null}
            onTabChange={(nextTab) => void handleTabChange(nextTab)}
            onRefresh={handleRefresh}
            actions={
              tabActionItems.length > 0 ? (
                <Dropdown items={tabActionItems} />
              ) : null
            }
          />

          {tab === "conflict" ? (
            <ConflictScopeControls
              draft={conflictScopeDraft}
              activeChapterNumber={activeChapterNumber}
              disabled={loadingTab === "conflict" || refreshingRef.current}
              onChange={(next) => {
                setConflicts(null);
                setConflictScopeDraft(next);
              }}
              onUseCurrentChapter={setConflictScopeToCurrentChapter}
              onClear={clearConflictScope}
            />
          ) : null}

          <div>
            {tab === "arc" && currentArcLoadError ? (
              <p role="alert" className="mb-2 text-small text-error">{currentArcLoadError}</p>
            ) : null}
            {tab === "voice" && currentVoiceLoadError ? (
              <p role="alert" className="mb-2 text-small text-error">{currentVoiceLoadError}</p>
            ) : null}
            <CharacterDetailContent
              tab={tab}
              loadingTab={loadingTab}
              arc={currentArc}
              arcLoaded={currentArcCache != null}
              arcLoadError={currentArcLoadError}
              voice={currentVoice}
              voiceLoaded={currentVoiceCache != null}
              voiceLoadError={currentVoiceLoadError}
              conflicts={currentConflicts}
              character={character}
              initialManageSection={initialTab === "manage" ? "event" : undefined}
              initialEventFilter={initialEventFilter}
              onChanged={onChanged}
              onRefresh={handleRefresh}
            />
          </div>
        </div>
      </Modal>

      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("characterDetail.modelRequest")}
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          setRequestPreviewOpen(false);
        }}
      />
    </>
  );
};
