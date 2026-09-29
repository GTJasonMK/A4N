import React from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import type { ChapterOutline, PartOutline } from "../../../api/outline";
import {
  ChapterOutlineSection,
  type ChapterOutlineEditForm,
} from "./ChapterOutlineSection";
import {
  ChapterOutlineActionDialog,
  ChapterOutlineDetailDialog,
  ManualChapterOutlineDialog,
  type ManualChapterOutlineForm,
  ManualPartOutlineDialog,
  type ManualPartOutlineForm,
  PartDetailDialog,
} from "./OutlineDialogs";
import { OutlineAdvancedToolsDialog } from "./OutlineAdvancedToolsDialog";
import { PartOutlineSection } from "./PartOutlineSection";
import type { NumberInputValue } from "./outlineTabUtils";
import type { RunOutlineGeneration } from "./useOutlineGeneration";
import { useI18n } from "../../../i18n";

type OutlineView = "parts" | "chapters";

interface ManualPartEditor {
  part: PartOutline | null;
  initial: ManualPartOutlineForm;
}

export interface OutlinesTabViewProps {
  projectId: number;
  loading: boolean;
  partsLoaded: boolean;
  outlinesLoaded: boolean;
  activeOutlineView: OutlineView;
  parts: PartOutline[];
  outlines: ChapterOutline[];
  canEditOutlines: boolean;
  partCount: NumberInputValue;
  chaptersPerPart: NumberInputValue;
  defaultChaptersPerPart: NumberInputValue;
  generatingParts: boolean;
  chapterCount: NumberInputValue;
  generatingOutlines: boolean;
  editingId: number | null;
  editForm: ChapterOutlineEditForm;
  savingId: number | null;
  regenId: number | null;
  manualChapterInitial: ManualChapterOutlineForm | null;
  manualPartEditor: ManualPartEditor | null;
  manualSaving: "chapter" | "part" | null;
  partDetail: PartOutline | null;
  outlineDetail: ChapterOutline | null;
  outlineAction: ChapterOutline | null;
  advancedOpen: boolean;
  delPartCount: NumberInputValue;
  delChapterCount: NumberInputValue;
  delPartCountError: string | null;
  delChapterCountError: string | null;
  delActing: string | null;
  regenLastPart: boolean;
  interactionBusy: boolean;
  requestPreviewOpen: boolean;
  requestPreviewLoading: boolean;
  requestPreviewData: ModelRequestPreview | null;
  runOutlineGeneration: RunOutlineGeneration;
  onSetOutlineView: (view: OutlineView) => void;
  onPartCountChange: (value: NumberInputValue) => void;
  onChaptersPerPartChange: (value: NumberInputValue) => void;
  onGenerateParts: () => void;
  onPreviewGenerateParts: () => void;
  onCreatePart: () => void;
  onOpenAdvanced: () => void;
  onOpenPart: (part: PartOutline) => void;
  onChapterCountChange: (value: NumberInputValue) => void;
  onGenerateChapterOutlines: () => void;
  onPreviewGenerateChapterOutlines: () => void;
  onCreateChapter: () => void;
  onOpenOutlineDetail: (outline: ChapterOutline) => void;
  onCancelEdit: () => void;
  onEditFormChange: (form: ChapterOutlineEditForm) => void;
  onSaveOutline: (outline: ChapterOutline) => void | Promise<void>;
  onOpenOutlineAction: (outline: ChapterOutline) => void;
  onClosePartDetail: () => void;
  onEditPart: (part: PartOutline) => void;
  onRefinePart: (part: PartOutline) => void;
  onCloseManualChapter: () => void;
  onSaveManualChapter: (form: ManualChapterOutlineForm) => void | Promise<void>;
  onCloseManualPart: () => void;
  onSaveManualPart: (form: ManualPartOutlineForm) => void | Promise<void>;
  onCloseOutlineDetail: () => void;
  onCloseOutlineAction: () => void;
  onStartEdit: (outline: ChapterOutline) => void;
  onRegenerateOutline: (outline: ChapterOutline) => void | Promise<void>;
  onPreviewRegenerateOutline: (outline: ChapterOutline) => void | Promise<void>;
  onDeleteOutline: (outline: ChapterOutline) => void | Promise<void>;
  onCloseAdvanced: () => void;
  onDelPartCountChange: (value: NumberInputValue) => void;
  onDelChapterCountChange: (value: NumberInputValue) => void;
  onRegenLastPart: () => void;
  onPreviewRegenLastPart: () => void;
  onDeleteLatestParts: () => void;
  onDeleteLatestOutlines: () => void;
  onChanged: () => boolean | void | Promise<boolean | void>;
  onCloseRequestPreview: () => void;
}

