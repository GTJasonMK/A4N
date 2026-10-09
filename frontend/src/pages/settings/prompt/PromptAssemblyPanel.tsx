import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowDown, ArrowUp, Plus, RotateCcw, Save, Trash2 } from "lucide-react";
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
import { PromptProjectSelect } from "./PromptProjectSelect";
import { useI18n } from "../../../i18n";

interface PromptAssemblyPanelProps {
  promptName: string;
}

const emptyConfig = { blocks: [] as PromptAssemblyBlock[] };

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

export const PromptAssemblyPanel: React.FC<PromptAssemblyPanelProps> = ({ promptName }) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [summaries, setSummaries] = useState<PromptAssemblySummary[]>([]);
  const [callKey, setCallKey] = useState("");
  const [projectId, setProjectId] = useState<number | null>(null);
  const [savedConfig, setSavedConfig] = useState(emptyConfig);
  const [draft, setDraft] = useState(emptyConfig);
  const [source, setSource] = useState<PromptAssemblyOutput["source"]>("system_default");
  const [savedPreview, setSavedPreview] = useState("");
  const [preview, setPreview] = useState("");
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [loadingList, setLoadingList] = useState(true);
  const [loadingConfig, setLoadingConfig] = useState(false);
  const [saving, setSaving] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scopeSeqRef = useRef(0);
  const previewSeqRef = useRef(0);

  const calls = useMemo(
    () => summaries.filter((summary) => summary.prompt_name === promptName),
    [promptName, summaries],
  );
  const selected = calls.find((summary) => summary.call_key === callKey) ?? calls[0];
  const dirty = JSON.stringify(draft) !== JSON.stringify(savedConfig);
  const busy = loadingList || loadingConfig || saving || resetting;

  useEffect(() => {
    let active = true;
    setLoadingList(true);
    setError(null);
    promptAssemblyList()
      .then((items) => {
        if (!active) return;
        setSummaries(items);
      })
      .catch((loadError: unknown) => {
        if (active) setError(extractErrorMessage(loadError, t("settingsPrompt.assemblyLoadFailed")));
      })
      .finally(() => {
        if (active) setLoadingList(false);
      });
    return () => {
      active = false;
      scopeSeqRef.current += 1;
    };
  }, [promptName, t]);

  useEffect(() => {
    if (!calls.some((item) => item.call_key === callKey)) {
      setCallKey(calls[0]?.call_key ?? "");
    }
  }, [callKey, calls]);

  const loadConfig = useCallback(async () => {
    if (!selected) {
      scopeSeqRef.current += 1;
      setLoadingConfig(false);
      setSavedConfig(emptyConfig);
      setDraft(emptyConfig);
      setSource("system_default");
      setSavedPreview("");
      setPreview("");
      setPreviewError(null);
      setPreviewing(false);
      return;
    }
    const requestSeq = scopeSeqRef.current + 1;
    scopeSeqRef.current = requestSeq;
    setLoadingConfig(true);
    setError(null);
    setPreviewError(null);
    setPreviewing(false);
    setSavedPreview("");
    setPreview("");
    try {
      const output = await promptAssemblyGet({
        call_key: selected.call_key,
        prompt_name: selected.prompt_name,
        project_id: projectId,
      });
      if (scopeSeqRef.current !== requestSeq) return;
      const config = cloneConfig(output);
      setSavedConfig(config);
      setDraft(config);
      setSource(output.source);
      setSavedPreview(output.rendered_preview);
      setPreview(output.rendered_preview);
    } catch (loadError) {
      if (scopeSeqRef.current === requestSeq) {
        setError(extractErrorMessage(loadError, t("settingsPrompt.assemblyLoadFailed")));
      }
    } finally {
      if (scopeSeqRef.current === requestSeq) setLoadingConfig(false);
    }
  }, [projectId, selected, t]);

  useEffect(() => {
    void loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    if (!selected || loadingConfig) return;
    const requestSeq = ++previewSeqRef.current;
    setPreviewError(null);
    if (!dirty) {
      setPreview(savedPreview);
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
  }, [dirty, draft, loadingConfig, projectId, savedPreview, selected, t]);

  const updateBlock = (blockId: string, update: Partial<PromptAssemblyBlock>) => {
    setDraft((current) => ({
      blocks: current.blocks.map((block) =>
        block.id === blockId ? { ...block, ...update } : block,
      ),
    }));
  };

  const addBlock = (kind: PromptAssemblyBlockKind) => {
    if (!selected) return;
    const option =
      kind === "template"
        ? selected.available_templates[0]?.name
        : kind === "variable"
          ? selected.variables[0]
          : "Custom instruction";
    if (!option) {
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

  const moveBlock = (index: number, offset: -1 | 1) => {
    setDraft((current) => {
      const target = index + offset;
      if (target < 0 || target >= current.blocks.length) return current;
      const blocks = [...current.blocks];
      [blocks[index], blocks[target]] = [blocks[target], blocks[index]];
      return { blocks };
    });
  };

  const save = async () => {
    if (!selected || !dirty || busy) return;
    setSaving(true);
    try {
      await promptAssemblySave({
        call_key: selected.call_key,
        prompt_name: selected.prompt_name,
        project_id: projectId,
        config: draft,
      });
      await loadConfig();
      addToast(t("settingsPrompt.assemblySaved"), "success");
    } catch (saveError) {
      addToast(extractErrorMessage(saveError, t("settingsPrompt.assemblySaveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  const reset = async () => {
    if (!selected || busy) return;
    if (dirty) {
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
    }
    setResetting(true);
    try {
      await promptAssemblyReset({
        call_key: selected.call_key,
        prompt_name: selected.prompt_name,
        project_id: projectId,
      });
      await loadConfig();
      addToast(t("settingsPrompt.assemblyReset"), "success");
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
    if (dirty) {
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
    }
    action();
  };

  return (
    <section className="min-w-0 py-3">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <h4 className="min-w-0 flex-1 text-xs font-semibold text-book-text-main">
          {t("settingsPrompt.assembly")}
        </h4>
        {selected ? (
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
      ) : error && calls.length === 0 ? (
        <div role="alert" className="py-2 text-small text-error">{error}</div>
      ) : !selected ? (
        <div className="py-2 text-small text-book-text-muted">{t("settingsPrompt.assemblyEmpty")}</div>
      ) : (
        <div className="mt-2 grid min-w-0 gap-2">
          <div className="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(12rem,0.6fr)]">
            <label className="grid min-w-0 gap-1 text-small text-book-text-muted">
              <span>{t("settingsPrompt.assemblyCall")}</span>
              <select
                className="pyqt-input min-w-0 w-full"
                value={selected.call_key}
                title={selected.call_key}
                disabled={busy}
                onChange={(event) => void confirmScopeChange(() => setCallKey(event.target.value))}
              >
                {calls.map((call) => (
                  <option key={call.call_key} value={call.call_key} title={call.call_key}>
                    {callOptionLabel(call.call_key, t)}
                  </option>
                ))}
              </select>
            </label>
            <label className="grid min-w-0 gap-1 text-small text-book-text-muted">
              <span>{t("settingsPrompt.apply")}</span>
              <PromptProjectSelect
                value={projectId}
                onChange={(nextProjectId) => void confirmScopeChange(() => setProjectId(nextProjectId))}
                allowEmpty
                ariaLabel={t("settingsPrompt.apply")}
              />
            </label>
          </div>

          {error ? <div role="alert" className="text-small text-error">{error}</div> : null}

          <div className="grid min-w-0 gap-2">
            {draft.blocks.map((block, index) => (
              <AssemblyBlockRow
                key={block.id}
                block={block}
                index={index}
                count={draft.blocks.length}
                summary={selected}
                disabled={busy}
                onChange={(update) => updateBlock(block.id, update)}
                onMove={(offset) => moveBlock(index, offset)}
                onDelete={() => setDraft((current) => ({
                  blocks: current.blocks.filter((item) => item.id !== block.id),
                }))}
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
              <BookButton variant="secondary" size="sm" onClick={() => addBlock("template")} disabled={busy || draft.blocks.length >= 64 || selected.available_templates.length === 0}>
                <Plus size={13} />{t("settingsPrompt.assemblyAddTemplate")}
              </BookButton>
              <BookButton variant="secondary" size="sm" onClick={() => addBlock("variable")} disabled={busy || draft.blocks.length >= 64 || selected.variables.length === 0}>
                <Plus size={13} />{t("settingsPrompt.assemblyAddVariable")}
              </BookButton>
              <BookButton variant="secondary" size="sm" onClick={() => addBlock("custom")} disabled={busy || draft.blocks.length >= 64}>
                <Plus size={13} />{t("settingsPrompt.assemblyAddCustom")}
              </BookButton>
            </div>
            {dirty ? (
              <span className="min-w-0 text-small text-warning sm:flex-1">
                {t("settingsPrompt.assemblyUnsaved")}
              </span>
            ) : null}
            <div className="flex min-w-0 flex-wrap gap-2 sm:ml-auto">
              <BookButton variant="secondary" size="sm" onClick={() => void reset()} disabled={busy}>
                {resetting ? <LoadingSpinner size="sm" /> : <RotateCcw size={13} />}
                {t("settingsPrompt.assemblyResetAction")}
              </BookButton>
              <BookButton variant="primary" size="sm" onClick={() => void save()} disabled={busy || !dirty || draft.blocks.length === 0 || !draft.blocks.some((block) => block.enabled)}>
                {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
                {t("common.save")}
              </BookButton>
            </div>
          </div>
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
  disabled: boolean;
  onChange: (update: Partial<PromptAssemblyBlock>) => void;
  onMove: (offset: -1 | 1) => void;
  onDelete: () => void;
}> = ({ block, index, count, summary, disabled, onChange, onMove, onDelete }) => {
  const { t } = useI18n();
  const kindLabel = t(
    block.kind === "template"
      ? "settingsPrompt.assemblyTemplate"
      : block.kind === "variable"
        ? "settingsPrompt.assemblyVariable"
        : "settingsPrompt.assemblyCustom",
  );
  return (
    <div className={`grid min-w-0 gap-2 border-b border-book-border/35 py-2 sm:grid-cols-[auto_minmax(9rem,0.55fr)_minmax(0,1fr)_auto] ${block.enabled ? "" : "opacity-60"}`}>
      <label className="flex min-h-9 items-center gap-2 text-small text-book-text-muted">
        <input type="checkbox" checked={block.enabled} disabled={disabled} onChange={(event) => onChange({ enabled: event.target.checked })} aria-label={t("settingsPrompt.assemblyEnabled")} />
        <span>{index + 1}</span>
      </label>
      <label className="grid min-w-0 gap-1 text-small text-book-text-muted">
        <span>{kindLabel}</span>
        <select
          className="pyqt-input min-w-0 w-full"
          value={block.kind}
          disabled={disabled}
          onChange={(event) => {
            const kind = event.target.value as PromptAssemblyBlockKind;
            const value = kind === "template"
              ? summary.available_templates[0]?.name ?? ""
              : kind === "variable"
                ? summary.variables[0] ?? ""
                : `{{ ${summary.variables[0] ?? ""} }}`;
            onChange({ kind, value });
          }}
        >
          <option value="template">{t("settingsPrompt.assemblyTemplate")}</option>
          <option value="variable">{t("settingsPrompt.assemblyVariable")}</option>
          <option value="custom">{t("settingsPrompt.assemblyCustom")}</option>
        </select>
      </label>
      {block.kind === "template" ? (
        <select className="pyqt-input min-w-0 w-full self-end" value={block.value} disabled={disabled} onChange={(event) => onChange({ value: event.target.value })} aria-label={t("settingsPrompt.assemblyTemplateSelect")}>
          {summary.available_templates.map((template) => <option key={template.name} value={template.name}>{template.title || template.name}</option>)}
        </select>
      ) : block.kind === "variable" ? (
        <select className="pyqt-input min-w-0 w-full self-end" value={block.value} disabled={disabled} onChange={(event) => onChange({ value: event.target.value })} aria-label={t("settingsPrompt.assemblyVariableSelect")}>
          {summary.variables.map((variable) => <option key={variable} value={variable}>{`{{ ${variable} }}`}</option>)}
        </select>
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
