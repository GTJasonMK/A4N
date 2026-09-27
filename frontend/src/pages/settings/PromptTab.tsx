import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
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
import { PromptEditorPanel } from "./prompt/PromptEditorPanel";
import { PromptList } from "./prompt/PromptList";
import { PromptCategoryTabs } from "./prompt/PromptNavigation";
import { PromptToolbar } from "./prompt/PromptToolbar";
import { filterPrompts } from "./prompt/promptTabUtils";
import { useI18n } from "../../i18n";

export const PromptTab: React.FC = () => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [items, setItems] = useState<PromptEntry[]>([]);
  const [categories, setCategories] = useState<CategoryInfo[]>([]);
  const [loadErrors, setLoadErrors] = useState<string[]>([]);
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

  const refresh = useCallback(async (): Promise<boolean> => {
    if (refreshBusyRef.current) return false;
    const requestSeq = refreshSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    refreshSeqRef.current = requestSeq;
    refreshBusyRef.current = true;
    setLoading(true);
    setLoadErrors([]);
    try {
      const [listResult, categoriesResult] = await Promise.allSettled([
        promptList(),
        promptListCategories(),
      ]);
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return false;
      if (dirtyRef.current && editVersionRef.current !== requestEditVersion) {
        addToast(t("settingsPrompt.draftKept"), "info");
        return false;
      }
      const failures: string[] = [];
      if (listResult.status === "fulfilled") {
        setItems(listResult.value);
        setSelectedName((current) =>
          current && listResult.value.some((prompt) => prompt.name === current)
            ? current
            : listResult.value[0]?.name ?? null,
        );
      } else {
        failures.push(extractErrorMessage(listResult.reason, t("settingsPrompt.loadFailed")));
      }
      if (categoriesResult.status === "fulfilled") {
        setCategories(categoriesResult.value);
      } else {
        failures.push(extractErrorMessage(categoriesResult.reason, t("settingsPrompt.loadFailed")));
      }
      setLoadErrors(failures);
      failures.forEach((message) => addToast(message, "error"));
      return failures.length === 0;
    } finally {
      if (refreshSeqRef.current === requestSeq) refreshBusyRef.current = false;
      if (mountedRef.current && refreshSeqRef.current === requestSeq)
        setLoading(false);
    }
  }, [addToast, t]);

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

  const handleCategoryChange = async (nextCategory: string): Promise<boolean> => {
    if (nextCategory === activeCategory) return true;
    if (!(await confirmDiscardDraft(t("settingsPrompt.category")))) return false;
    if (!mountedRef.current) return false;
    if (dirtyRef.current) discardDraftToSavedState();
    const nextFiltered = filterPrompts(items, {
      activeCategory: nextCategory,
      keyword,
    });
    if (selectedName && !nextFiltered.some((prompt) => prompt.name === selectedName)) {
      setSelectedName(nextFiltered[0]?.name ?? null);
    }
    setActiveCategory(nextCategory);
    return true;
  };

  const selected = useMemo(
    () => items.find((p) => p.name === selectedName) ?? null,
    [items, selectedName],
  );
  const reverseDependencies = useMemo(
    () =>
      selectedName
        ? items
            .filter((prompt) => prompt.dependencies.includes(selectedName))
            .map((prompt) => prompt.name)
        : [],
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
        title: t("common.unsavedChangesTitle"),
        message: t("common.unsavedChangesMessage", { name: t("settingsPrompt.prompt"), action: actionText }),
        confirmText: t("common.discardDraft"),
        dialogType: "warning",
      });
    },
    [dirty, t],
  );

  const handleSelectPrompt = async (name: string) => {
    if (name === selectedName) return;
    if (!(await confirmDiscardDraft(t("settingsPrompt.switchPrompt")))) return;
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
      if (!(await confirmDiscardDraft(t("settingsPrompt.refreshList")))) return;
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
    if (!dirty) return;
    const requestSeq = saveSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    const requestName = selected.name;
    const requestLabel = selected.title || t("settingsPrompt.prompt");
    const requestDraft = draft;
    saveSeqRef.current = requestSeq;
    saveBusyRef.current = true;
    setSaving(true);
    try {
      await promptUpdate({
        name: requestName,
        content: requestDraft,
      });
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      if (editVersionRef.current !== requestEditVersion) {
        addToast(
          t("settingsPrompt.savedSubmittedDraft", { name: requestLabel }),
          "info",
        );
        return;
      }
      const refreshed = await refresh();
      if (!refreshed) return;
      markClean();
      addToast(t("settingsPrompt.saved", { name: requestLabel }), "success");
    } catch (err) {
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("settings.saveFailed")), "error");
    } finally {
      if (saveSeqRef.current === requestSeq) saveBusyRef.current = false;
      if (mountedRef.current && saveSeqRef.current === requestSeq)
        setSaving(false);
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
    const requestLabel = selected.title || t("settingsPrompt.prompt");
    resetBusyRef.current = true;
    const ok = await confirmDialog({
      title: t("settingsPrompt.resetTitle"),
      message: t("settingsPrompt.resetMessage", { name: requestLabel }),
      confirmText: t("common.reset"),
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
        addToast(
          t("settingsPrompt.restoredSubmittedDraft", { name: requestLabel }),
          "info",
        );
        return;
      }
      const refreshed = await refresh();
      if (!refreshed) return;
      markClean();
      addToast(t("settingsPrompt.restored", { name: requestLabel }), "success");
    } catch (err) {
      if (!mountedRef.current || resetSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("settingsPrompt.resetFailed")), "error");
    } finally {
      if (resetSeqRef.current === requestSeq) resetBusyRef.current = false;
      if (mountedRef.current && resetSeqRef.current === requestSeq)
        setResetting(false);
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
    if (!(await confirmDiscardDraft(t("settingsPrompt.loadDefaults")))) {
      defaultsBusyRef.current = false;
      return;
    }
    if (!mountedRef.current) {
      defaultsBusyRef.current = false;
      return;
    }
    const ok = await confirmDialog({
      title: t("settingsPrompt.loadDefaultsTitle"),
      message: t("settingsPrompt.loadDefaultsMessage"),
      confirmText: t("settingsPrompt.reloadDefaults"),
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
          ? t("settingsPrompt.defaultsLoaded", { count: loadedCount })
          : t("settingsPrompt.defaultsAlreadyLoaded");
      if (dirtyRef.current && editVersionRef.current !== requestEditVersion) {
        addToast(t("settingsPrompt.keepDraft", { message: successMessage }), "info");
        return;
      }
      const refreshed = await refresh();
      if (!refreshed) return;
      markClean();
      addToast(successMessage, "success");
    } catch (err) {
      if (!mountedRef.current || defaultsSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("settingsPrompt.loadDefaultsFailed")), "error");
    } finally {
      if (defaultsSeqRef.current === requestSeq)
        defaultsBusyRef.current = false;
      if (mountedRef.current && defaultsSeqRef.current === requestSeq)
        setReloadingDefaults(false);
    }
  };

  return (
    <div
      data-settings-dirty={dirty ? "true" : undefined}
      className="flex h-auto min-h-0 flex-col gap-3 md:h-full"
    >
      <PromptCategoryTabs
        categories={categories}
        activeCategory={activeCategory}
        filteredCount={filtered.length}
        onCategoryChange={handleCategoryChange}
      />

      <PromptToolbar
        keyword={keyword}
        loading={loading}
        reloadingDefaults={reloadingDefaults}
        onKeywordChange={setKeyword}
        onLoadDefaults={() => void handleLoadDefaults()}
        onRefresh={() => void handleRefresh()}
      />

      {loadErrors.length > 0 ? (
        <div role="alert" className="space-y-1 text-xs text-error">
          {loadErrors.map((message) => <p key={message}>{message}</p>)}
        </div>
      ) : null}

      <div className="grid min-h-0 gap-3 md:flex-1 md:grid-cols-[clamp(180px,24%,260px)_minmax(0,1fr)] md:grid-rows-1">
        <PromptList
          prompts={filtered}
          loading={loading}
          selectedName={selectedName}
          onSelect={(name) => void handleSelectPrompt(name)}
        />

        <PromptEditorPanel
          prompt={selected}
          reverseDependencies={reverseDependencies}
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
