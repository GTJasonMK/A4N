import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ArrowRight, FilePlus2, Save, Sparkles, Upload } from "lucide-react";
import {
  generateCanvasTextAssetDraft,
  listCanvasTextAssetVersions,
  listCanvasTextAssets,
  saveCanvasTextAsset,
  type CanvasTextAssetSummary,
  type CanvasTextAssetVersionSummary,
} from "../../api/canvas";
import { importProjectTextAssetUpload } from "../../api/import";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { Modal } from "../../components/ui/Modal";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { useI18n } from "../../i18n";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";
import { useNovelDetailDirty } from "../novel-detail/novelDetailDirty";
import { EmptyHint } from "../novel-detail/shared";

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
  const [aiInstruction, setAiInstruction] = useState("");
  const [aiDraftContent, setAiDraftContent] = useState("");
  const [aiAssistantOpen, setAiAssistantOpen] = useState(false);
  const [sourceDialogOpen, setSourceDialogOpen] = useState(false);
  const [generatingDraft, setGeneratingDraft] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [assetsLoaded, setAssetsLoaded] = useState(false);
  const requestRef = useRef(0);
  const selectedIdRef = useRef<number | null>(null);
  const activeProjectIdRef = useRef(projectId);
  activeProjectIdRef.current = projectId;
  const selected = assets.find((asset) => asset.id === selectedId) ?? null;
  const sections = useMemo(() => textAssetSections(content), [content]);
  const editorDirty = selected
    ? title !== baselineTitle || content !== baselineContent
    : title.trim().length > 0 || content.trim().length > 0;
  const assistantDraftDirty = aiInstruction.length > 0 || aiDraftContent.length > 0;
  const dirty = editorDirty || assistantDraftDirty;
  const actionBusy = saving || importing || generatingDraft;
  const discardDraft = () => {
    if (actionBusy) return false;
    setTitle(baselineTitle);
    setContent(baselineContent);
    setAiInstruction("");
    setAiDraftContent("");
    setAiAssistantOpen(false);
  };
  useNovelDetailDirty(
    `project-text-material:${projectId}`,
    dirty || actionBusy,
    discardDraft,
    () => !actionBusy,
  );

  const load = useCallback(async (preferId?: number, preferVersion?: number): Promise<boolean> => {
    if (activeProjectIdRef.current !== projectId) return false;
    const request = requestRef.current + 1;
    requestRef.current = request;
    setLoading(true);
    setError(null);
    try {
      const result = await listCanvasTextAssets(projectId);
      if (activeProjectIdRef.current !== projectId || requestRef.current !== request) return false;
      setAssets(result);
      setAssetsLoaded(true);
      const next = result.find((asset) => asset.id === (preferId ?? selectedSource?.assetId ?? selectedIdRef.current)) ?? result[0] ?? null;
      const nextVersions = next ? await listCanvasTextAssetVersions(projectId, next.id) : [];
      if (activeProjectIdRef.current !== projectId || requestRef.current !== request) return false;
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
      return true;
    } catch (cause) {
      if (activeProjectIdRef.current === projectId && requestRef.current === request) {
        const message = extractErrorMessage(cause, t("novel.textMaterialsLoadFailed"));
        setError(message);
      }
      return false;
    } finally {
      if (activeProjectIdRef.current === projectId && requestRef.current === request) setLoading(false);
    }
  }, [projectId, selectedSource, t]);

  useLayoutEffect(() => {
    requestRef.current += 1;
    selectedIdRef.current = null;
    setAssets([]);
    setAssetsLoaded(false);
    setSelectedId(null);
    setTitle("");
    setContent("");
    setBaselineTitle("");
    setBaselineContent("");
    setSourceVersion(null);
    setVersions([]);
    setStartSection(0);
    setEndSection(0);
    setAiInstruction("");
    setAiDraftContent("");
    setAiAssistantOpen(false);
    setSourceDialogOpen(false);
    setError(null);
    setLoading(true);
    setSaving(false);
    setImporting(false);
    setGeneratingDraft(false);
  }, [projectId]);

  useEffect(() => {
    void load();
    return () => { requestRef.current += 1; };
  }, [load]);
  useEffect(() => onDirtyChange?.(dirty), [dirty, onDirtyChange]);
  useEffect(() => {
    setStartSection((current) => Math.min(current, sections.length - 1));
    setEndSection((current) => Math.min(current, sections.length - 1));
  }, [sections.length]);

  const confirmDraftDiscard = () => confirmDialog({
    title: t("novel.textMaterialsDiscardTitle"),
    message: t("novel.textMaterialsDiscardChanges"),
  });

  const applyAssetResult = (asset: CanvasTextAssetSummary) => {
    selectedIdRef.current = asset.id;
    setAssets((current) => {
      const index = current.findIndex((item) => item.id === asset.id);
      if (index < 0) return [asset, ...current];
      return current.map((item, itemIndex) => itemIndex === index ? asset : item);
    });
    setSelectedId(asset.id);
    setTitle(asset.title);
    setBaselineTitle(asset.title);
    setContent(asset.content);
    setBaselineContent(asset.content);
    setSourceVersion(asset.current_version);
    setStartSection(0);
    setEndSection(textAssetSections(asset.content).length - 1);
    setVersions([]);
  };

  const retryLoad = async () => {
    if (saving || importing) return;
    if (dirty && !await confirmDraftDiscard()) return;
    if (await load()) {
      setAiInstruction("");
      setAiDraftContent("");
    }
  };

  const chooseAsset = async (asset: CanvasTextAssetSummary) => {
    const requestProjectId = projectId;
    if (saving || importing || loading || activeProjectIdRef.current !== requestProjectId) return;
    if (dirty && !await confirmDraftDiscard()) return;
    if (activeProjectIdRef.current !== requestProjectId) return;
    setAiInstruction("");
    setAiDraftContent("");
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
      if (activeProjectIdRef.current !== requestProjectId || requestRef.current !== request || selectedIdRef.current !== asset.id) return;
      setVersions(nextVersions);
    } catch (cause) {
      if (activeProjectIdRef.current !== requestProjectId || requestRef.current !== request || selectedIdRef.current !== asset.id) return;
      addToast(extractErrorMessage(cause, t("novel.textMaterialsHistoryFailed")), "error");
      setVersions([]);
    }
  };

  const createNew = () => {
    void (async () => {
      if (dirty && !await confirmDraftDiscard()) return;
      if (activeProjectIdRef.current !== projectId || loading || saving || importing) return;
      requestRef.current += 1;
      selectedIdRef.current = null;
      setAiInstruction("");
      setAiDraftContent("");
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
    if (!canEdit || !assetsLoaded || loading || saving || importing || !title.trim() || !content.trim()) return;
    const requestProjectId = projectId;
    const requestTitle = title.trim();
    const requestContent = content;
    setSaving(true);
    try {
      const asset = await saveCanvasTextAsset(requestProjectId, {
        id: selected?.id ?? null,
        expected_version: selected?.current_version ?? null,
        title: requestTitle,
        text_kind: "script",
        content: requestContent,
      });
      if (activeProjectIdRef.current !== requestProjectId) return;
      applyAssetResult(asset);
      addToast(t("novel.textMaterialsSaved", { version: asset.current_version }), "success");
      void load(asset.id, asset.current_version);
    } catch (cause) {
      if (activeProjectIdRef.current !== requestProjectId) return;
      addToast(extractErrorMessage(cause, t("novel.textMaterialsSaveFailed")), "error");
    } finally {
      if (activeProjectIdRef.current === requestProjectId) setSaving(false);
    }
  };

  const importFile = async () => {
    if (!canEdit || !assetsLoaded || loading || importing || saving) return;
    const requestProjectId = projectId;
    if (dirty && !await confirmDraftDiscard()) return;
    if (activeProjectIdRef.current !== requestProjectId) return;
    const file = await pickLocalFile(".txt,.md,.markdown,.epub,.docx");
    if (!file || activeProjectIdRef.current !== requestProjectId) return;
    setImporting(true);
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (activeProjectIdRef.current !== requestProjectId) return;
      const asset = await importProjectTextAssetUpload({
        project_id: requestProjectId,
        file_name: file.name,
        data_base64: dataBase64,
      });
      if (activeProjectIdRef.current !== requestProjectId) return;
      setAiInstruction("");
      setAiDraftContent("");
      applyAssetResult(asset);
      addToast(t("novel.textMaterialsImported", { title: asset.title }), "success");
      void load(asset.id);
    } catch (cause) {
      if (activeProjectIdRef.current !== requestProjectId) return;
      addToast(extractErrorMessage(cause, t("novel.textMaterialsImportFailed")), "error");
    } finally {
      if (activeProjectIdRef.current === requestProjectId) setImporting(false);
    }
  };

  const openAiAssistant = () => {
    if (!canEdit || saving || importing || generatingDraft) return;
    if (editorDirty) {
      addToast(t("novel.textMaterialsSaveBeforeAi"), "info");
      return;
    }
    setAiAssistantOpen(true);
  };

  const generateDraft = async () => {
    if (!canEdit || saving || importing || generatingDraft || !aiInstruction.trim()) return;
    if (editorDirty) {
      addToast(t("novel.textMaterialsSaveBeforeAi"), "info");
      return;
    }
    const requestProjectId = projectId;
    setGeneratingDraft(true);
    try {
      const draft = await generateCanvasTextAssetDraft({
        project_id: requestProjectId,
        llm_config_id: null,
        instruction: aiInstruction.trim(),
        source_asset_id: selected?.id ?? null,
        source_version: selected ? sourceVersion ?? selected.current_version : null,
      });
      if (activeProjectIdRef.current !== requestProjectId) return;
      setAiDraftContent(draft.content);
    } catch (cause) {
      if (activeProjectIdRef.current !== requestProjectId) return;
      addToast(extractErrorMessage(cause, t("novel.textMaterialsDraftFailed")), "error");
    } finally {
      if (activeProjectIdRef.current === requestProjectId) setGeneratingDraft(false);
    }
  };

  const applyDraft = () => {
    if (!aiDraftContent) return;
    setContent(aiDraftContent);
    setAiAssistantOpen(false);
    setAiInstruction("");
    setAiDraftContent("");
    addToast(t("novel.textMaterialsDraftApplied"), "success");
  };

  const cancelAiAssistant = () => {
    setAiAssistantOpen(false);
    setAiInstruction("");
    setAiDraftContent("");
  };

  const loadVersion = (version: CanvasTextAssetVersionSummary) => {
    void (async () => {
      const requestProjectId = projectId;
      if (saving || importing || loading) return;
      if (dirty && !await confirmDraftDiscard()) return;
      if (activeProjectIdRef.current !== requestProjectId) return;
      setAiInstruction("");
      setAiDraftContent("");
      setContent(version.content);
      setTitle(selected?.title ?? baselineTitle);
      setBaselineTitle(selected?.title ?? baselineTitle);
      setBaselineContent(selected?.content ?? baselineContent);
      setSourceVersion(version.version);
    })();
  };

  const selectSource = () => {
    if (!selected || !onSelectSource || dirty || saving || importing || loading) return;
    onSelectSource({
      assetId: selected.id,
      version: sourceVersion ?? selected.current_version,
      startSection,
      endSection,
    });
    setSourceDialogOpen(false);
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
        <BookIconButton label={t("novel.textMaterialsNew")} onClick={createNew} disabled={!canEdit || !assetsLoaded || loading || saving || importing} variant="secondary" size="sm">
          <FilePlus2 size={14} />
        </BookIconButton>
        <BookButton variant="secondary" size="sm" onClick={() => void importFile()} disabled={!canEdit || !assetsLoaded || loading || importing || saving}>
          {importing ? <LoadingSpinner size="sm" /> : <Upload size={14} />}
          {t("novel.textMaterialsImport")}
        </BookButton>
        <BookButton variant="primary" size="sm" onClick={() => void save()} disabled={!canEdit || !assetsLoaded || loading || !editorDirty || saving || importing || !title.trim() || !content.trim()}>
          {saving ? <LoadingSpinner size="sm" /> : <Save size={14} />}
          {t("common.save")}
        </BookButton>
        <BookButton
          variant="secondary"
          size="sm"
          onClick={openAiAssistant}
          disabled={!canEdit || saving || importing || generatingDraft}
        >
          <Sparkles size={14} />
          {t("novel.textMaterialsAiAssist")}
        </BookButton>
        {onSelectSource && selected ? (
          <BookButton variant="secondary" size="sm" onClick={() => setSourceDialogOpen(true)} disabled={!canEdit || dirty || saving || importing || loading}>
            <ArrowRight size={14} />
            {selectedSource?.assetId === selected.id && selectedSource.version === (sourceVersion ?? selected.current_version) && selectedSource.startSection === startSection && selectedSource.endSection === endSection
              ? t("novel.textMaterialsSourceSelected")
              : t("novel.textMaterialsUseSource")}
          </BookButton>
        ) : null}
      </header>
      {error ? (
        <div role="alert" className="flex items-center gap-2 border-b border-book-border/70 py-2 text-sm text-error">
          <span className="min-w-0 flex-1">{error}</span>
          <BookButton size="sm" variant="secondary" onClick={() => void retryLoad()} disabled={loading || saving || importing}>{t("common.retry")}</BookButton>
        </div>
      ) : null}
      <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[220px_minmax(0,1fr)]">
        <nav aria-label={t("novel.textMaterialsTitle")} className="max-h-36 overflow-y-auto border-b border-book-border/70 bg-book-bg-paper/25 p-1 md:max-h-none md:border-b-0 md:border-r custom-scrollbar">
          {loading ? <div className="p-3"><LoadingSpinner size="sm" /></div> : assets.map((asset) => (
            <button
              key={asset.id}
              type="button"
              aria-pressed={selectedId === asset.id}
              onClick={() => void chooseAsset(asset)}
              disabled={saving || importing}
              data-active={selectedId === asset.id ? "1" : undefined}
              className="a4n-sidebar-item block w-full truncate text-left text-sm"
            >
              <span className="block truncate">{asset.title}</span>
              <span className="text-small text-book-text-muted">{t("novel.textMaterialsVersion", { version: asset.current_version })}</span>
            </button>
          ))}
          {!loading && assetsLoaded && assets.length === 0 ? <EmptyHint text={t("novel.textMaterialsEmpty")} /> : null}
        </nav>
        <div className="flex min-h-0 min-w-0 flex-col gap-3 overflow-y-auto p-3">
          <label className="block text-small text-book-text-muted">
            <span>{t("novel.textMaterialsName")} <span className="text-warning">{t("novel.textMaterialsRequired")}</span></span>
            <input required value={title} onChange={(event) => setTitle(event.target.value)} disabled={!canEdit || loading || saving || importing || !assetsLoaded} className="pyqt-input mt-1 w-full text-sm" />
          </label>
          <label className="flex min-h-48 flex-1 flex-col text-small text-book-text-muted">
            <span>{t("novel.textMaterialsContent")} <span className="text-warning">{t("novel.textMaterialsRequired")}</span></span>
            <textarea required value={content} onChange={(event) => setContent(event.target.value)} disabled={!canEdit || loading || saving || importing || !assetsLoaded} rows={12} className="pyqt-input mt-1 min-h-48 flex-1 resize-y text-sm leading-relaxed" />
          </label>
          {selected && versions.length > 1 ? (
            <details className="border-t border-book-border/70 pt-2 text-small">
              <summary className="cursor-pointer text-book-text-muted">{t("novel.textMaterialsHistory", { count: versions.length })}</summary>
              <ul className="mt-2 divide-y divide-book-border/40">
                {versions.map((version) => (
                  <li key={version.version} className="flex items-center justify-between gap-2 py-1.5">
                    <span className="text-book-text-main">{t("novel.textMaterialsVersion", { version: version.version })}</span>
                    {version.version !== (sourceVersion ?? selected.current_version) ? (
                      <BookButton size="sm" variant="secondary" onClick={() => loadVersion(version)} disabled={!canEdit || saving || importing || loading}>
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
      <Modal
        isOpen={aiAssistantOpen}
        onClose={() => { if (!generatingDraft) setAiAssistantOpen(false); }}
        title={t("novel.textMaterialsAiAssistTitle")}
        maxWidthClassName="max-w-3xl"
        showCloseButton={!generatingDraft}
        closeOnBackdrop={!generatingDraft}
        closeOnEscape={!generatingDraft}
        footer={(
          <div className="flex w-full flex-wrap items-center justify-end gap-2">
            <BookButton variant="secondary" size="sm" onClick={cancelAiAssistant} disabled={generatingDraft}>
              {t("common.cancel")}
            </BookButton>
            {aiDraftContent ? (
              <>
                <BookButton variant="secondary" size="sm" onClick={() => void generateDraft()} disabled={generatingDraft || !aiInstruction.trim()}>
                  {generatingDraft ? <LoadingSpinner size="sm" /> : <Sparkles size={14} />}
                  {t("novel.textMaterialsRegenerateDraft")}
                </BookButton>
                <BookButton variant="primary" size="sm" onClick={applyDraft} disabled={generatingDraft}>
                  {t("novel.textMaterialsApplyDraft")}
                </BookButton>
              </>
            ) : (
              <BookButton variant="primary" size="sm" onClick={() => void generateDraft()} disabled={generatingDraft || !aiInstruction.trim()}>
                {generatingDraft ? <LoadingSpinner size="sm" /> : <Sparkles size={14} />}
                {t("novel.textMaterialsGenerateDraft")}
              </BookButton>
            )}
          </div>
        )}
      >
        <div className="flex flex-col gap-3">
          <label className="block text-small text-book-text-muted">
            {t("novel.textMaterialsInstruction")}
            <textarea
              value={aiInstruction}
              onChange={(event) => { setAiInstruction(event.target.value); setAiDraftContent(""); }}
              disabled={generatingDraft}
              placeholder={t("novel.textMaterialsAiInstructionPlaceholder")}
              rows={4}
              className="pyqt-input mt-1 w-full resize-y text-sm leading-relaxed"
            />
          </label>
          {aiDraftContent ? (
            <label className="block text-small text-book-text-muted">
              {t("novel.textMaterialsDraftResult")}
              <textarea value={aiDraftContent} readOnly rows={12} className="pyqt-input mt-1 w-full resize-y text-sm leading-relaxed" />
            </label>
          ) : null}
          {generatingDraft ? (
            <LoadingSpinner size="sm" text={t("novel.textMaterialsGeneratingDraft")} />
          ) : null}
        </div>
      </Modal>
      {onSelectSource && selected ? (
        <Modal
          isOpen={sourceDialogOpen}
          onClose={() => setSourceDialogOpen(false)}
          title={t("novel.textMaterialsSourceDialogTitle")}
          maxWidthClassName="max-w-xl"
          footer={(
            <div className="flex w-full justify-end gap-2">
              <BookButton variant="secondary" size="sm" onClick={() => setSourceDialogOpen(false)}>{t("common.cancel")}</BookButton>
              <BookButton variant="primary" size="sm" onClick={selectSource} disabled={dirty || startSection > endSection}>{t("novel.textMaterialsUseSource")}</BookButton>
            </div>
          )}
        >
          <div className="flex flex-col gap-3">
            <p className="text-small text-book-text-muted">{t("novel.textMaterialsSourceRangeHint")}</p>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="text-small text-book-text-muted">
                {t("novel.textMaterialsFrom")}
                <ThemedSelect aria-label={t("novel.textMaterialsFrom")} value={startSection} onChange={(value) => setStartSection(Number(value))} disabled={!canEdit} className="pyqt-input mt-1 w-full text-sm">
                  {sections.map((section, index) => <option key={`${index}:${section.start}`} value={index}>{section.title}</option>)}
                </ThemedSelect>
              </label>
              <label className="text-small text-book-text-muted">
                {t("novel.textMaterialsTo")}
                <ThemedSelect aria-label={t("novel.textMaterialsTo")} value={endSection} onChange={(value) => setEndSection(Number(value))} disabled={!canEdit} className="pyqt-input mt-1 w-full text-sm">
                  {sections.map((section, index) => <option key={`${index}:${section.start}`} value={index}>{section.title}</option>)}
                </ThemedSelect>
              </label>
            </div>
          </div>
        </Modal>
      ) : null}
    </div>
  );
};
