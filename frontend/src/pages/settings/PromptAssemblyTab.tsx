import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Blocks, RefreshCw, Search } from "lucide-react";
import { extractErrorMessage } from "../../api";
import { promptList, type PromptEntry } from "../../api/prompt";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { useDirtyRegistry } from "../../components/navigation/dirtyRegistry";
import { useI18n } from "../../i18n";
import { PromptAssemblyPanel, PROMPT_ASSEMBLY_DIRTY_KEY } from "./prompt/PromptAssemblyPanel";
import { PromptList } from "./prompt/PromptList";
import { filterPrompts } from "./prompt/promptTabUtils";

export const PromptAssemblyTab: React.FC = () => {
  const { t } = useI18n();
  const { addToast } = useToast();
  const dirtyRegistry = useDirtyRegistry();
  const [prompts, setPrompts] = useState<PromptEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<unknown | null>(null);
  const [keyword, setKeyword] = useState("");
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [loadRevision, setLoadRevision] = useState(0);
  const loadSeqRef = useRef(0);
  const transitionRef = useRef(false);
  const promptsRef = useRef(prompts);
  const selectedNameRef = useRef(selectedName);
  const loadingRef = useRef(loading);
  const loadErrorRef = useRef(loadError);
  promptsRef.current = prompts;
  selectedNameRef.current = selectedName;
  loadingRef.current = loading;
  loadErrorRef.current = loadError;

  const loadPrompts = useCallback(async (): Promise<boolean> => {
    if (transitionRef.current) return false;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoadRevision(requestSeq);
    loadingRef.current = true;
    loadErrorRef.current = null;
    setLoading(true);
    setLoadError(null);
    try {
      const items = await promptList();
      if (loadSeqRef.current !== requestSeq) return false;
      promptsRef.current = items;
      loadErrorRef.current = null;
      setPrompts(items);
      const current = selectedNameRef.current;
      const nextName = current && items.some((prompt) => prompt.name === current)
        ? current
        : items[0]?.name ?? null;
      selectedNameRef.current = nextName;
      setSelectedName(nextName);
      return true;
    } catch (error: unknown) {
      if (loadSeqRef.current === requestSeq) {
        loadErrorRef.current = error;
        setLoadError(error);
      }
      return false;
    } finally {
      if (loadSeqRef.current === requestSeq) {
        loadingRef.current = false;
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void loadPrompts();
    return () => {
      loadSeqRef.current += 1;
    };
  }, [loadPrompts]);

  const filtered = useMemo(
    () => filterPrompts(prompts, { activeCategory: "all", keyword }),
    [keyword, prompts],
  );
  const selected = prompts.find((prompt) => prompt.name === selectedName) ?? null;

  const ensurePromptListCurrent = (): boolean => {
    if (!loadingRef.current && loadErrorRef.current == null) return true;
    addToast(
      loadErrorRef.current
        ? extractErrorMessage(loadErrorRef.current, t("settingsPrompt.loadFailed"))
        : t("common.operationInProgress"),
      "error",
    );
    return false;
  };

  const selectPrompt = async (name: string) => {
    if (!ensurePromptListCurrent()) return;
    if (name === selectedName) return;
    if (transitionRef.current) return;
    transitionRef.current = true;
    try {
      if (dirtyRegistry.hasDirty(PROMPT_ASSEMBLY_DIRTY_KEY)) {
        if (!dirtyRegistry.canDiscardDirty(PROMPT_ASSEMBLY_DIRTY_KEY)) {
          addToast(t("common.operationInProgress"), "info");
          return;
        }
        const discard = await confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", {
            name: t("settingsPrompt.assembly"),
            action: t("settingsPrompt.switchPrompt"),
          }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        });
        if (!discard) return;
        if (
          loadingRef.current ||
          loadErrorRef.current != null ||
          !promptsRef.current.some((prompt) => prompt.name === name)
        ) {
          addToast(t("common.listChangedRetry"), "info");
          return;
        }
        if (!await dirtyRegistry.discardDirty(PROMPT_ASSEMBLY_DIRTY_KEY)) {
          addToast(t("common.operationInProgress"), "info");
          return;
        }
      }
      if (
        loadingRef.current ||
        loadErrorRef.current != null ||
        !promptsRef.current.some((prompt) => prompt.name === name)
      ) {
        addToast(t("common.listChangedRetry"), "info");
        return;
      }
      selectedNameRef.current = name;
      setSelectedName(name);
    } finally {
      transitionRef.current = false;
    }
  };

  const errorMessage = loadError
    ? extractErrorMessage(loadError, t("settingsPrompt.loadFailed"))
    : null;

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <header className="flex min-w-0 flex-wrap items-center justify-between gap-3 border-b border-book-border/45 pb-2">
        <h2 className="inline-flex min-w-0 items-center gap-2 text-sm font-semibold text-book-text-main">
          <Blocks size={16} className="shrink-0 text-book-primary" aria-hidden="true" />
          <span className="break-words">{t("settings.promptAssembly")}</span>
        </h2>
        <label className="relative block min-w-0 w-full sm:max-w-xs">
          <span className="sr-only">{t("settingsPrompt.searchPlaceholder")}</span>
          <Search size={14} className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-book-text-muted" aria-hidden="true" />
          <input
            type="search"
            className="pyqt-input min-w-0 w-full pl-9"
            value={keyword}
            onChange={(event) => setKeyword(event.target.value)}
            placeholder={t("settingsPrompt.searchPlaceholder")}
            disabled={loading || loadError != null}
          />
        </label>
      </header>

      {errorMessage ? (
        <div role="alert" className="flex flex-wrap items-center gap-2 text-small text-error">
          <span className="min-w-0 flex-1">{errorMessage}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void loadPrompts()} disabled={loading}>
            <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
            {t("common.retry")}
          </BookButton>
        </div>
      ) : null}

      <div className="grid min-h-0 gap-3 md:flex-1 md:grid-cols-[clamp(180px,24%,260px)_minmax(0,1fr)] md:grid-rows-1">
        <aside className="min-w-0 md:min-h-0 md:overflow-y-auto">
          <PromptList
            prompts={filtered}
            loading={loading}
            disabled={loading || loadError != null}
            unavailable={Boolean(errorMessage) || (!loading && prompts.length === 0)}
            maxHeightClassName="max-h-[min(72dvh,42rem)] md:h-full md:max-h-none"
            selectedName={selectedName}
            onSelect={(name) => void selectPrompt(name)}
          />
          {!loading && prompts.length === 0 && !errorMessage ? (
            <p className="py-2 text-center text-small text-book-text-muted">{t("settingsPrompt.noPrompts")}</p>
          ) : null}
        </aside>

        <main className="min-w-0 border-t border-book-border/45 pt-2 md:min-h-0 md:overflow-y-auto md:border-l md:border-t-0 md:pl-3 md:pt-0">
          {selected ? (
            <>
              <header className="mb-1 flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
                <h3 className="min-w-0 break-words text-sm font-semibold text-book-text-main">
                  {selected.title || t("settingsPrompt.untitledPrompt")}
                </h3>
                <code className="min-w-0 break-all text-caption text-book-text-muted">{selected.name}</code>
              </header>
              <PromptAssemblyPanel
                key={`${selected.name}:${loadRevision}`}
                promptName={selected.name}
                promptListReady={!loading && loadError == null}
                promptListRevision={loadRevision}
              />
            </>
          ) : loading ? (
            <div className="flex min-h-20 items-center gap-2 text-small text-book-text-muted">
              <LoadingSpinner size="sm" /> {t("settingsPrompt.loading")}
            </div>
          ) : null}
        </main>
      </div>
    </div>
  );
};
