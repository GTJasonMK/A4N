import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../api";
import {
  promptList,
  promptListCategories,
  promptLoadDefaults,
  promptReset,
  promptUpdate,
  type CategoryInfo,
  type PromptEntry,
} from "../../api/prompt";
import { useAuthStore } from "../../store/auth";
import { PromptEditorPanel } from "./prompt/PromptEditorPanel";
import { PromptList } from "./prompt/PromptList";
import { PromptCategoryTabs, PromptRegistryHeader } from "./prompt/PromptNavigation";
import { PromptToolbar } from "./prompt/PromptToolbar";
import { filterPrompts } from "./prompt/promptTabUtils";

export const PromptTab: React.FC = () => {
  const { addToast } = useToast();
  const { user } = useAuthStore();
  const [items, setItems] = useState<PromptEntry[]>([]);
  const [categories, setCategories] = useState<CategoryInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [activeCategory, setActiveCategory] = useState<string>("all");
  const [keyword, setKeyword] = useState("");
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [draft, setDraft] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [reloadingDefaults, setReloadingDefaults] = useState(false);
  const mountedRef = useRef(true);
  const dirtyRef = useRef(false);
  const editVersionRef = useRef(0);
  const refreshSeqRef = useRef(0);
  const saveSeqRef = useRef(0);
  const resetSeqRef = useRef(0);
  const defaultsSeqRef = useRef(0);
  const refreshEntryRef = useRef(false);
  const refreshBusyRef = useRef(false);
  const saveBusyRef = useRef(false);
  const resetBusyRef = useRef(false);
  const defaultsBusyRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      saveSeqRef.current += 1;
      resetSeqRef.current += 1;
      defaultsSeqRef.current += 1;
      refreshEntryRef.current = false;
      refreshBusyRef.current = false;
      saveBusyRef.current = false;
      resetBusyRef.current = false;
      defaultsBusyRef.current = false;
    };
  }, []);

  const markClean = useCallback(() => {
    dirtyRef.current = false;
    setDirty(false);
  }, []);

  const markDirty = useCallback(() => {
    editVersionRef.current += 1;
    dirtyRef.current = true;
    setDirty(true);
  }, []);

  const refresh = useCallback(async () => {
    if (refreshBusyRef.current) return;
    const requestSeq = refreshSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    refreshSeqRef.current = requestSeq;
    refreshBusyRef.current = true;
    setLoading(true);
    try {
      const [list, cats] = await Promise.all([promptList(), promptListCategories()]);
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      if (dirtyRef.current && editVersionRef.current !== requestEditVersion) {
        addToast("已保留未保存的提示词草稿，刷新结果未覆盖当前编辑", "info");
        return;
      }
      setItems(list);
      setCategories(cats);
      setSelectedName((current) =>
        current && !list.some((prompt) => prompt.name === current) ? null : current,
      );
    } catch (err) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "加载提示词失败"), "error");
    } finally {
      if (refreshSeqRef.current === requestSeq) refreshBusyRef.current = false;
      if (mountedRef.current && refreshSeqRef.current === requestSeq) setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const filtered = useMemo(
    () =>
      filterPrompts(items, {
        activeCategory,
        keyword,
      }),
    [items, activeCategory, keyword],
  );

  const selected = useMemo(
    () => items.find((p) => p.name === selectedName) ?? null,
    [items, selectedName],
  );

  useEffect(() => {
    if (dirtyRef.current) return;
    if (selected) {
      setDraft(selected.content);
      markClean();
    } else {
      setDraft("");
      markClean();
    }
  }, [markClean, selectedName, selected?.content]);

  const discardDraftToSavedState = useCallback(() => {
    editVersionRef.current += 1;
    if (selected) {
      setDraft(selected.content);
    } else {
      setDraft("");
    }
    markClean();
  }, [markClean, selected]);

  const confirmDiscardDraft = useCallback(
    async (actionText: string): Promise<boolean> => {
      if (!dirty) return true;
      return confirmDialog({
        title: "未保存修改",
        message: `当前提示词有未保存修改，${actionText}会放弃当前草稿。是否继续？`,
        confirmText: "放弃草稿",
        dialogType: "warning",
      });
    },
    [dirty],
  );

  const handleSelectPrompt = async (name: string) => {
    if (name === selectedName) return;
    if (!(await confirmDiscardDraft("切换提示词"))) return;
    discardDraftToSavedState();
    if (!mountedRef.current) return;
    setSelectedName(name);
  };

  const handleRefresh = async () => {
    if (
      refreshEntryRef.current ||
      refreshBusyRef.current ||
      saveBusyRef.current ||
      resetBusyRef.current ||
      defaultsBusyRef.current
    ) {
      return;
    }
    refreshEntryRef.current = true;
    try {
      if (!(await confirmDiscardDraft("刷新列表"))) return;
      if (dirtyRef.current) discardDraftToSavedState();
      if (!mountedRef.current) return;
      await refresh();
    } finally {
      refreshEntryRef.current = false;
    }
  };

  const handleSave = async () => {
    if (
      refreshEntryRef.current ||
      refreshBusyRef.current ||
      saveBusyRef.current ||
      resetBusyRef.current ||
      defaultsBusyRef.current
    ) {
      return;
    }
    if (!selected) return;
    if (!user) {
      addToast("请先登录后再编辑提示词", "error");
      return;
    }
    if (!dirty) return;
    const requestSeq = saveSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    const requestName = selected.name;
    const requestDraft = draft;
    saveSeqRef.current = requestSeq;
    saveBusyRef.current = true;
    setSaving(true);
    try {
      await promptUpdate({
        name: requestName,
        content: requestDraft,
        author: user.username,
      });
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      if (editVersionRef.current !== requestEditVersion) {
        addToast(`已保存提交时的提示词：${requestName}，当前保留后续编辑草稿`, "info");
        return;
      }
      markClean();
      addToast(`已保存：${requestName}`, "success");
      await refresh();
    } catch (err) {
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "保存失败"), "error");
    } finally {
      if (saveSeqRef.current === requestSeq) saveBusyRef.current = false;
      if (mountedRef.current && saveSeqRef.current === requestSeq) setSaving(false);
    }
  };

  const handleReset = async () => {
    if (
      refreshEntryRef.current ||
      refreshBusyRef.current ||
      saveBusyRef.current ||
      resetBusyRef.current ||
      defaultsBusyRef.current
    ) {
      return;
    }
    if (!selected) return;
    const requestName = selected.name;
    resetBusyRef.current = true;
    const ok = await confirmDialog({
      title: "重置提示词",
      message: `将提示词 "${requestName}" 重置为默认内容？所有自定义将丢失。`,
      confirmText: "重置",
      dialogType: "warning",
    });
    if (!ok) {
      resetBusyRef.current = false;
      return;
    }
    if (!mountedRef.current) {
      resetBusyRef.current = false;
      return;
    }
    const requestSeq = resetSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    resetSeqRef.current = requestSeq;
    setResetting(true);
    try {
      await promptReset(requestName);
      if (!mountedRef.current || resetSeqRef.current !== requestSeq) return;
      if (editVersionRef.current !== requestEditVersion) {
        addToast(`已重置提交时的提示词：${requestName}，当前保留后续编辑草稿`, "info");
        return;
      }
      markClean();
      addToast(`已重置：${requestName}`, "success");
      await refresh();
    } catch (err) {
      if (!mountedRef.current || resetSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "重置失败"), "error");
    } finally {
      if (resetSeqRef.current === requestSeq) resetBusyRef.current = false;
      if (mountedRef.current && resetSeqRef.current === requestSeq) setResetting(false);
    }
  };

  const handleLoadDefaults = async () => {
    if (
      refreshEntryRef.current ||
      refreshBusyRef.current ||
      saveBusyRef.current ||
      resetBusyRef.current ||
      defaultsBusyRef.current
    ) {
      return;
    }
    defaultsBusyRef.current = true;
    if (!(await confirmDiscardDraft("载入默认模板"))) {
      defaultsBusyRef.current = false;
      return;
    }
    if (!mountedRef.current) {
      defaultsBusyRef.current = false;
      return;
    }
    const ok = await confirmDialog({
      title: "载入默认提示词",
      message: "从 prompts/ 目录重新载入全部默认模板？只补全缺失项，不会覆盖你已经修改的提示词。",
      confirmText: "重新载入",
      dialogType: "warning",
    });
    if (!ok) {
      defaultsBusyRef.current = false;
      return;
    }
    if (!mountedRef.current) {
      defaultsBusyRef.current = false;
      return;
    }
    if (dirtyRef.current) discardDraftToSavedState();
    const requestSeq = defaultsSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    defaultsSeqRef.current = requestSeq;
    setReloadingDefaults(true);
    try {
      const loadedCount = await promptLoadDefaults();
      if (!mountedRef.current || defaultsSeqRef.current !== requestSeq) return;
      const successMessage =
        loadedCount > 0
          ? `默认提示词已重新载入，补全 ${loadedCount} 个缺失模板`
          : "默认提示词已重新载入，没有新增缺失模板";
      if (dirtyRef.current && editVersionRef.current !== requestEditVersion) {
        addToast(`${successMessage}，当前保留后续编辑草稿`, "info");
        return;
      }
      markClean();
      addToast(successMessage, "success");
      await refresh();
    } catch (err) {
      if (!mountedRef.current || defaultsSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "载入默认失败"), "error");
    } finally {
      if (defaultsSeqRef.current === requestSeq) defaultsBusyRef.current = false;
      if (mountedRef.current && defaultsSeqRef.current === requestSeq) setReloadingDefaults(false);
    }
  };

  return (
    <div className="h-full flex flex-col gap-3 min-h-0">
      <PromptRegistryHeader
        categories={categories}
        totalCount={items.length}
        loading={loading}
      />

      <PromptCategoryTabs
        categories={categories}
        activeCategory={activeCategory}
        filteredCount={filtered.length}
        onCategoryChange={setActiveCategory}
      />

      <PromptToolbar
        keyword={keyword}
        loading={loading}
        reloadingDefaults={reloadingDefaults}
        onKeywordChange={setKeyword}
        onLoadDefaults={() => void handleLoadDefaults()}
        onRefresh={() => void handleRefresh()}
      />

      <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-3 md:grid-cols-[280px_1fr] md:grid-rows-1">
        <PromptList
          prompts={filtered}
          loading={loading}
          selectedName={selectedName}
          onSelect={(name) => void handleSelectPrompt(name)}
        />

        <PromptEditorPanel
          prompt={selected}
          draft={draft}
          dirty={dirty}
          saving={saving}
          resetting={resetting}
          onDraftChange={(nextDraft) => {
            markDirty();
            setDraft(nextDraft);
            if (nextDraft === selected?.content) {
              markClean();
            }
          }}
          onSave={() => void handleSave()}
          onReset={() => void handleReset()}
        />
      </div>
    </div>
  );
};
