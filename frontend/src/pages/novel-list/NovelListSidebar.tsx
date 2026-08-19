import React from "react";
import { Plus, Settings as SettingsIcon, Upload } from "lucide-react";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { ParticleLogoWithSubtitle } from "./ParticleLogoWithSubtitle";
import type { Quote } from "./novelListUtils";

interface NovelListSidebarProps {
  quote: Quote;
  onOpenSettings: () => void;
  onCreateProject: () => void;
  onImportProject: () => void;
}

export const NovelListSidebar: React.FC<NovelListSidebarProps> = ({
  quote,
  onOpenSettings,
  onCreateProject,
  onImportProject,
}) => (
  <aside className="flex shrink-0 flex-col overflow-hidden border-b border-book-border/40 bg-book-bg-paper/70 backdrop-blur-xl md:w-[min(31vw,360px)] md:min-w-[288px] md:max-w-[360px] md:border-b-0 md:border-r">
    <div className="flex flex-col px-4 py-3.5 sm:px-5 md:flex-1 md:overflow-y-auto md:px-6 md:py-6 lg:px-7 custom-scrollbar">
      <div className="mb-4 hidden justify-end md:flex">
        <button
          type="button"
          onClick={onOpenSettings}
          className="pyqt-btn-secondary !min-h-9 px-3.5 text-sm"
        >
          <SettingsIcon size={14} className="mr-1" />
          设置
        </button>
      </div>

      {/* Logo 粒子效果（现网 canvas，勿替换为纯文字） */}
      <div className="mb-3 flex justify-center md:mb-6">
        <div className="max-w-[230px] sm:max-w-[260px] md:max-w-none w-full">
          <ParticleLogoWithSubtitle />
        </div>
      </div>

      <figure className="hidden rounded-2xl bg-book-primary/[0.06] px-4 py-3.5 md:block">
        <p className="text-body-lg text-book-text-main font-serif italic leading-relaxed">
          {quote.zh}
        </p>
        <p className="mt-1.5 hidden text-xs text-book-text-muted leading-relaxed sm:block">{quote.en}</p>
      </figure>

      <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2 md:mt-8 md:flex md:flex-col">
        <BookButton
          variant="primary"
          size="lg"
          onClick={onCreateProject}
          className="w-full justify-center !min-h-10 !px-3 !text-sm md:!min-h-11 md:!px-4 md:!text-base"
        >
          <span className="inline-flex items-center gap-2">
            <Plus size={17} />
            创建项目
          </span>
        </BookButton>

        <BookButton
          variant="secondary"
          size="lg"
          onClick={onImportProject}
          aria-label="导入项目"
          title="导入项目"
          className="!min-h-10 !px-3 !text-sm max-md:!w-11 max-md:!min-w-11 md:w-full md:justify-center md:!min-h-11 md:!px-4 md:!text-body-lg"
        >
          <span className="inline-flex items-center gap-2">
            <Upload size={17} />
            <span className="hidden sm:inline">导入项目</span>
          </span>
        </BookButton>

        <BookIconButton
          label="打开设置"
          onClick={onOpenSettings}
          variant="secondary"
          className="md:hidden"
        >
          <SettingsIcon size={16} />
        </BookIconButton>
      </div>

      <div className="flex-1" />
    </div>
  </aside>
);
