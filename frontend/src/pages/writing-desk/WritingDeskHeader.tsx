import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useMemo } from "react";
import {
  ChevronLeft,
  ChevronRight,
  FileText,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  Save,
  Search,
  Users,
  Wand2,
} from "lucide-react";
import { type NovelProject } from "../../api/project";
import { type ChapterMetadata } from "../../api/chapter";
import { type ChapterOutline } from "../../api/outline";
import { isPositiveSafeInteger } from "../../utils/numberRange";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { BackButton } from "../../components/ui/BackButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { PageHeader } from "../../components/shell/PageHeader";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useI18n } from "../../i18n";
import { usePackagePreferences } from "../../store/packagePreferences";
import type { PackageId } from "../../api/generated/page";

export type RightPanel = "none" | "rag" | "optimize";

interface WritingDeskHeaderProps {
  project: NovelProject;
  activeNumber: number | null;
  isMobileLayout: boolean;
  canGoPrev: boolean;
  canGoNext: boolean;
  chapterNumbers: number[];
  chapters: ChapterMetadata[];
  outlines: ChapterOutline[];
  leftSidebarVisible: boolean;
  rightPanel: RightPanel;
  dirty: boolean;
  savingContent: boolean;
  editBlockedReason: string | null;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  rightPanelAvailable: boolean;
  rightPanelUnavailableTip: string;
  onSmartBack: () => void | Promise<void>;
  onOpenProjectDetail: () => void | Promise<void>;
  onCreateChapter: () => void | Promise<void>;
  onPrev: () => void;
  onNext: () => void;
  onSelectChapter: (chapterNumber: number) => void | Promise<unknown>;
  onToggleLeftSidebar: () => void;
  onOpenProfiles: () => void;
  onRightPanelChange: (panel: RightPanel) => void;
  onSaveContent: () => void | Promise<void>;
}

const PANEL_OPTIONS: ReadonlyArray<{
  id: Exclude<RightPanel, "none">;
  labelKey: "writing.rag" | "writing.writingAssistant";
  icon: React.ReactNode;
  packageId: PackageId;
}> = [
  { id: "rag", labelKey: "writing.rag", icon: <Search size={16} />, packageId: "vector-rag" },
  { id: "optimize", labelKey: "writing.writingAssistant", icon: <Wand2 size={16} />, packageId: "agent" },
];

/**
 * DeskShell 头栏：保留导航、章节切换、辅助面板与必要的保存操作。
 */
