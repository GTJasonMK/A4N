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
import type {
  CharacterDetailInitialEventFilter,
  CharacterDetailTab,
} from "./character-detail-dialog/characterDetailUtils";

export type { CharacterDetailInitialEventFilter } from "./character-detail-dialog/characterDetailUtils";

/**
 * 角色详情对话框
 *
 * 暴露 character 域的深度分析能力：
 *  - getCharacterArcStatus(project_id, name) —— 角色弧光状态
 *  - getCharacterVoiceFingerprint(project_id, name) —— 声音指纹
 *  - checkCharacterConflict(project_id, name, scope) —— LLM 检测档案与章节的矛盾
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
  onChanged?: () => void | Promise<void>;
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

const conflictScopeFromDraft = (draft: ConflictScopeDraft): CharacterConflictScope => ({
  startChapter: optionalChapterInput(draft.startChapter, "起始章节"),
  endChapter: optionalChapterInput(draft.endChapter, "结束章节"),
});

const optionalChapterInput = (value: string, label: string): number | null => {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const numeric = Number(trimmed);
  if (!Number.isInteger(numeric)) {
    throw new Error(`${label}必须是整数`);
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
}) => (
  <div className="flex flex-col gap-2 border-b border-book-border/45 pb-3">
    <div className="flex flex-wrap items-end gap-2">
      <span className="text-small font-semibold text-book-text-muted">检查范围</span>
      <label className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
        <span>起始章</span>
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
        <span>结束章</span>
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
          <LocateFixed size={13} /> 当前章
        </span>
      </BookButton>
      <BookButton variant="secondary" size="sm" onClick={onClear} disabled={disabled}>
        <span className="inline-flex items-center gap-1.5">
          <RotateCcw size={13} /> 全项目
        </span>
      </BookButton>
    </div>
  </div>
);

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
  const addToast = useToast((state) => state.addToast);
  const [tab, setTab] = useState<CharacterDetailTab>("arc");
  const [arc, setArc] = useState<CharacterCacheValue<unknown> | null>(null);
  const [voice, setVoice] = useState<CharacterCacheValue<unknown> | null>(null);
  const [conflicts, setConflicts] = useState<CharacterCacheValue<unknown[]> | null>(null);
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
  const currentArc = characterCacheKey && arc?.key === characterCacheKey ? arc.value : null;
  const currentVoice =
    characterCacheKey && voice?.key === characterCacheKey ? voice.value : null;
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
    if (tab === "arc" && currentArc != null) return;
    if (tab === "voice" && currentVoice != null) return;
    if (tab !== "arc" && tab !== "voice") return;

    let cancelled = false;
    const cacheKey = characterCacheKey;

    const loadCurrentTab = async () => {
      setLoadingTab(tab);
      try {
        if (tab === "arc") {
          const data = await getCharacterArcStatus(projectId, name);
          if (!cancelled) setArc({ key: cacheKey, value: data });
        } else {
          const data = await getCharacterVoiceFingerprint(projectId, name);
          if (!cancelled) setVoice({ key: cacheKey, value: data });
        }
      } catch (err) {
        if (!cancelled) addToast(extractErrorMessage(err, "加载失败"), "error");
      } finally {
        if (!cancelled) setLoadingTab(null);
      }
    };

    void loadCurrentTab();

    return () => {
      cancelled = true;
    };
  }, [addToast, characterCacheKey, currentArc, currentVoice, name, open, projectId, tab]);

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
        requestConflictScope = conflictScopeFromDraft(conflictScopeDraft);
      } catch (err) {
        refreshingRef.current = false;
        addToast(extractErrorMessage(err, "章节范围无效"), "error");
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
            title: "角色一致性检查",
            startMessage: "正在准备角色一致性检查",
            successMessage: "角色一致性检查完成",
            errorMessage: "角色一致性检查失败",
            total: 4,
            cancelLabel: "停止检查",
            runningHint: "角色任务执行过程中已完成的数据会保留",
          },
          () => {
            if (!isCurrentDialogScope(requestScope)) {
              return Promise.reject(new Error("任务已停止"));
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
        const message = extractErrorMessage(err, "角色一致性检查失败");
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
    switch (tab) {
      case "arc":
        setArc(null);
        break;
      case "voice":
        setVoice(null);
        break;
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
          }
          break;
      }
    } catch (err) {
      if (!isCurrentDialogScope(requestScope) || refreshSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "刷新失败"), "error");
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
    syncingRef.current = true;
    const requestScope = characterCacheKey;
    const requestName = name;
    const requestProjectId = projectId;
    const requestChapterNumber = activeChapterNumber;
    const cacheKey = characterCacheKey;
    if (!requestChapterNumber) {
      addToast("请先选择要同步的章节", "error");
      syncingRef.current = false;
      return;
    }
    const targetText = `同步到第 ${requestChapterNumber} 章`;
    const ok = await confirmDialog({
      title: "同步角色档案",
      message: `将「${requestName}」的角色档案${targetText}？\n\n会从上次同步断点开始分析章节，并写入事件溯源、行为记录、删除候选、状态索引和快照。`,
      confirmText: "开始同步",
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
          title: `同步角色 · ${requestName}`,
          startMessage: "正在准备角色同步",
          successMessage: "角色同步完成",
          errorMessage: "同步失败",
          cancelLabel: "停止同步",
          runningHint: "角色任务执行过程中已完成的数据会保留",
        },
        () => {
          if (!isCurrentCharacterScope(requestScope)) {
            return Promise.reject(new Error("任务已停止"));
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
      if (result.status === "skipped") {
        addToast("角色档案无需同步", "info");
      } else if (result.state_rag_error || !result.state_rag_indexed) {
        const ragError = result.state_rag_error?.trim();
        addToast(
          `同步完成，但状态 RAG 未完全更新：${
            ragError ? ragError : "后端未返回错误详情"
          }`,
          "error",
        );
      } else {
        addToast("同步完成", "success");
      }
      setArc(null);
      setVoice(null);
      setConflicts(null);
      await onChanged?.();
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
        } catch (err) {
          if (!isCurrentDialogScope(requestScope) || syncSeqRef.current !== requestSeq) return;
          addToast(extractErrorMessage(err, "同步完成，但刷新弧光失败"), "error");
        }
      } else if (tab === "voice") {
        try {
          const data = await getCharacterVoiceFingerprint(requestProjectId, requestName);
          if (!isCurrentDialogScope(requestScope) || syncSeqRef.current !== requestSeq) return;
          setVoice({
            key: cacheKey,
            value: data,
          });
        } catch (err) {
          if (!isCurrentDialogScope(requestScope) || syncSeqRef.current !== requestSeq) return;
          addToast(extractErrorMessage(err, "同步完成，但刷新声音指纹失败"), "error");
        }
      }
    } catch (err) {
      if (!isCurrentCharacterScope(requestScope) || syncSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, "同步失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
      await onChanged?.();
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
      addToast(extractErrorMessage(err, "预览角色请求失败"), "error");
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
      requestConflictScope = conflictScopeFromDraft(conflictScopeDraft);
    } catch (err) {
      addToast(extractErrorMessage(err, "章节范围无效"), "error");
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
      addToast("请先选择当前章节", "error");
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
      addToast("请先选择要同步的章节", "error");
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
      label: "编辑初始档案",
      onClick: () => onEditInitial(character),
      icon: <Pencil size={12} />,
    });
  }
  if (tab === "manage" && activeChapterNumber != null) {
    tabActionItems.push({
      label: syncing ? "正在从章节同步" : "从当前章节同步",
      onClick: () => void handleSync(),
      icon: syncing ? <LoadingSpinner size="sm" /> : <Sparkles size={12} />,
      disabled: syncing || syncingRef.current,
    });
  }
  const requestPreviewItems: DropdownItem[] = [];
  if (tab === "conflict") {
    requestPreviewItems.push({
      label: "一致性检查请求",
      onClick: handlePreviewConflict,
      icon: <Eye size={12} />,
    });
  }
  if (activeChapterNumber != null) {
    requestPreviewItems.push({
      label: "章节同步请求",
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
        onClose={onClose}
        title={character.name}
        maxWidthClassName="max-w-3xl"
        footer={activeChapterNumber != null && tab !== "manage" ? (
          <BookButton
            onClick={handleSync}
            disabled={syncing || syncingRef.current}
          >
            {syncing ? (
              <span className="inline-flex items-center gap-2">
                <LoadingSpinner size="sm" /> 同步中…
              </span>
            ) : (
              <span className="inline-flex items-center gap-2">
                <Sparkles size={14} /> 从章节同步
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
            onTabChange={setTab}
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
            <CharacterDetailContent
              tab={tab}
              loadingTab={loadingTab}
              arc={currentArc}
              voice={currentVoice}
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
        fallbackTitle="角色模型请求"
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          setRequestPreviewOpen(false);
        }}
      />
    </>
  );
};
