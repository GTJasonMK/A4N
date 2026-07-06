import React from "react";
import { Plus, Settings as SettingsIcon, Upload } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { ParticleLogoWithSubtitle } from "./ParticleLogoWithSubtitle";
import type { Quote } from "./novelListUtils";

interface NovelListSidebarProps {
  quote: Quote;
  onOpenSettings: () => void;
  onCreateProject: () => void;
  onShowAllProjects: () => void;
  onImportProject: () => void;
}

export const NovelListSidebar: React.FC<NovelListSidebarProps> = ({
  quote,
  onOpenSettings,
  onCreateProject,
  onShowAllProjects,
  onImportProject,
}) => (
  <aside className="flex shrink-0 flex-col overflow-hidden border-b border-book-border bg-book-bg-paper md:w-[min(34vw,384px)] md:min-w-[288px] md:max-w-[384px] md:border-b-0 md:border-r">
    <div className="flex flex-col px-4 py-4 sm:px-5 md:flex-1 md:overflow-y-auto md:px-5 md:py-6 lg:px-7 lg:py-8 custom-scrollbar">
      <div className="mb-3 flex justify-end md:mb-4">
        <button
          type="button"
          onClick={onOpenSettings}
          className="pyqt-btn-secondary h-8 px-4 text-xs"
        >
          <SettingsIcon size={12} className="mr-1" />
          设置
        </button>
      </div>

      {/* Logo 粒子效果 */}
      <div className="-mx-1 mb-3 max-w-[260px] md:mb-4 md:max-w-none">
        <ParticleLogoWithSubtitle />
      </div>

      <div className="border-l-2 border-book-primary/35 pl-3">
        <p className="text-sm text-book-text-main font-serif italic leading-relaxed">
          {quote.zh}
        </p>
        <p className="hidden text-xs text-book-text-muted leading-relaxed sm:block">{quote.en}</p>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-2.5 sm:grid-cols-3 md:mt-8 md:flex md:flex-col">
        <BookButton
          variant="primary"
          size="lg"
          onClick={onCreateProject}
          className="!min-h-[42px] w-full justify-center !px-3"
        >
          <span className="inline-flex items-center gap-2">
            <Plus size={18} />
            创建小说
          </span>
        </BookButton>

        <BookButton
          variant="secondary"
          size="lg"
          onClick={onShowAllProjects}
          className="!min-h-[42px] w-full justify-center !px-3"
        >
          查看全部项目
        </BookButton>

        <BookButton
          variant="secondary"
          size="lg"
          onClick={onImportProject}
          className="!min-h-[42px] w-full justify-center !px-3"
        >
          <span className="inline-flex items-center gap-2">
            <Upload size={18} />
            导入项目
          </span>
        </BookButton>
      </div>

      <div className="flex-1" />
    </div>
  </aside>
);
