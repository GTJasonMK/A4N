import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowRight, FilePlus2, Save, Sparkles, Upload } from "lucide-react";
import {
  generateCanvasTextAssetDraft,
  listCanvasTextAssetVersions,
  listCanvasTextAssets,
  previewCanvasTextAssetDraftRequest,
  saveCanvasTextAsset,
  type CanvasTextAssetSummary,
  type CanvasTextAssetVersionSummary,
} from "../../api/canvas";
import { listLlmConfig } from "../../api/config/llm";
import type { LlmConfigSummary } from "../../api/config/types";
import { importProjectTextAssetUpload } from "../../api/import";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { useI18n } from "../../i18n";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";

export interface ProjectTextSourceSelection {
  assetId: number;
  version: number;
  startSection: number;
  endSection: number;
}

interface ProjectTextMaterialsPanelProps {
  projectId: number;
  canEdit: boolean;
  onDirtyChange?: (dirty: boolean) => void;
  onSelectSource?: (selection: ProjectTextSourceSelection) => void;
  selectedSource?: ProjectTextSourceSelection | null;
}

interface TextSection {
  title: string;
  start: number;
}

export function textAssetSections(content: string): TextSection[] {
  const sections: TextSection[] = [];
  const heading = /^(?:#{1,6}\s+.+|第.{1,32}(?:章|节|幕|集).*)$/gm;
  for (const match of content.matchAll(heading)) {
    if (match.index === undefined) continue;
    sections.push({ title: match[0].replace(/^#{1,6}\s+/, "").trim(), start: match.index });
  }
  return sections.length ? sections : [{ title: "全文", start: 0 }];
}

export const ProjectTextMaterialsPanel: React.FC<ProjectTextMaterialsPanelProps> = ({
  projectId,
  canEdit,
  onDirtyChange,
  onSelectSource,
  selectedSource,
}) => {
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const [assets, setAssets] = useState<CanvasTextAssetSummary[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [baselineTitle, setBaselineTitle] = useState("");
  const [baselineContent, setBaselineContent] = useState("");
  const [sourceVersion, setSourceVersion] = useState<number | null>(null);
  const [versions, setVersions] = useState<CanvasTextAssetVersionSummary[]>([]);
  const [startSection, setStartSection] = useState(0);
  const [endSection, setEndSection] = useState(0);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [importing, setImporting] = useState(false);
  const [llmConfigs, setLlmConfigs] = useState<LlmConfigSummary[]>([]);
  const [llmConfigId, setLlmConfigId] = useState<number | null>(null);
  const [aiInstruction, setAiInstruction] = useState("");
  const [requestPreview, setRequestPreview] = useState<ModelRequestPreview | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [draftInput, setDraftInput] = useState<import("../../api/generated/host").CanvasTextAssetDraftInput | null>(null);
  const [draftReviewed, setDraftReviewed] = useState(false);
  const [generatingDraft, setGeneratingDraft] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestRef = useRef(0);
  const selectedIdRef = useRef<number | null>(null);
  const selected = assets.find((asset) => asset.id === selectedId) ?? null;
  const sections = useMemo(() => textAssetSections(content), [content]);
  const dirty = selected
    ? title !== baselineTitle || content !== baselineContent
    : title.trim().length > 0 || content.trim().length > 0;

  const load = useCallback(async (preferId?: number, preferVersion?: number) => {
    const request = requestRef.current + 1;
    requestRef.current = request;
    setLoading(true);
    setError(null);
    try {
      const result = await listCanvasTextAssets(projectId);
      if (requestRef.current !== request) return;
      setAssets(result);
      const next = result.find((asset) => asset.id === (preferId ?? selectedSource?.assetId ?? selectedIdRef.current)) ?? result[0] ?? null;
      const nextVersions = next ? await listCanvasTextAssetVersions(projectId, next.id) : [];
      if (requestRef.current !== request) return;
      const selectedVersion = next
        ? preferVersion ?? (selectedSource?.assetId === next.id ? selectedSource.version : next.current_version)
        : null;
      const version = nextVersions.find((item) => item.version === selectedVersion) ?? null;
      const nextContent = version?.content ?? next?.content ?? "";
      const nextSections = textAssetSections(nextContent);
      const sourceRange =
        selectedSource != null &&
        selectedSource.assetId === next?.id &&
        selectedSource.version === selectedVersion
          ? selectedSource
          : null;
      selectedIdRef.current = next?.id ?? null;
      setSelectedId(next?.id ?? null);
      setTitle(next?.title ?? "");
      setBaselineTitle(next?.title ?? "");
      setContent(nextContent);
      setBaselineContent(nextContent);
      setSourceVersion(version?.version ?? next?.current_version ?? null);
      setStartSection(
        sourceRange
          ? Math.min(sourceRange.startSection, nextSections.length - 1)
          : 0,
      );
      setEndSection(
        sourceRange
          ? Math.min(sourceRange.endSection, nextSections.length - 1)
          : nextSections.length - 1,
      );
      setVersions(nextVersions);
    } catch (cause) {
      if (requestRef.current === request) {
        const message = extractErrorMessage(cause, t("novel.textMaterialsLoadFailed"));
        setError(message);
      }
    } finally {
      if (requestRef.current === request) setLoading(false);
    }
  }, [projectId, selectedSource, t]);

  useEffect(() => {
    void load();
    return () => { requestRef.current += 1; };
  }, [load]);
  useEffect(() => {
    void listLlmConfig()
      .then((configs) => {
        setLlmConfigs(configs);
        const defaultConfig = configs.find((config) => config.is_default) ?? configs[0];
        if (defaultConfig) setLlmConfigId(defaultConfig.id);
      })
      .catch((cause) => addToast(extractErrorMessage(cause, t("novel.textMaterialsConfigLoadFailed")), "error"));
  }, [addToast, t]);
  useEffect(() => onDirtyChange?.(dirty), [dirty, onDirtyChange]);
  useEffect(() => {
    setStartSection((current) => Math.min(current, sections.length - 1));
    setEndSection((current) => Math.min(current, sections.length - 1));
  }, [sections.length]);

  const chooseAsset = async (asset: CanvasTextAssetSummary) => {
    if (dirty && !await confirmDialog({
      title: t("novel.textMaterialsDiscardTitle"),
      message: t("novel.textMaterialsDiscardChanges"),
    })) return;
    const request = requestRef.current + 1;
    requestRef.current = request;
    selectedIdRef.current = asset.id;
    setSelectedId(asset.id);
    setTitle(asset.title);
    setContent(asset.content);
    setBaselineTitle(asset.title);
    setBaselineContent(asset.content);
    setSourceVersion(asset.current_version);
    setStartSection(0);
    setEndSection(textAssetSections(asset.content).length - 1);
    try {
      const nextVersions = await listCanvasTextAssetVersions(projectId, asset.id);
      if (requestRef.current !== request || selectedIdRef.current !== asset.id) return;
      setVersions(nextVersions);
    } catch (cause) {
      if (requestRef.current !== request || selectedIdRef.current !== asset.id) return;
      addToast(extractErrorMessage(cause, t("novel.textMaterialsHistoryFailed")), "error");
      setVersions([]);
    }
  };

  const createNew = () => {
    void (async () => {
      if (dirty && !await confirmDialog({
        title: t("novel.textMaterialsDiscardTitle"),
        message: t("novel.textMaterialsDiscardChanges"),
      })) return;
      requestRef.current += 1;
      selectedIdRef.current = null;
      setSelectedId(null);
      setTitle("");
      setContent("");
      setBaselineTitle("");
      setBaselineContent("");
      setSourceVersion(null);
      setVersions([]);
      setStartSection(0);
      setEndSection(0);
    })();
  };

  const save = async () => {
    if (!canEdit || saving || !title.trim() || !content.trim()) return;
    setSaving(true);
    try {
      const asset = await saveCanvasTextAsset(projectId, {
        id: selected?.id ?? null,
        expected_version: selected?.current_version ?? null,
        title: title.trim(),
        text_kind: "script",
        content,
      });
      addToast(t("novel.textMaterialsSaved", { version: asset.current_version }), "success");
      await load(asset.id, asset.current_version);
    } catch (cause) {
      addToast(extractErrorMessage(cause, t("novel.textMaterialsSaveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  const importFile = async () => {
    if (!canEdit || importing) return;
    if (dirty && !await confirmDialog({
      title: t("novel.textMaterialsDiscardTitle"),
      message: t("novel.textMaterialsDiscardChanges"),
    })) return;
    const file = await pickLocalFile(".txt,.md,.markdown,.epub,.docx");
    if (!file) return;
    setImporting(true);
    try {
      const asset = await importProjectTextAssetUpload({
        project_id: projectId,
        file_name: file.name,
        data_base64: await readFileAsBase64(file),
      });
      addToast(t("novel.textMaterialsImported", { title: asset.title }), "success");
      await load(asset.id);
    } catch (cause) {
      addToast(extractErrorMessage(cause, t("novel.textMaterialsImportFailed")), "error");
    } finally {
      setImporting(false);
    }
  };

  const prepareDraft = async () => {
    if (!canEdit || requestPreviewLoading || generatingDraft || !aiInstruction.trim()) return;
    const input = {
      project_id: projectId,
      llm_config_id: llmConfigId,
      instruction: aiInstruction.trim(),
      source_asset_id: selected?.id ?? null,
      source_version: selected ? sourceVersion ?? selected.current_version : null,
    } satisfies import("../../api/generated/host").CanvasTextAssetDraftInput;
    setRequestPreviewLoading(true);
    setDraftInput(null);
    setDraftReviewed(false);
    try {
      const preview = await previewCanvasTextAssetDraftRequest(input);
      setDraftInput(input);
      setRequestPreview(preview);
      setRequestPreviewOpen(true);
    } catch (cause) {
      addToast(extractErrorMessage(cause, t("novel.textMaterialsPreviewFailed")), "error");
    } finally {
      setRequestPreviewLoading(false);
    }
  };

  const generateDraft = async () => {
    if (!draftReviewed || !draftInput || !canEdit || generatingDraft) return;
    setGeneratingDraft(true);
    try {
      const draft = await generateCanvasTextAssetDraft(draftInput);
      setContent(draft.content);
      setDraftReviewed(false);
      setDraftInput(null);
      setRequestPreview(null);
      setRequestPreviewOpen(false);
      addToast(t("novel.textMaterialsDraftReady"), "success");
    } catch (cause) {
      addToast(extractErrorMessage(cause, t("novel.textMaterialsDraftFailed")), "error");
    } finally {
      setGeneratingDraft(false);
    }
  };

  const loadVersion = (version: CanvasTextAssetVersionSummary) => {
    void (async () => {
      if (dirty && !await confirmDialog({
        title: t("novel.textMaterialsDiscardTitle"),
        message: t("novel.textMaterialsDiscardChanges"),
      })) return;
      setContent(version.content);
      setBaselineContent(version.content);
      setSourceVersion(version.version);
    })();
  };

  const selectSource = () => {
    if (!selected || !onSelectSource || dirty) return;
    onSelectSource({
      assetId: selected.id,
      version: sourceVersion ?? selected.current_version,
      startSection,
      endSection,
    });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex flex-wrap items-center gap-2 border-b border-book-border/70 pb-2">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold text-book-text-main">{t("novel.textMaterialsTitle")}</h2>
          <p className="truncate text-small text-book-text-muted">
            {selected ? t("novel.textMaterialsVersion", { version: sourceVersion ?? selected.current_version }) : t("novel.textMaterialsCount", { count: assets.length })}
          </p>
        </div>
        <BookIconButton label={t("novel.textMaterialsNew")} onClick={createNew} disabled={!canEdit || saving || importing} variant="secondary" size="sm">
          <FilePlus2 size={14} />
        </BookIconButton>
        <BookButton variant="secondary" size="sm" onClick={() => void importFile()} disabled={!canEdit || importing || saving}>
          {importing ? <LoadingSpinner size="sm" /> : <Upload size={14} />}
          {t("novel.textMaterialsImport")}
        </BookButton>
        <BookButton variant="primary" size="sm" onClick={() => void save()} disabled={!canEdit || !dirty || saving || !title.trim() || !content.trim()}>
          {saving ? <LoadingSpinner size="sm" /> : <Save size={14} />}
          {t("common.save")}
        </BookButton>
      </header>
      {error ? (
        <div role="alert" className="flex items-center gap-2 border-b border-book-border/70 py-2 text-sm text-error">
          <span className="min-w-0 flex-1">{error}</span>
          <BookButton size="sm" variant="secondary" onClick={() => void load()} disabled={loading}>{t("common.retry")}</BookButton>
        </div>
      ) : null}
      <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[220px_minmax(0,1fr)]">
        <nav aria-label={t("novel.textMaterialsTitle")} className="max-h-36 overflow-y-auto border-b border-book-border/70 md:max-h-none md:border-b-0 md:border-r">
          {loading ? <div className="p-3"><LoadingSpinner size="sm" /></div> : assets.map((asset) => (
            <button
              key={asset.id}
              type="button"
              aria-pressed={selectedId === asset.id}
              onClick={() => void chooseAsset(asset)}
              className={`block w-full truncate border-b border-book-border/40 px-3 py-2 text-left text-sm ${selectedId === asset.id ? "bg-book-primary/10 text-book-primary" : "text-book-text-main hover:bg-book-bg-tertiary/40"}`}
            >
              <span className="block truncate">{asset.title}</span>
              <span className="text-small text-book-text-muted">{t("novel.textMaterialsVersion", { version: asset.current_version })}</span>
            </button>
          ))}
          {!loading && assets.length === 0 ? <p className="px-3 py-4 text-small text-book-text-muted">{t("novel.textMaterialsEmpty")}</p> : null}
        </nav>
        <div className="flex min-h-0 min-w-0 flex-col gap-3 overflow-y-auto p-3">
          <div className="flex flex-wrap items-end gap-2 border-b border-book-border/70 pb-3">
            <label className="min-w-44 flex-1 text-small text-book-text-muted">
              {t("novel.textMaterialsLlmConfig")}
              <ThemedSelect
                aria-label={t("novel.textMaterialsLlmConfig")}
                value={llmConfigId ?? "default"}
                onChange={(value) => setLlmConfigId(value === "default" ? null : Number(value))}
                disabled={!canEdit || requestPreviewLoading || generatingDraft}
                className="pyqt-input mt-1 w-full min-w-0 text-sm"
              >
                <option value="default">{t("novel.textMaterialsUseDefaultConfig")}</option>
                {llmConfigs.map((config) => <option key={config.id} value={config.id}>{config.name}</option>)}
              </ThemedSelect>
            </label>
            <BookButton size="sm" variant="secondary" onClick={() => void prepareDraft()} disabled={!canEdit || !aiInstruction.trim() || requestPreviewLoading || generatingDraft || dirty}>
              {requestPreviewLoading ? <LoadingSpinner size="sm" /> : <Sparkles size={14} />}
              {t("novel.textMaterialsPreviewDraft")}
            </BookButton>
            {draftReviewed ? (
              <BookButton size="sm" variant="primary" onClick={() => void generateDraft()} disabled={generatingDraft}>
                {generatingDraft ? <LoadingSpinner size="sm" /> : <Sparkles size={14} />}
                {t("novel.textMaterialsGenerateDraft")}
              </BookButton>
            ) : null}
          </div>
          <label className="block text-small text-book-text-muted">
            {t("novel.textMaterialsInstruction")}
            <textarea value={aiInstruction} onChange={(event) => { setAiInstruction(event.target.value); setDraftInput(null); setDraftReviewed(false); setRequestPreview(null); }} disabled={!canEdit || requestPreviewLoading || generatingDraft} rows={3} className="pyqt-input mt-1 w-full resize-y text-sm leading-relaxed" />
          </label>
          <label className="block text-small text-book-text-muted">
            {t("novel.textMaterialsName")}
            <input value={title} onChange={(event) => setTitle(event.target.value)} disabled={!canEdit || loading} className="pyqt-input mt-1 w-full text-sm" />
          </label>
          <label className="flex min-h-48 flex-1 flex-col text-small text-book-text-muted">
            {t("novel.textMaterialsContent")}
            <textarea value={content} onChange={(event) => setContent(event.target.value)} disabled={!canEdit || loading} rows={12} className="pyqt-input mt-1 min-h-48 flex-1 resize-y text-sm leading-relaxed" />
          </label>
          {onSelectSource && selected ? (
            <div className="flex flex-wrap items-end gap-2 border-t border-book-border/70 pt-3">
              <label className="min-w-0 flex-1 text-small text-book-text-muted">
                {t("novel.textMaterialsFrom")}
                <ThemedSelect
                  aria-label={t("novel.textMaterialsFrom")}
                  value={startSection}
                  onChange={(value) => setStartSection(Number(value))}
                  disabled={!canEdit}
                  className="pyqt-input mt-1 w-full min-w-0 text-sm"
                >
                  {sections.map((section, index) => <option key={`${index}:${section.start}`} value={index}>{section.title}</option>)}
                </ThemedSelect>
              </label>
              <label className="min-w-0 flex-1 text-small text-book-text-muted">
                {t("novel.textMaterialsTo")}
                <ThemedSelect
                  aria-label={t("novel.textMaterialsTo")}
                  value={endSection}
                  onChange={(value) => setEndSection(Number(value))}
                  disabled={!canEdit}
                  className="pyqt-input mt-1 w-full min-w-0 text-sm"
                >
                  {sections.map((section, index) => <option key={`${index}:${section.start}`} value={index}>{section.title}</option>)}
                </ThemedSelect>
              </label>
              <BookButton variant="secondary" size="sm" onClick={selectSource} disabled={dirty || startSection > endSection}>
                <ArrowRight size={14} />{selectedSource?.assetId === selected.id && selectedSource.version === (sourceVersion ?? selected.current_version) && selectedSource.startSection === startSection && selectedSource.endSection === endSection ? t("novel.textMaterialsSourceSelected") : t("novel.textMaterialsUseSource")}
              </BookButton>
            </div>
          ) : null}
          {selected && versions.length > 1 ? (
            <details className="border-t border-book-border/70 pt-2 text-small">
              <summary className="cursor-pointer text-book-text-muted">{t("novel.textMaterialsHistory", { count: versions.length })}</summary>
              <ul className="mt-2 divide-y divide-book-border/40">
                {versions.map((version) => (
                  <li key={version.version} className="flex items-center justify-between gap-2 py-1.5">
                    <span className="text-book-text-main">{t("novel.textMaterialsVersion", { version: version.version })}</span>
                    {version.version !== (sourceVersion ?? selected.current_version) ? (
                      <BookButton size="sm" variant="secondary" onClick={() => loadVersion(version)} disabled={!canEdit || saving}>
                        {t("novel.textMaterialsRestore")}
                      </BookButton>
                    ) : null}
                  </li>
                ))}
              </ul>
            </details>
          ) : null}
        </div>
      </div>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreview}
        onClose={() => {
          setRequestPreviewOpen(false);
          setDraftReviewed(Boolean(requestPreview && draftInput));
        }}
      />
    </div>
  );
};