export const OutlinesTabView: React.FC<OutlinesTabViewProps> = ({
  projectId,
  loading,
  partsLoaded,
  outlinesLoaded,
  activeOutlineView,
  parts,
  outlines,
  canEditOutlines,
  partCount,
  chaptersPerPart,
  defaultChaptersPerPart,
  generatingParts,
  chapterCount,
  generatingOutlines,
  editingId,
  editForm,
  savingId,
  regenId,
  manualChapterInitial,
  manualPartEditor,
  manualSaving,
  partDetail,
  outlineDetail,
  outlineAction,
  advancedOpen,
  delPartCount,
  delChapterCount,
  delPartCountError,
  delChapterCountError,
  delActing,
  regenLastPart,
  interactionBusy,
  requestPreviewOpen,
  requestPreviewLoading,
  requestPreviewData,
  runOutlineGeneration,
  onSetOutlineView,
  onPartCountChange,
  onChaptersPerPartChange,
  onGenerateParts,
  onPreviewGenerateParts,
  onCreatePart,
  onOpenAdvanced,
  onOpenPart,
  onChapterCountChange,
  onGenerateChapterOutlines,
  onPreviewGenerateChapterOutlines,
  onCreateChapter,
  onOpenOutlineDetail,
  onCancelEdit,
  onEditFormChange,
  onSaveOutline,
  onOpenOutlineAction,
  onClosePartDetail,
  onEditPart,
  onRefinePart,
  onCloseManualChapter,
  onSaveManualChapter,
  onCloseManualPart,
  onSaveManualPart,
  onCloseOutlineDetail,
  onCloseOutlineAction,
  onStartEdit,
  onRegenerateOutline,
  onPreviewRegenerateOutline,
  onDeleteOutline,
  onCloseAdvanced,
  onDelPartCountChange,
  onDelChapterCountChange,
  onRegenLastPart,
  onPreviewRegenLastPart,
  onDeleteLatestParts,
  onDeleteLatestOutlines,
  onChanged,
  onCloseRequestPreview,
}) => {
  const { t } = useI18n();

  if (loading && !partsLoaded && !outlinesLoaded) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> {t("outline.loading")}
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-5">
      <nav
        className="tab-bar min-w-0 shrink-0 border-b border-book-border/45"
        role="tablist"
        aria-label={t("outline.tabs.ariaLabel")}
      >
        <button
          id="outline-parts-tab"
          type="button"
          role="tab"
          aria-selected={activeOutlineView === "parts"}
          aria-controls="outline-parts-panel"
          onClick={() => onSetOutlineView("parts")}
          data-active={activeOutlineView === "parts" ? "1" : undefined}
          className="pyqt-tab"
        >
          {t("outline.tabs.parts")}
        </button>
        <button
          id="outline-chapters-tab"
          type="button"
          role="tab"
          aria-selected={activeOutlineView === "chapters"}
          aria-controls="outline-chapters-panel"
          onClick={() => onSetOutlineView("chapters")}
          data-active={activeOutlineView === "chapters" ? "1" : undefined}
          className="pyqt-tab"
        >
          {t("outline.tabs.chapters")}
        </button>
      </nav>

      {activeOutlineView === "parts" ? (
        <div id="outline-parts-panel" role="tabpanel" aria-labelledby="outline-parts-tab" className="min-w-0">
          <PartOutlineSection
            parts={parts}
            loaded={partsLoaded}
            canEdit={canEditOutlines}
            partCount={partCount}
            chaptersPerPart={chaptersPerPart}
            defaultChaptersPerPart={defaultChaptersPerPart}
            generating={generatingParts}
            disabled={interactionBusy || !partsLoaded}
            onPartCountChange={onPartCountChange}
            onChaptersPerPartChange={onChaptersPerPartChange}
            onGenerate={onGenerateParts}
            onPreviewGenerate={onPreviewGenerateParts}
            onCreatePart={onCreatePart}
            onOpenAdvanced={onOpenAdvanced}
            onOpenPart={onOpenPart}
          />
        </div>
      ) : (
        <div id="outline-chapters-panel" role="tabpanel" aria-labelledby="outline-chapters-tab" className="min-w-0">
          <ChapterOutlineSection
            outlines={outlines}
            loaded={outlinesLoaded}
            canEdit={canEditOutlines}
            chapterCount={chapterCount}
            generating={generatingOutlines}
            editingId={editingId}
            editForm={editForm}
            savingId={savingId}
            regenId={regenId}
            delActing={delActing}
            disabled={interactionBusy || !outlinesLoaded}
            onChapterCountChange={onChapterCountChange}
            onGenerate={onGenerateChapterOutlines}
            onPreviewGenerate={onPreviewGenerateChapterOutlines}
            onCreate={onCreateChapter}
            onOpenDetail={onOpenOutlineDetail}
            onCancelEdit={onCancelEdit}
            onEditFormChange={onEditFormChange}
            onSave={onSaveOutline}
            onOpenAction={onOpenOutlineAction}
          />
        </div>
      )}

      {partDetail ? (
        <PartDetailDialog
          part={partDetail}
          onClose={onClosePartDetail}
          onEdit={canEditOutlines ? onEditPart : undefined}
          onRefine={canEditOutlines ? onRefinePart : undefined}
          disabled={interactionBusy}
        />
      ) : null}
      {manualChapterInitial && canEditOutlines ? (
        <ManualChapterOutlineDialog
          initial={manualChapterInitial}
          saving={manualSaving === "chapter"}
          onClose={onCloseManualChapter}
          onSave={async (form) => {
            await onSaveManualChapter(form);
          }}
        />
      ) : null}
      {manualPartEditor && canEditOutlines ? (
        <ManualPartOutlineDialog
          part={manualPartEditor.part}
          initial={manualPartEditor.initial}
          saving={manualSaving === "part"}
          onClose={onCloseManualPart}
          onSave={async (form) => {
            await onSaveManualPart(form);
          }}
        />
      ) : null}
      {outlineDetail ? (
        <ChapterOutlineDetailDialog
          outline={outlineDetail}
          onClose={onCloseOutlineDetail}
        />
      ) : null}
      {canEditOutlines ? (
        <ChapterOutlineActionDialog
          outline={outlineAction}
          regenId={regenId}
          delActing={delActing}
          disabled={interactionBusy}
          onClose={onCloseOutlineAction}
          onEdit={onStartEdit}
          onRegenerate={async (outline) => {
            await onRegenerateOutline(outline);
          }}
          onPreviewRegenerate={async (outline) => {
            await onPreviewRegenerateOutline(outline);
          }}
          onDelete={async (outline) => {
            await onDeleteOutline(outline);
          }}
        />
      ) : null}
      {canEditOutlines ? (
        <OutlineAdvancedToolsDialog
          open={advancedOpen}
          projectId={projectId}
          parts={parts}
          outlines={outlines}
          chaptersPerPart={chaptersPerPart}
          delPartCount={delPartCount}
          delChapterCount={delChapterCount}
          delPartCountError={delPartCountError}
          delChapterCountError={delChapterCountError}
          delActing={delActing}
          regenLastPart={regenLastPart}
          runGeneration={runOutlineGeneration}
          disabled={interactionBusy}
          onClose={onCloseAdvanced}
          onDelPartCountChange={onDelPartCountChange}
          onDelChapterCountChange={onDelChapterCountChange}
          onRegenLastPart={onRegenLastPart}
          onPreviewRegenLastPart={onPreviewRegenLastPart}
          onDeleteLatestParts={onDeleteLatestParts}
          onDeleteLatestOutlines={onDeleteLatestOutlines}
          onChanged={onChanged}
        />
      ) : null}
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("outline.requestPreviewTitle")}
        onClose={onCloseRequestPreview}
      />
    </div>
  );
};
