import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ArrowDown, ArrowUp, Plus, RefreshCw, RotateCcw, Save, Trash2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  promptAssemblyGet,
  promptAssemblyList,
  promptAssemblyReset,
  promptAssemblySave,
} from "../../../api/prompt";
import type {
  PromptAssemblyBlock,
  PromptAssemblyBlockKind,
  PromptAssemblyOutput,
  PromptAssemblySummary,
} from "../../../api/generated/host";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { BookButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { PromptProjectSelect } from "./PromptProjectSelect";
import { useI18n } from "../../../i18n";
import { useDirty } from "../../../components/navigation/dirtyRegistry";

export const PROMPT_ASSEMBLY_DIRTY_KEY = "settings-prompt-assembly";

interface PromptAssemblyPanelProps {
  promptName: string;
  promptListReady: boolean;
  promptListRevision: number;
}

const emptyConfig = { blocks: [] as PromptAssemblyBlock[] };
const MAX_PROMPT_ASSEMBLY_BLOCKS = 64;

const projectContextField = (key: string): string =>
  key.startsWith("project_info.") ? key.slice("project_info.".length) : key;

const objectValue = (value: unknown): Record<string, unknown> =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};

const displayContextValue = (value: unknown, emptyLabel: string): string => {
  if (value == null || value === "") return emptyLabel;
  if (typeof value === "string") return value;
  return JSON.stringify(value, null, 2) ?? String(value);
};

const cloneConfig = (output: PromptAssemblyOutput) => ({
  blocks: output.config.blocks.map((block) => ({ ...block })),
});

const callOptionLabel = (
  callKey: string,
  t: (key: "settingsPrompt.assemblyGenericCall") => string,
): string => {
  if (callKey.startsWith("prompt:")) return t("settingsPrompt.assemblyGenericCall");
  const leaf = callKey.split(/::|\./).filter(Boolean).at(-1) ?? callKey;
  return leaf
    .replace(/_for_user_validated$/, "")
    .replace(/_(?:validated|request|task)$/, "")
    .replace(/_/g, " ");
};