export const WritingDeskHeader: React.FC<WritingDeskHeaderProps> = ({
  project,
  activeNumber,
  isMobileLayout,
  canGoPrev,
  canGoNext,
  chapterNumbers,
  chapters,
  outlines,
  leftSidebarVisible,
  rightPanel,
  dirty,
  savingContent,
  editBlockedReason,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  rightPanelAvailable,
  rightPanelUnavailableTip,
  onSmartBack,
  onOpenProjectDetail,
  onCreateChapter,
  onPrev,
  onNext,
  onSelectChapter,
  onToggleLeftSidebar,
  onOpenProfiles,
  onRightPanelChange,
  onSaveContent,
}) => {
  const { t } = useI18n();
  const { isVisible } = usePackagePreferences();
  const visiblePanels = PANEL_OPTIONS.filter(panel => isVisible(panel.packageId));
  const panelLabels = new Map(PANEL_OPTIONS.map((panel) => [panel.id, t(panel.labelKey)]));
  const chapterLabels = useMemo(() => {
    const labels = new Map<number, string>();
    chapters.forEach((chapter) => {
      const title = chapter.title?.trim();
      if (title) labels.set(chapter.chapter_number, title);
    });
    outlines.forEach((outline) => {
      if (labels.has(outline.chapter_number)) return;
      const title = outline.title?.trim();
      if (title) labels.set(outline.chapter_number, title);
    });
    return labels;
  }, [chapters, outlines]);

  const chapterSelect = chapterNumbers.length > 0 ? (
    <ThemedSelect
      aria-label={t("writing.chooseChapterTitle")}
      title={t("writing.chooseChapterTitle")}
      value={activeNumber == null ? "" : String(activeNumber)}
      onChange={(selectedValue) => {
        const nextNumber = Number(selectedValue);
        if (isPositiveSafeInteger(nextNumber)) {
          void onSelectChapter(nextNumber);
        }
      }}
      className="a4n-mobile-chapter-select pyqt-input min-w-0 flex-1 !px-2 text-xs"
    >
      {activeNumber == null ? <option value="">{t("writing.chooseChapterTitle")}</option> : null}
      {chapterNumbers.map((chapterNumber) => {
        const label = chapterLabels.get(chapterNumber);
        return (
          <option key={chapterNumber} value={chapterNumber}>
            {label ? `${t("writing.chapterLabel", { number: chapterNumber })} · ${label}` : t("writing.chapterLabel", { number: chapterNumber })}
          </option>
        );
      })}
    </ThemedSelect>
  ) : null;

  const saveButton = dirty ? (
    <BookButton
      variant="primary"
      size="sm"
      onClick={onSaveContent}
      disabled={savingContent || activeChapterBusy || Boolean(editBlockedReason)}
      className="!w-11 !px-0 !text-xs sm:!w-auto sm:!px-3"
      title={
        editBlockedReason ??
        (savingContent
          ? t("writing.savingChapterBody")
          : activeChapterBusy
            ? activeChapterGeneratingMessage
            : t("writing.saveChanges"))
      }
      aria-label={t("writing.saveChanges")}
    >
      {savingContent ? <LoadingSpinner size="sm" /> : <Save size={12} />}
      <span className="hidden sm:inline">{t("writing.saveChanges")}</span>
    </BookButton>
  ) : null;

  if (isMobileLayout) {
    return (
      <div className="min-w-0 shrink-0">
        <PageHeader
          className="a4n-app-page-header--detail min-w-0 flex-nowrap py-1.5"
          leading={<BackButton iconOnly title={t("nav.backToPrevious")} onClick={() => void onSmartBack()} />}
          title={
            <>
              {project.title}
              {activeNumber != null ? (
                <span className="ml-1.5 font-sans text-body font-medium text-book-text-muted">
                  · {t("writing.chapterLabel", { number: activeNumber })}
                </span>
              ) : null}
            </>
          }
          actions={
            <>
              {saveButton}
              <Dropdown
                items={[
                  {
                    label: t("writing.projectDetails"),
                    icon: <FileText size={12} />,
                    onClick: () => void onOpenProjectDetail(),
                  },
                  {
                    label: t("writing.createChapter"),
                    icon: <Plus size={12} />,
                    onClick: () => void onCreateChapter(),
                    disabled: Boolean(editBlockedReason),
                  },
                  {
                    label: t("writing.characterProfiles"),
                    icon: <Users size={12} />,
                    onClick: onOpenProfiles,
                  },
                ]}
              />
            </>
          }
        />
        <div className="flex min-w-0 items-center gap-1.5 border-b border-book-border/50 bg-book-bg-paper/70 px-2 py-1.5">
          {chapterSelect}
          <BookIconButton
            label={t("writing.previousChapter")}
            onClick={onPrev}
            disabled={!canGoPrev}
            title={canGoPrev ? t("writing.previousChapter") : t("writing.firstChapter")}
            variant="secondary"
            size="sm"
          >
            <ChevronLeft size={16} />
          </BookIconButton>
          <BookIconButton
            label={t("writing.nextChapter")}
            onClick={onNext}
            disabled={!canGoNext}
            title={canGoNext ? t("writing.nextChapter") : t("writing.lastChapter")}
            variant="secondary"
            size="sm"
          >
            <ChevronRight size={16} />
          </BookIconButton>
          <SegmentedControl
            ariaLabel={t("writing.writingPanels")}
            variant="panel"
            value={rightPanel === "none" ? null : rightPanel}
            allowDeselect
            onDeselect={() => onRightPanelChange("none")}
            onChange={(id) => onRightPanelChange(id)}
            options={visiblePanels.map((panel) => ({
              id: panel.id,
              label: <span className="sr-only">{panelLabels.get(panel.id)}</span>,
              icon: panel.icon,
              disabled: !rightPanelAvailable && rightPanel !== panel.id,
              title:
                !rightPanelAvailable && rightPanel !== panel.id
                  ? rightPanelUnavailableTip
                  : `${panelLabels.get(panel.id)} (${t("writing.deskAssistant")})`,
            }))}
            className="a4n-writing-panel-toggle"
          />
        </div>
      </div>
    );
  }

  return <PageHeader
    className="a4n-app-page-header--desk a4n-app-page-header--detail h-auto min-w-0 shrink-0 py-2 sm:flex-nowrap sm:overflow-hidden sm:py-0"
    leading={
      <>
        <BackButton iconOnly title={t("nav.backToPrevious")} onClick={() => void onSmartBack()} />
        <BookIconButton
          label={t("writing.projectDetails")}
          onClick={() => void onOpenProjectDetail()}
          variant="secondary"
          size="sm"
        >
          <FileText size={16} />
        </BookIconButton>
        <BookIconButton
          label={leftSidebarVisible ? t("writing.hideLeftSidebar") : t("writing.showLeftSidebar")}
          onClick={onToggleLeftSidebar}
          variant="secondary"
          size="sm"
        >
          {leftSidebarVisible ? <PanelLeftClose size={16} /> : <PanelLeftOpen size={16} />}
        </BookIconButton>
      </>
    }
    title={
      <>
        {project.title}
        {activeNumber != null ? (
          <span className="ml-1.5 font-sans text-body font-medium text-book-text-muted">
            · {t("writing.chapterLabel", { number: activeNumber })}
          </span>
        ) : null}
      </>
    }
    actions={
      <>
        <BookIconButton
          label={t("writing.previousChapter")}
          onClick={onPrev}
          disabled={!canGoPrev}
          title={canGoPrev ? t("writing.previousChapter") : t("writing.firstChapter")}
          variant="secondary"
          size="sm"
        >
          <ChevronLeft size={16} />
        </BookIconButton>
        <BookIconButton
          label={t("writing.nextChapter")}
          onClick={onNext}
          disabled={!canGoNext}
          title={canGoNext ? t("writing.nextChapter") : t("writing.lastChapter")}
          variant="secondary"
          size="sm"
        >
          <ChevronRight size={16} />
        </BookIconButton>

        <SegmentedControl
          ariaLabel={t("writing.writingPanels")}
          variant="panel"
          value={rightPanel === "none" ? null : rightPanel}
          allowDeselect
          onDeselect={() => onRightPanelChange("none")}
          onChange={(id) => onRightPanelChange(id)}
          options={visiblePanels.map((panel) => ({
            id: panel.id,
            label: <span className="hidden sm:inline">{panelLabels.get(panel.id)}</span>,
            icon: panel.icon,
            disabled: !rightPanelAvailable && rightPanel !== panel.id,
            title:
              !rightPanelAvailable && rightPanel !== panel.id
                ? rightPanelUnavailableTip
              : `${panelLabels.get(panel.id)} (${t("writing.deskAssistant")})`,
          }))}
          className="a4n-writing-panel-toggle max-w-full"
        />

        {saveButton}
      </>
    }
  />
};
