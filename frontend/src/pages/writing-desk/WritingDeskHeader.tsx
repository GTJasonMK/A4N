import React from "react";
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
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { BackButton } from "../../components/ui/BackButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { PageHeader } from "../../components/shell/PageHeader";

export type RightPanel = "none" | "rag" | "optimize";

interface WritingDeskHeaderProps {
  project: NovelProject;
  activeNumber: number | null;
  isMobileLayout: boolean;
  canGoPrev: boolean;
  canGoNext: boolean;
  leftSidebarVisible: boolean;
  rightPanel: RightPanel;
  dirty: boolean;
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
  onToggleLeftSidebar: () => void;
  onOpenProfiles: () => void;
  onRightPanelChange: (panel: RightPanel) => void;
  onSaveContent: () => void | Promise<void>;
}

const PANEL_OPTIONS: ReadonlyArray<{
  id: Exclude<RightPanel, "none">;
  label: string;
  icon: React.ReactNode;
}> = [
  { id: "rag", label: "资料检索", icon: <Search size={11} /> },
  { id: "optimize", label: "写作助手", icon: <Wand2 size={11} /> },
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
  leftSidebarVisible,
  rightPanel,
  dirty,
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
  onToggleLeftSidebar,
  onOpenProfiles,
  onRightPanelChange,
  onSaveContent,
}) => (
  <PageHeader
    className="a4n-app-page-header--desk a4n-app-page-header--detail h-auto min-w-0 shrink-0 py-2 sm:flex-nowrap sm:overflow-hidden sm:py-0"
    leading={
      <>
        <BackButton iconOnly title="返回上一页" onClick={() => void onSmartBack()} />
        {!isMobileLayout ? (
          <>
            <BookIconButton
              label="项目详情"
              onClick={() => void onOpenProjectDetail()}
              variant="secondary"
              size="sm"
            >
              <FileText size={13} />
            </BookIconButton>
            <BookIconButton
              label={leftSidebarVisible ? "隐藏左侧栏" : "显示左侧栏"}
              onClick={onToggleLeftSidebar}
              variant="secondary"
              size="sm"
            >
              {leftSidebarVisible ? <PanelLeftClose size={13} /> : <PanelLeftOpen size={13} />}
            </BookIconButton>
          </>
        ) : null}
      </>
    }
    title={
      <>
        {project.title}
        {activeNumber != null ? (
          <span className="ml-1.5 font-sans text-body font-medium text-book-text-muted">
            · 第 {activeNumber} 章
          </span>
        ) : null}
      </>
    }
    actions={
      <>
        <BookIconButton
          label="上一章"
          onClick={onPrev}
          disabled={!canGoPrev}
          title={canGoPrev ? "上一章" : "已经是第一章"}
          variant="secondary"
          size="sm"
        >
          <ChevronLeft size={13} />
        </BookIconButton>
        <BookIconButton
          label="下一章"
          onClick={onNext}
          disabled={!canGoNext}
          title={canGoNext ? "下一章" : "已经是最后一章"}
          variant="secondary"
          size="sm"
        >
          <ChevronRight size={13} />
        </BookIconButton>

        <SegmentedControl
          ariaLabel="写作辅助面板"
          variant="panel"
          value={rightPanel === "none" ? null : rightPanel}
          allowDeselect
          onDeselect={() => onRightPanelChange("none")}
          onChange={(id) => onRightPanelChange(id)}
          options={PANEL_OPTIONS.map((panel) => ({
            id: panel.id,
            label: <span className="hidden sm:inline">{panel.label}</span>,
            icon: panel.icon,
            disabled: !rightPanelAvailable && rightPanel !== panel.id,
            title:
              !rightPanelAvailable && rightPanel !== panel.id
                ? rightPanelUnavailableTip
                : `${panel.label}（写作桌助手）`,
          }))}
          className="max-w-full max-md:!w-[76px] max-md:!min-w-[76px] max-md:!max-w-[76px] max-md:!flex-[0_0_76px]"
        />

        {isMobileLayout ? (
          <Dropdown
            items={[
              {
                label: "项目详情",
                icon: <FileText size={12} />,
                onClick: () => void onOpenProjectDetail(),
              },
              {
                label: "新建章节",
                icon: <Plus size={12} />,
                onClick: () => void onCreateChapter(),
                disabled: Boolean(editBlockedReason),
              },
              {
                label: "角色档案",
                icon: <Users size={12} />,
                onClick: onOpenProfiles,
              },
            ]}
          />
        ) : null}

        {dirty ? (
          <BookButton
            variant="primary"
            size="sm"
            onClick={onSaveContent}
            disabled={activeChapterBusy || Boolean(editBlockedReason)}
            className="!min-h-8 !w-8 !px-0 !text-xs sm:!w-auto sm:!px-3"
            title={
              editBlockedReason ??
              (activeChapterBusy ? activeChapterGeneratingMessage : "保存修改")
            }
            aria-label="保存修改"
          >
            <Save size={12} />
            <span className="hidden sm:inline">保存修改</span>
          </BookButton>
        ) : null}
      </>
    }
  />
);