export const PromptAssemblyPanel: React.FC<PromptAssemblyPanelProps> = ({
  promptName,
  promptListReady,
  promptListRevision,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const projectContextLabels: Record<string, string> = {
    id: t("settingsPrompt.projectContextId"),
    title: t("settingsPrompt.projectContextTitle"),
    description: t("settingsPrompt.projectContextDescription"),
    project_type: t("settingsPrompt.projectContextType"),
    project_kind: t("settingsPrompt.projectContextKind"),
    source_sync_status: t("settingsPrompt.projectContextSyncStatus"),
    status: t("settingsPrompt.projectContextStatus"),
    target_chapters: t("settingsPrompt.projectContextTargetChapters"),
    current_chapter: t("settingsPrompt.projectContextCurrentChapter"),
    total_chapters: t("settingsPrompt.projectContextTotalChapters"),
    chapters_per_part: t("settingsPrompt.projectContextChaptersPerPart"),
    raw_blueprint: t("settingsPrompt.projectContextRawBlueprint"),
    import_analysis_progress: t("settingsPrompt.projectContextImportProgress"),
    export_config: t("settingsPrompt.projectContextExportConfig"),
  };
  const [summaries, setSummaries] = useState<PromptAssemblySummary[]>([]);
  const [callKey, setCallKey] = useState("");
  const [projectId, setProjectId] = useState<number | null>(null);
  const [savedConfig, setSavedConfig] = useState(emptyConfig);
  const [draft, setDraft] = useState(emptyConfig);
  const [source, setSource] = useState<PromptAssemblyOutput["source"]>("system_default");
  const [projectContext, setProjectContext] = useState<Record<string, unknown>>({});
  const [savedPreview, setSavedPreview] = useState("");
  const [preview, setPreview] = useState("");
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [loadingList, setLoadingList] = useState(true);
  const [loadingConfig, setLoadingConfig] = useState(false);
  const [saving, setSaving] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [errorKind, setErrorKind] = useState<"list" | "config" | null>(null);
  const [errorOwnerKey, setErrorOwnerKey] = useState<string | null>(null);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const scopeSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const summarySeqRef = useRef(0);
  const loadedScopeKeyRef = useRef<string | null>(null);
  const activeScopeKeyRef = useRef<string | null>(null);
  const activePromptListRevisionRef = useRef(promptListRevision);

  const calls = useMemo(
    () => summaries.filter((summary) => summary.prompt_name === promptName),
    [promptName, summaries],
  );
  const selected = calls.find((summary) => summary.call_key === callKey) ?? calls[0];
  const contextVariables = selected?.project_context_variables ?? [];
  const configScopeKey = selected
    ? JSON.stringify([selected.prompt_name, selected.call_key, projectId])
    : null;
  useLayoutEffect(() => {
    activeScopeKeyRef.current = configScopeKey;
  }, [configScopeKey]);
  useLayoutEffect(() => {
    activePromptListRevisionRef.current = promptListRevision;
  }, [promptListRevision]);
  const configReady = configScopeKey != null && loadedScopeKey === configScopeKey;
  const hasUnsavedChanges = configReady && JSON.stringify(draft) !== JSON.stringify(savedConfig);
  const hasValidDraft = draft.blocks.length > 0
    && draft.blocks.some((block) => block.enabled)
    && draft.blocks.every((block) => block.value.trim().length > 0);
  const working = loadingList || loadingConfig || saving || resetting;
  const actionsDisabled = working || !promptListReady;
  const dirty = hasUnsavedChanges || saving || resetting;
  const visibleError = errorKind === "list"
    ? errorOwnerKey === promptName ? error : null
    : errorKind === "config" && errorOwnerKey === configScopeKey ? error : null;

  const discardDraft = useCallback(() => {
    if (saving || resetting) return false;
    previewSeqRef.current += 1;
    setDraft(savedConfig);
    setPreview(savedPreview);
    setPreviewError(null);
    setPreviewing(false);
    return true;
  }, [resetting, savedConfig, savedPreview, saving]);

  useDirty(PROMPT_ASSEMBLY_DIRTY_KEY, dirty, discardDraft, () => !saving && !resetting);

  const loadSummaries = useCallback(async (): Promise<boolean> => {
    const requestSeq = summarySeqRef.current + 1;
    summarySeqRef.current = requestSeq;
    setLoadingList(true);
    setError(null);
    setErrorKind(null);
    setErrorOwnerKey(null);
    setSummaries([]);
    setCallKey("");
    try {
      const items = await promptAssemblyList();
      if (summarySeqRef.current !== requestSeq) return false;
      setSummaries(items);
      return true;
    } catch (loadError) {
      if (summarySeqRef.current === requestSeq) {
        setError(extractErrorMessage(loadError, t("settingsPrompt.assemblyLoadFailed")));
        setErrorKind("list");
        setErrorOwnerKey(promptName);
      }
      return false;
    } finally {
      if (summarySeqRef.current === requestSeq) setLoadingList(false);
    }
  }, [promptName, t]);

  useEffect(() => {
    void loadSummaries();
    return () => {
      summarySeqRef.current += 1;
      scopeSeqRef.current += 1;
    };
  }, [loadSummaries, promptName]);

  useEffect(() => {
    if (!calls.some((item) => item.call_key === callKey)) {
      setCallKey(calls[0]?.call_key ?? "");
    }
  }, [callKey, calls]);

  const loadConfig = useCallback(async (): Promise<boolean> => {
    if (!selected) {
      scopeSeqRef.current += 1;
      loadedScopeKeyRef.current = null;
      setLoadedScopeKey(null);
      setLoadingConfig(false);
      setSavedConfig(emptyConfig);
      setDraft(emptyConfig);
      setSource("system_default");
      setProjectContext({});
      setSavedPreview("");
      setPreview("");
      setPreviewError(null);
      setPreviewing(false);
      return false;
    }
    const requestScopeKey = JSON.stringify([selected.prompt_name, selected.call_key, projectId]);
    if (activeScopeKeyRef.current !== requestScopeKey) return false;
    const keepCurrentScope = loadedScopeKeyRef.current === requestScopeKey;
    const requestSeq = scopeSeqRef.current + 1;
    scopeSeqRef.current = requestSeq;
    setLoadingConfig(true);
    setError(null);
    setErrorKind(null);
    setErrorOwnerKey(null);
    setPreviewError(null);
    setPreviewing(false);
    setSavedPreview("");
    setPreview("");
    if (!keepCurrentScope) {
      loadedScopeKeyRef.current = null;
      setLoadedScopeKey(null);
      setSavedConfig(emptyConfig);
      setDraft(emptyConfig);
      setSource("system_default");
      setProjectContext({});
    }
    try {
      const output = await promptAssemblyGet({
        call_key: selected.call_key,
        prompt_name: selected.prompt_name,
        project_id: projectId,
      });
      if (scopeSeqRef.current !== requestSeq || activeScopeKeyRef.current !== requestScopeKey) return false;
      const config = cloneConfig(output);
      setSavedConfig(config);
      setDraft(config);
      setSource(output.source);
      setProjectContext(objectValue(output.project_context));
      setSavedPreview(output.rendered_preview);
      setPreview(output.rendered_preview);
      loadedScopeKeyRef.current = requestScopeKey;
      setLoadedScopeKey(requestScopeKey);
      return true;
    } catch (loadError) {
      if (scopeSeqRef.current === requestSeq) {
        setError(extractErrorMessage(loadError, t("settingsPrompt.assemblyLoadFailed")));
        setErrorKind("config");
        setErrorOwnerKey(requestScopeKey);
      }
      return false;
    } finally {
      if (scopeSeqRef.current === requestSeq) setLoadingConfig(false);
    }
  }, [projectId, selected, t]);

  useEffect(() => {
    void loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    if (!selected || loadingConfig || !configReady || saving || resetting) {
      setPreviewing(false);
      return;
    }
    const requestSeq = ++previewSeqRef.current;
    setPreviewError(null);
    if (!hasUnsavedChanges) {
      setPreview(savedPreview);
      setPreviewing(false);
      return;
    }
    if (!hasValidDraft) {
      setPreview("");
      setPreviewing(false);
      return;
    }

    setPreviewing(true);
    const timer = window.setTimeout(() => {
      promptAssemblyGet({
        call_key: selected.call_key,
        prompt_name: selected.prompt_name,
        project_id: projectId,
        preview_config: draft,
      })
        .then((output) => {
          if (previewSeqRef.current === requestSeq) {
            setProjectContext(objectValue(output.project_context));
            setPreview(output.rendered_preview);
          }
        })
        .catch((previewFailure: unknown) => {
          if (previewSeqRef.current === requestSeq) {
            setPreviewError(
              extractErrorMessage(previewFailure, t("settingsPrompt.assemblyPreviewFailed")),
            );
          }
        })
        .finally(() => {
          if (previewSeqRef.current === requestSeq) setPreviewing(false);
        });
    }, 180);
    return () => {
      window.clearTimeout(timer);
      previewSeqRef.current += 1;
    };
  }, [configReady, hasUnsavedChanges, hasValidDraft, draft, loadingConfig, projectId, resetting, savedPreview, saving, selected, t]);

  const updateBlock = (blockId: string, update: Partial<PromptAssemblyBlock>) => {
    if (!promptListReady || !configReady || working) return;
    setDraft((current) => ({
      blocks: current.blocks.map((block) =>
        block.id === blockId ? { ...block, ...update } : block,
      ),
    }));
  };

  const addBlock = (kind: PromptAssemblyBlockKind) => {
    if (!selected || !promptListReady || !configReady || working) return;
    const option =
      kind === "template"
        ? selected.available_templates[0]?.name
        : kind === "variable"
          ? selected.variables[0] ?? (projectId ? selected.project_context_variables[0] : undefined)
          : "";
    if (!option && kind !== "custom") {
      addToast(t(kind === "template" ? "settingsPrompt.assemblyNoTemplates" : "settingsPrompt.assemblyVariableSelect"), "error");
      return;
    }
    setDraft((current) => ({
      blocks: [
        ...current.blocks,
        { id: crypto.randomUUID(), kind, value: option, enabled: true },
      ],
    }));
  };

  const toggleProjectContext = (key: string) => {
    if (!projectId || !selected || !promptListReady || !configReady || working) return;
    const alreadyIncluded = draft.blocks.some(
      (block) => block.kind === "variable" && block.value === key,
    );
    if (!alreadyIncluded && draft.blocks.length >= MAX_PROMPT_ASSEMBLY_BLOCKS) {
      addToast(t("settingsPrompt.assemblyBlockLimitReached", { count: MAX_PROMPT_ASSEMBLY_BLOCKS }), "info");
      return;
    }
    setDraft((current) => {
      const hasBlock = current.blocks.some(
        (block) => block.kind === "variable" && block.value === key,
      );
      if (hasBlock) {
        return {
          blocks: current.blocks.filter(
            (block) => !(block.kind === "variable" && block.value === key),
          ),
        };
      }
      if (current.blocks.length >= MAX_PROMPT_ASSEMBLY_BLOCKS) return current;
      return {
        blocks: [
          ...current.blocks,
          { id: crypto.randomUUID(), kind: "variable", value: key, enabled: true },
        ],
      };
    });
  };

  const moveBlock = (index: number, offset: -1 | 1) => {
    if (!promptListReady || !configReady || working) return;
    setDraft((current) => {
      const target = index + offset;
      if (target < 0 || target >= current.blocks.length) return current;
      const blocks = [...current.blocks];
      [blocks[index], blocks[target]] = [blocks[target], blocks[index]];
      return { blocks };
    });
  };

  const deleteBlock = (blockId: string) => {
    if (!promptListReady || !configReady || working) return;
    setDraft((current) => ({
      blocks: current.blocks.filter((block) => block.id !== blockId),
    }));
  };

  const save = async () => {
    if (!selected || !promptListReady || !configReady || !hasUnsavedChanges || !hasValidDraft || working) return;
    const requestScopeKey = configScopeKey;
    const requestPromptName = promptName;
    const requestPromptListRevision = promptListRevision;
    const config = { blocks: draft.blocks.map((block) => ({ ...block })) };
    setSaving(true);
    try {
      await promptAssemblySave({
        call_key: selected.call_key,
        prompt_name: selected.prompt_name,
        project_id: projectId,
        config,
      });
      if (
        activeScopeKeyRef.current !== requestScopeKey ||
        activePromptListRevisionRef.current !== requestPromptListRevision ||
        promptName !== requestPromptName ||
        !promptListReady
      ) return;
      setSavedConfig(config);
      setDraft(config);
      setSource(projectId == null ? "global" : "project");
      setSavedPreview("");
      setPreview("");
      addToast(t("settingsPrompt.assemblySaved"), "success");
      await loadConfig();
    } catch (saveError) {
      addToast(extractErrorMessage(saveError, t("settingsPrompt.assemblySaveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  const reset = async () => {
    if (!selected || !promptListReady || !configReady || working) return;
    const requestScopeKey = configScopeKey;
    const requestPromptName = promptName;
    const requestPromptListRevision = promptListRevision;
    if (hasUnsavedChanges) {
      const discard = await confirmDialog({
        title: t("common.unsavedChangesTitle"),
        message: t("common.unsavedChangesMessage", {
          name: t("settingsPrompt.assembly"),
          action: t("settingsPrompt.assemblyResetAction"),
        }),
        confirmText: t("common.discardDraft"),
        dialogType: "warning",
      });
      if (!discard) return;
      if (
        activeScopeKeyRef.current !== requestScopeKey ||
        activePromptListRevisionRef.current !== requestPromptListRevision ||
        !promptListReady
      ) {
        addToast(t("common.listChangedRetry"), "info");
        return;
      }
    }
    setResetting(true);
    try {
      await promptAssemblyReset({
        call_key: selected.call_key,
        prompt_name: selected.prompt_name,
        project_id: projectId,
      });
      if (
        activeScopeKeyRef.current !== requestScopeKey ||
        activePromptListRevisionRef.current !== requestPromptListRevision ||
        promptName !== requestPromptName ||
        !promptListReady
      ) return;
      loadedScopeKeyRef.current = null;
      setLoadedScopeKey(null);
      setSavedConfig(emptyConfig);
      setDraft(emptyConfig);
      setSource("system_default");
      setProjectContext({});
      setSavedPreview("");
      setPreview("");
      addToast(t("settingsPrompt.assemblyReset"), "success");
      await loadConfig();
    } catch (resetError) {
      addToast(extractErrorMessage(resetError, t("settingsPrompt.assemblyResetFailed")), "error");
    } finally {
      setResetting(false);
    }
  };

  const sourceLabel = t(
    source === "project"
      ? "settingsPrompt.assemblySourceProject"
      : source === "global"
        ? "settingsPrompt.assemblySourceGlobal"
        : "settingsPrompt.assemblySourceDefault",
  );

  const confirmScopeChange = async (action: () => void) => {
    const requestScopeKey = configScopeKey;
    const requestPromptListRevision = promptListRevision;
    if (hasUnsavedChanges) {
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
        activeScopeKeyRef.current !== requestScopeKey ||
        activePromptListRevisionRef.current !== requestPromptListRevision ||
        !promptListReady
      ) {
        addToast(t("common.listChangedRetry"), "info");
        return;
      }
    }
    action();
  };

  const retryConfig = () => {
    if (!promptListReady) {
      addToast(t("common.listChangedRetry"), "info");
      return;
    }
    void confirmScopeChange(() => { void loadConfig(); });
  };

  return (
    <section className="min-w-0 py-3">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <h4 className="min-w-0 flex-1 text-xs font-semibold text-book-text-main">
          {t("settingsPrompt.assembly")}
        </h4>
        {selected && configReady ? (
          <span className="max-w-full truncate text-small text-book-text-muted" title={sourceLabel}>
            {sourceLabel}
          </span>
        ) : null}
      </div>

      {loadingList ? (
        <div className="flex min-h-12 items-center gap-2 text-small text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("settingsPrompt.loading")}
        </div>
      ) : visibleError && errorKind === "list" && calls.length === 0 ? (
        <div role="alert" className="flex flex-wrap items-center gap-2 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{visibleError}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void loadSummaries()} disabled={working}>
            <RefreshCw size={12} />{t("common.retry")}
          </BookButton>
        </div>
      ) : !selected ? (
        <div className="py-2 text-small text-book-text-muted">{t("settingsPrompt.assemblyEmpty")}</div>
      ) : (
        <div className="mt-2 grid min-w-0 gap-2">
          <div className="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(12rem,0.6fr)]">
            <label className="grid min-w-0 gap-1 text-small text-book-text-muted">
              <span>{t("settingsPrompt.assemblyCall")}</span>
              <ThemedSelect
                className="pyqt-input min-w-0 w-full"
                value={selected.call_key}
                title={selected.call_key}
                aria-label={t("settingsPrompt.assemblyCall")}
                disabled={actionsDisabled}
                onChange={(value) => void confirmScopeChange(() => setCallKey(value))}
              >
                {calls.map((call) => (
                  <option key={call.call_key} value={call.call_key} title={call.call_key}>
                    {callOptionLabel(call.call_key, t)}
                  </option>
                ))}
              </ThemedSelect>
            </label>
            <label className="grid min-w-0 gap-1 text-small text-book-text-muted">
              <span>{t("settingsPrompt.apply")}</span>
              <PromptProjectSelect
                value={projectId}
                onChange={(nextProjectId) => void confirmScopeChange(() => setProjectId(nextProjectId))}
                disabled={actionsDisabled}
                allowEmpty
                requireEditor
                ariaLabel={t("settingsPrompt.apply")}
              />
            </label>
          </div>

          {visibleError && errorKind === "config" ? (
            <div role="alert" className="flex flex-wrap items-center gap-2 text-small text-error">
              <span className="min-w-0 flex-1">{visibleError}</span>
              <BookButton variant="secondary" size="sm" onClick={retryConfig} disabled={working}>
                <RefreshCw size={12} />{t("common.retry")}
              </BookButton>
            </div>
          ) : null}

          {configReady ? (
            <>
          <section className="min-w-0 overflow-hidden rounded-md border border-book-border/45 bg-book-bg-secondary/20">
            <div className="flex min-w-0 flex-wrap items-center justify-between gap-2 border-b border-book-border/40 px-3 py-2">
              <div className="min-w-0">
                <h5 className="text-xs font-semibold text-book-text-main">
                  {t("settingsPrompt.projectContext")}
                </h5>
                <p className="mt-0.5 text-caption text-book-text-muted">
                  {projectId
                    ? t("settingsPrompt.projectContextHint")
                    : t("settingsPrompt.projectContextSelectProject")}
                </p>
              </div>
              {projectId ? (
                <span className="shrink-0 text-caption text-book-text-muted">
                  {contextVariables.length} {t("settingsPrompt.projectContextAvailable")}
                </span>
              ) : null}
            </div>
            {projectId ? (
              <div className="grid min-w-0 divide-y divide-book-border/30 sm:grid-cols-2 sm:divide-x sm:divide-y-0">
                {contextVariables.map((key) => {
                  const field = projectContextField(key);
                  const checked = draft.blocks.some(
                    (block) => block.kind === "variable" && block.value === key,
                  );
                  const value = displayContextValue(projectContext[field], t("common.none"));
                  const expandable = value.length > 180;
                  return (
                    <div
                      key={key}
                      className={`min-w-0 px-3 py-2.5 ${checked ? "bg-book-accent/8" : ""}`}
                    >
                      <label className="flex min-w-0 cursor-pointer items-start gap-2">
                        <input
                          type="checkbox"
                          className="mt-0.5 shrink-0"
                          checked={checked}
                          disabled={actionsDisabled}
                          onChange={() => toggleProjectContext(key)}
                          aria-label={projectContextLabels[field] ?? field}
                        />
                        <span className="min-w-0 truncate text-small font-medium text-book-text-main">
                          {projectContextLabels[field] ?? field}
                        </span>
                      </label>
                      {expandable ? (
                        <details className="ml-6 mt-1 min-w-0 text-caption text-book-text-muted">
                          <summary className="cursor-pointer select-none">
                            <span className="line-clamp-2 whitespace-pre-wrap break-words">
                              {value}
                            </span>
                          </summary>
                          <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap break-words rounded border border-book-border/35 p-2 font-mono text-caption [overflow-wrap:anywhere]">
                            {value}
                          </pre>
                        </details>
                      ) : (
                        <span className="ml-6 mt-1 block whitespace-pre-wrap break-words text-caption text-book-text-muted">
                          {value}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            ) : null}
          </section>

          <div className="grid min-w-0 gap-2">
            {draft.blocks.map((block, index) => (
              <AssemblyBlockRow
                key={block.id}
                block={block}
                index={index}
                count={draft.blocks.length}
                summary={selected}
                projectContextEnabled={projectId != null}
                disabled={!configReady || working}
                onChange={(update) => updateBlock(block.id, update)}
                onMove={(offset) => moveBlock(index, offset)}
                onDelete={() => deleteBlock(block.id)}
              />
            ))}
            {loadingConfig ? (
              <div className="flex min-h-12 items-center gap-2 text-small text-book-text-muted">
                <LoadingSpinner size="sm" />
                {t("settingsPrompt.loading")}
              </div>
            ) : draft.blocks.length === 0 ? (
              <div className="border-y border-book-border/35 py-3 text-small text-book-text-muted">
                {t("settingsPrompt.assemblyEmpty")}
              </div>
            ) : null}
          </div>

          <section className="min-w-0 overflow-hidden rounded-md border border-book-border/45 bg-book-bg-secondary/30">
            <div className="flex min-w-0 items-center justify-between gap-3 border-b border-book-border/40 px-3 py-2">
              <h5 className="text-xs font-semibold text-book-text-main">
                {t("settingsPrompt.assemblyPreview")}
              </h5>
              {previewing ? (
                <span className="inline-flex shrink-0 items-center gap-1.5 text-caption text-book-text-muted">
                  <LoadingSpinner size="sm" />
                  {t("settingsPrompt.assemblyPreviewUpdating")}
                </span>
              ) : null}
            </div>
            {previewError ? (
              <p role="alert" className="px-3 py-2 text-small text-error">{previewError}</p>
            ) : (
              <pre aria-live="polite" className="max-h-80 min-h-16 overflow-auto whitespace-pre-wrap break-words px-3 py-3 font-mono text-small leading-relaxed text-book-text-main [overflow-wrap:anywhere]">
                {preview || t("settingsPrompt.assemblyPreviewEmpty")}
              </pre>
            )}
            <p className="border-t border-book-border/35 px-3 py-2 text-caption text-book-text-muted">
              {t("settingsPrompt.assemblyPreviewHint")}
            </p>
          </section>

          <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center">
            <div className="flex flex-wrap gap-1.5">
              <BookButton variant="secondary" size="sm" onClick={() => addBlock("template")} disabled={!configReady || working || draft.blocks.length >= MAX_PROMPT_ASSEMBLY_BLOCKS || selected.available_templates.length === 0}>
                <Plus size={13} />{t("settingsPrompt.assemblyAddTemplate")}
              </BookButton>
              <BookButton variant="secondary" size="sm" onClick={() => addBlock("variable")} disabled={!configReady || working || draft.blocks.length >= MAX_PROMPT_ASSEMBLY_BLOCKS || (selected.variables.length === 0 && (!projectId || selected.project_context_variables.length === 0))}>
                <Plus size={13} />{t("settingsPrompt.assemblyAddVariable")}
              </BookButton>
              <BookButton variant="secondary" size="sm" onClick={() => addBlock("custom")} disabled={!configReady || working || draft.blocks.length >= MAX_PROMPT_ASSEMBLY_BLOCKS}>
                <Plus size={13} />{t("settingsPrompt.assemblyAddCustom")}
              </BookButton>
            </div>
            {draft.blocks.length >= MAX_PROMPT_ASSEMBLY_BLOCKS ? (
              <span role="status" className="min-w-0 text-small text-book-text-muted sm:flex-1">
                {t("settingsPrompt.assemblyBlockLimitReached", { count: MAX_PROMPT_ASSEMBLY_BLOCKS })}
              </span>
            ) : null}
            {hasUnsavedChanges ? (
              <span className="min-w-0 text-small text-warning sm:flex-1">
                {t("settingsPrompt.assemblyUnsaved")}
              </span>
            ) : null}
            {draft.blocks.length > 0 && !draft.blocks.some((block) => block.enabled) ? (
              <span className="min-w-0 text-small text-error sm:flex-1">
                {t("settingsPrompt.assemblyRequiresEnabledBlock")}
              </span>
            ) : null}
            {draft.blocks.some((block) => !block.value.trim()) ? (
              <span className="min-w-0 text-small text-error sm:flex-1">
                {t("settingsPrompt.assemblyRequiresBlockContent")}
              </span>
            ) : null}
            <div className="flex min-w-0 flex-wrap gap-2 sm:ml-auto">
              <BookButton variant="secondary" size="sm" onClick={() => void reset()} disabled={!configReady || working}>
                {resetting ? <LoadingSpinner size="sm" /> : <RotateCcw size={13} />}
                {t("settingsPrompt.assemblyResetAction")}
              </BookButton>
              <BookButton variant="primary" size="sm" onClick={() => void save()} disabled={!configReady || working || !hasUnsavedChanges || !hasValidDraft}>
                {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
                {t("common.save")}
              </BookButton>
            </div>
          </div>
            </>
          ) : visibleError && errorKind === "config" ? null : (
            <div className="flex min-h-12 items-center gap-2 text-small text-book-text-muted">
              <LoadingSpinner size="sm" />{t("settingsPrompt.loading")}
            </div>
          )}
        </div>
      )}
    </section>
  );
};

const AssemblyBlockRow: React.FC<{
  block: PromptAssemblyBlock;
  index: number;
  count: number;
  summary: PromptAssemblySummary;
  projectContextEnabled: boolean;
  disabled: boolean;
  onChange: (update: Partial<PromptAssemblyBlock>) => void;
  onMove: (offset: -1 | 1) => void;
  onDelete: () => void;
}> = ({ block, index, count, summary, projectContextEnabled, disabled, onChange, onMove, onDelete }) => {
  const { t } = useI18n();
  const kindLabel = t(
    block.kind === "template"
      ? "settingsPrompt.assemblyTemplate"
      : block.kind === "variable"
        ? "settingsPrompt.assemblyVariable"
        : "settingsPrompt.assemblyCustom",
  );
  const variableOptions = projectContextEnabled
    ? [...summary.variables, ...summary.project_context_variables]
    : summary.variables;
  const defaultVariable = variableOptions[0] ?? "";
  return (
    <div className={`grid min-w-0 gap-2 border-b border-book-border/35 py-2 sm:grid-cols-[auto_minmax(9rem,0.55fr)_minmax(0,1fr)_auto] ${block.enabled ? "" : "opacity-60"}`}>
      <label className="flex min-h-9 items-center gap-2 text-small text-book-text-muted">
        <input type="checkbox" checked={block.enabled} disabled={disabled} onChange={(event) => onChange({ enabled: event.target.checked })} aria-label={t("settingsPrompt.assemblyEnabled")} />
        <span>{index + 1}</span>
      </label>
      <label className="grid min-w-0 gap-1 text-small text-book-text-muted">
        <span>{kindLabel}</span>
        <ThemedSelect
          className="pyqt-input min-w-0 w-full"
          value={block.kind}
          aria-label={kindLabel}
          disabled={disabled}
          onChange={(selectedKind) => {
            const kind = selectedKind as PromptAssemblyBlockKind;
            const blockValue = kind === "template"
              ? summary.available_templates[0]?.name ?? ""
              : kind === "variable"
                ? defaultVariable
                : "";
            onChange({ kind, value: blockValue });
          }}
        >
          <option value="template">{t("settingsPrompt.assemblyTemplate")}</option>
          <option value="variable">{t("settingsPrompt.assemblyVariable")}</option>
          <option value="custom">{t("settingsPrompt.assemblyCustom")}</option>
        </ThemedSelect>
      </label>
      {block.kind === "template" ? (
        <ThemedSelect className="pyqt-input min-w-0 w-full self-end" value={block.value} disabled={disabled} onChange={(value) => onChange({ value })} aria-label={t("settingsPrompt.assemblyTemplateSelect")}>
          {summary.available_templates.map((template) => <option key={template.name} value={template.name}>{template.title || template.name}</option>)}
        </ThemedSelect>
      ) : block.kind === "variable" ? (
        <ThemedSelect
          className="pyqt-input min-w-0 w-full self-end"
          value={block.value}
          disabled={disabled}
          onChange={(value) => onChange({ value })}
          aria-label={t("settingsPrompt.assemblyVariableSelect")}
        >
          {summary.variables.map((variable) => (
            <option key={variable} value={variable}>
              {`${t("settingsPrompt.assemblyVariableGroup")}: {{ ${variable} }}`}
            </option>
          ))}
          {projectContextEnabled && summary.project_context_variables.length > 0
            ? summary.project_context_variables.map((variable) => (
                <option key={variable} value={variable}>
                  {`${t("settingsPrompt.projectContextGroup")}: {{ ${variable} }}`}
                </option>
              ))
            : null}
        </ThemedSelect>
      ) : (
        <textarea className="book-control min-h-10 min-w-0 w-full resize-y self-end p-2 font-mono text-small" value={block.value} disabled={disabled} onChange={(event) => onChange({ value: event.target.value })} aria-label={t("settingsPrompt.assemblyCustomPlaceholder")} placeholder={t("settingsPrompt.assemblyCustomPlaceholder")} />
      )}
      <div className="flex shrink-0 items-end gap-1">
        <button type="button" className="a4n-touch-target inline-flex h-9 w-9 items-center justify-center rounded border border-book-border/45 text-book-text-muted disabled:opacity-40" title={t("settingsPrompt.assemblyMoveUp")} aria-label={t("settingsPrompt.assemblyMoveUp")} disabled={disabled || index === 0} onClick={() => onMove(-1)}><ArrowUp size={14} /></button>
        <button type="button" className="a4n-touch-target inline-flex h-9 w-9 items-center justify-center rounded border border-book-border/45 text-book-text-muted disabled:opacity-40" title={t("settingsPrompt.assemblyMoveDown")} aria-label={t("settingsPrompt.assemblyMoveDown")} disabled={disabled || index === count - 1} onClick={() => onMove(1)}><ArrowDown size={14} /></button>
        <button type="button" className="a4n-touch-target inline-flex h-9 w-9 items-center justify-center rounded border border-error/30 text-error disabled:opacity-40" title={t("settingsPrompt.assemblyDelete")} aria-label={t("settingsPrompt.assemblyDelete")} disabled={disabled} onClick={onDelete}><Trash2 size={14} /></button>
      </div>
    </div>
  );
};
