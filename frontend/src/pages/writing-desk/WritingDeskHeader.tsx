import React from "react";
import {
  ChevronLeft,
  ChevronRight,
  FileText,
  Info,
  PanelLeftClose,
  PanelLeftOpen,
  Save,
  Search,
  Wand2,
} from "lucide-react";
import { type NovelProject } from "../../api/project";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { BackButton } from "../../components/ui/BackButton";

export type RightPanel = "none" | "rag" | "optimize";

interface WritingDeskHeaderProps {
  project: NovelProject;
  activeNumber: number | null;
  canGoPrev: boolean;
  canGoNext: boolean;
  leftSidebarVisible: boolean;
  rightPanel: RightPanel;
  dirty: boolean;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  rightPanelAvailable: boolean;
  rightPanelUnavailableTip: string;
  onSmartBack: () => void | Promise<void>;
  onOpenProjectDetail: () => void | Promise<void>;
  onPrev: () => void;
  onNext: () => void;
  onToggleLeftSidebar: () => void;
  onRightPanelChange: (panel: RightPanel) => void;
  onSaveContent: () => void | Promise<void>;
}

const PANEL_OPTIONS: ReadonlyArray<{
  id: Exclude<RightPanel, "none">;
  label: string;
  icon: React.ReactNode;
}> = [
  { id: "rag", label: "RAG 查询", icon: <Search size={11} /> },
  { id: "optimize", label: "正文优化", icon: <Wand2 size={11} /> },
];

export const WritingDeskHeader: React.FC<WritingDeskHeaderProps> = ({
  project,
  activeNumber,
  canGoPrev,
  canGoNext,
  leftSidebarVisible,
  rightPanel,
  dirty,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  rightPanelAvailable,
  rightPanelUnavailableTip,
  onSmartBack,
  onOpenProjectDetail,
  onPrev,
  onNext,
  onToggleLeftSidebar,
  onRightPanelChange,
  onSaveContent,
}) => (
  <header className="a4n-page-header h-auto min-h-12 min-w-0 shrink-0 flex-wrap overflow-visible py-2 sm:h-10 sm:min-h-10 sm:flex-nowrap sm:overflow-hidden sm:py-0">
    <div className="flex min-w-0 flex-1 basis-48 items-center gap-2">
      <BackButton iconOnly title="返回上一页" onClick={() => void onSmartBack()} />
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
      <div className="flex min-w-0 flex-1 items-center gap-2">
        <span className="hidden shrink-0 text-[10px] uppercase tracking-wide text-book-text-muted sm:inline">
          写作桌
        </span>
        <h2 className="text-xs font-serif font-semibold text-book-text-main truncate">
          {project.title}
          {activeNumber != null && (
            <span className="text-book-text-muted ml-1.5">· 第 {activeNumber} 章</span>
          )}
        </h2>
      </div>
    </div>
    <div className="flex min-w-0 flex-1 flex-wrap items-center justify-end gap-1 sm:flex-none sm:flex-nowrap">
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
      <div className="ml-0 inline-flex min-w-0 items-center gap-px overflow-hidden rounded-sm border border-book-border sm:ml-1">
        {PANEL_OPTIONS.map((panel) => {
          const active = rightPanel === panel.id;
          const disabled = !rightPanelAvailable && !active;
          return (
            <button
              key={panel.id}
              type="button"
              onClick={() => onRightPanelChange(active ? "none" : panel.id)}
              disabled={disabled}
              data-active={active ? "1" : undefined}
              className={`inline-flex h-9 min-w-0 items-center gap-1 px-2 text-[11px] font-semibold transition-colors sm:h-7 ${
                active
                  ? "bg-book-primary/10 text-book-primary"
                  : disabled
                    ? "cursor-not-allowed text-book-text-muted opacity-55"
                    : "text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main"
              }`}
              title={disabled ? rightPanelUnavailableTip : `${panel.label}（写作桌助手）`}
            >
              {panel.icon}
              <span className="truncate">{panel.label}</span>
            </button>
          );
        })}
      </div>
      {!rightPanelAvailable ? (
        <span
          className="hidden items-center gap-1 text-[10px] text-book-text-muted lg:inline-flex"
          title={rightPanelUnavailableTip}
        >
          <Info size={11} />
          仅正文页可用
        </span>
      ) : null}
      <BookButton
        variant="secondary"
        onClick={onSaveContent}
        disabled={!dirty || activeChapterBusy}
        className="!px-3"
        title={activeChapterBusy ? activeChapterGeneratingMessage : undefined}
      >
        <span className="inline-flex items-center gap-1">
          <Save size={12} />
          {dirty ? "保存" : "已保存"}
        </span>
      </BookButton>
    </div>
  </header>
);
