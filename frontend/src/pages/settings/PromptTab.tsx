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
import { useDirty } from "../../components/navigation/dirtyRegistry";

export const PromptTab: React.FC = () => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [items, setItems] = useState<PromptEntry[]>([]);
  const [categories, setCategories] = useState<CategoryInfo[]>([]);
  const [loadErrors, setLoadErrors] = useState<unknown[]>([]);
  const [promptListError, setPromptListError] = useState<unknown | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeCategory, setActiveCategory] = useState<string>("all");
  const [keyword, setKeyword] = useState("");
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [draft, setDraft] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [reloadingDefaults, setReloadingDefaults] = useState(false);
  const [draftTransitioning, setDraftTransitioning] = useState(false);
  const [projectOverrideBusy, setProjectOverrideBusy] = useState(false);
  const mountedRef = useRef(true);
  const dirtyRef = useRef(false);
  const draftRef = useRef(draft);
  const selectedNameRef = useRef(selectedName);
  const promptListErrorRef = useRef<unknown | null>(promptListError);
  const loadingRef = useRef(loading);
  const tRef = useRef(t);
  selectedNameRef.current = selectedName;
  promptListErrorRef.current = promptListError;
  loadingRef.current = loading;
  tRef.current = t;
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
  const draftTransitionRef = useRef(false);

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

  const writeDraft = useCallback((next: string) => {
    draftRef.current = next;
    setDraft(next);
  }, []);

  const markDirty = useCallback(() => {
    editVersionRef.current += 1;
    dirtyRef.current = true;
    setDirty(true);
  }, []);

  const syncDirtyToContent = useCallback((savedContent: string) => {
    const nextDirty = draftRef.current !== savedContent;
    dirtyRef.current = nextDirty;
    setDirty(nextDirty);
  }, []);

  const refresh = useCallback(async (
    options: { preserveDraft?: boolean } = {},
  ): Promise<PromptEntry[] | null> => {
    if (refreshBusyRef.current) return null;
    const requestSeq = refreshSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    refreshSeqRef.current = requestSeq;
    refreshBusyRef.current = true;
    setLoading(true);
    setLoadErrors([]);
    setPromptListError(null);
    try {
      const [listResult, categoriesResult] = await Promise.allSettled([
        promptList(),
        promptListCategories(),
      ]);
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return null;
      const preserveDraft = options.preserveDraft ?? false;
      if (!preserveDraft && dirtyRef.current && editVersionRef.current !== requestEditVersion) {
        addToast(tRef.current("settingsPrompt.draftKept"), "info");
        return null;
      }
      const failures: unknown[] = [];
      const refreshedPrompts = listResult.status === "fulfilled" ? listResult.value : null;
      if (listResult.status === "fulfilled") {
        const prompts = listResult.value;
        setItems(prompts);
        setPromptListError(null);
        setSelectedName((current) =>
          current && prompts.some((prompt) => prompt.name === current)
            ? current
            : prompts[0]?.name ?? null,
        );
      } else {
        setPromptListError(listResult.reason);
        failures.push(listResult.reason);
      }
      if (categoriesResult.status === "fulfilled") {
        setCategories(categoriesResult.value);
      } else {
        failures.push(categoriesResult.reason);
      }
      setLoadErrors(failures);
      failures.forEach((error) =>
        addToast(extractErrorMessage(error, tRef.current("settingsPrompt.loadFailed")), "error"),
      );

      return refreshedPrompts;
    } finally {
      if (refreshSeqRef.current === requestSeq) refreshBusyRef.current = false;
      if (mountedRef.current && refreshSeqRef.current === requestSeq)
        setLoading(false);
    }
  }, [addToast, markClean, syncDirtyToContent, writeDraft]);

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

  const isPromptOperationBusy = () =>
    refreshEntryRef.current ||
    refreshBusyRef.current ||
    saveBusyRef.current ||
    resetBusyRef.current ||
    defaultsBusyRef.current ||
    draftTransitionRef.current ||
    projectOverrideBusy;
  const ensurePromptListCurrent = () => {
    if (!loadingRef.current && !promptListErrorRef.current) return true;
    addToast(
      promptListErrorRef.current
        ? extractErrorMessage(promptListErrorRef.current, t("settingsPrompt.loadFailed"))
        : t("common.operationInProgress"),
      "error",
    );
    return false;
  };
  useDirty("settings-prompt-editor", dirty || projectOverrideBusy, undefined, () => !isPromptOperationBusy());

  const handleCategoryChange = async (nextCategory: string): Promise<boolean> => {
    if (isPromptOperationBusy()) return false;
    if (nextCategory === activeCategory) return true;
    if (!ensurePromptListCurrent()) return false;
    draftTransitionRef.current = true;
    setDraftTransitioning(true);
    try {
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
    } finally {
      draftTransitionRef.current = false;
      setDraftTransitioning(false);
    }
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
      writeDraft(selected.content);
      markClean();
    } else {
      writeDraft("");
      markClean();
    }
  }, [markClean, selectedName, selected?.content, writeDraft]);

  const discardDraftToSavedState = useCallback(() => {
    editVersionRef.current += 1;
    if (selected) {
      writeDraft(selected.content);
    } else {
      writeDraft("");
    }
    markClean();
  }, [markClean, selected, writeDraft]);

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
    if (isPromptOperationBusy()) return;
    if (name === selectedName) return;
    if (!ensurePromptListCurrent()) return;
    draftTransitionRef.current = true;
    setDraftTransitioning(true);
    try {
      if (!(await confirmDiscardDraft(t("settingsPrompt.switchPrompt")))) return;
      if (
        !mountedRef.current ||
        !items.some((prompt) => prompt.name === name)
      ) {
        addToast(t("common.listChangedRetry"), "info");
        return;
      }
      discardDraftToSavedState();
      setSelectedName(name);
    } finally {
      draftTransitionRef.current = false;
      setDraftTransitioning(false);
    }
  };

  const handleRefresh = async () => {
    if (isPromptOperationBusy()) return;
    refreshEntryRef.current = true;
    draftTransitionRef.current = true;
    setDraftTransitioning(true);
    try {
      if (!(await confirmDiscardDraft(t("settingsPrompt.refreshList")))) return;
      if (!mountedRef.current) return;
      if (dirtyRef.current) {
        discardDraftToSavedState();
      }
      if (!mountedRef.current) return;
      await refresh();
    } finally {
      refreshEntryRef.current = false;
      draftTransitionRef.current = false;
      setDraftTransitioning(false);
    }
  };

  const handleSave = async () => {
    if (isPromptOperationBusy()) return;
    if (!ensurePromptListCurrent()) return;
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
      const saved = await promptUpdate({
        name: requestName,
        content: requestDraft,
      });
      if (!mountedRef.current || saveSeqRef.current !== requestSeq) return;
      setItems((current) => current.map((item) => item.name === requestName ? saved : item));
      if (selectedNameRef.current === requestName && editVersionRef.current !== requestEditVersion) {
        syncDirtyToContent(saved.content);
        addToast(
          dirtyRef.current
            ? t("settingsPrompt.savedSubmittedDraft", { name: requestLabel })
            : t("settingsPrompt.saved", { name: requestLabel }),
          dirtyRef.current ? "info" : "success",
        );
        return;
      }
      if (selectedNameRef.current === requestName) {
        writeDraft(saved.content);
        markClean();
      }
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
    if (isPromptOperationBusy()) return;
    if (!ensurePromptListCurrent()) return;
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
    if (!ensurePromptListCurrent()) {
      resetBusyRef.current = false;
      return;
    }
    const requestSeq = resetSeqRef.current + 1;
    const requestEditVersion = editVersionRef.current;
    resetSeqRef.current = requestSeq;
    setResetting(true);
    try {
      const resetPrompt = await promptReset(requestName);
      if (!mountedRef.current || resetSeqRef.current !== requestSeq) return;
      setItems((current) => current.map((prompt) => prompt.name === resetPrompt.name ? resetPrompt : prompt));
      if (selectedNameRef.current === requestName && editVersionRef.current !== requestEditVersion) {
        syncDirtyToContent(resetPrompt.content);
      } else if (selectedNameRef.current === requestName) {
        writeDraft(resetPrompt.content);
        markClean();
      }
      addToast(
        dirtyRef.current && selectedNameRef.current === requestName
          ? t("settingsPrompt.restoredSubmittedDraft", { name: requestLabel })
          : t("settingsPrompt.restored", { name: requestLabel }),
        dirtyRef.current && selectedNameRef.current === requestName ? "info" : "success",
      );
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
    if (isPromptOperationBusy()) return;
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
      addToast(successMessage, "success");
      if (editVersionRef.current === requestEditVersion) markClean();
      void refresh({ preserveDraft: true }).then((refreshed) => {
        if (!mountedRef.current || defaultsSeqRef.current !== requestSeq) return;
        if (!refreshed) {
          addToast(t("settingsPrompt.refreshFailed"), "error");
          return;
        }
        if (dirtyRef.current && editVersionRef.current !== requestEditVersion) {
          addToast(t("settingsPrompt.keepDraft", { message: successMessage }), "info");
        }
      });
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

  const mutationBusy = loading || saving || resetting || reloadingDefaults || projectOverrideBusy;
  const selectionBusy = mutationBusy || draftTransitioning;

  return (
    <div
      data-settings-dirty={dirty ? "true" : undefined}
      className="flex h-auto min-h-0 flex-col gap-3 md:h-full"
    >
      <PromptCategoryTabs
        categories={categories}
        activeCategory={activeCategory}
        filteredCount={filtered.length}
        disabled={selectionBusy}
        onCategoryChange={handleCategoryChange}
      />

      <PromptToolbar
        keyword={keyword}
        loading={loading}
        mutationBusy={mutationBusy || draftTransitioning}
        reloadingDefaults={reloadingDefaults}
        onKeywordChange={setKeyword}
        onLoadDefaults={() => void handleLoadDefaults()}
        onRefresh={() => void handleRefresh()}
      />

      {loadErrors.length > 0 ? (
        <div role="alert" className="space-y-1 text-xs text-error">
          {loadErrors.map((error, index) => (
            <p key={index}>{extractErrorMessage(error, t("settingsPrompt.loadFailed"))}</p>
          ))}
        </div>
      ) : null}

      <div className="grid min-h-0 gap-3 md:flex-1 md:grid-cols-[clamp(180px,24%,260px)_minmax(0,1fr)] md:grid-rows-1">
        <PromptList
          prompts={filtered}
          loading={loading}
          disabled={selectionBusy}
          unavailable={promptListError != null && items.length === 0}
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
          mutationBusy={mutationBusy || promptListError != null || draftTransitioning}
          onDraftChange={(nextDraft) => {
            markDirty();
            writeDraft(nextDraft);
            if (nextDraft === selected?.content) markClean();
          }}
          onSave={() => void handleSave()}
          onReset={() => void handleReset()}
          onProjectOverrideBusyChange={setProjectOverrideBusy}
        />
      </div>
    </div>
  );
};
