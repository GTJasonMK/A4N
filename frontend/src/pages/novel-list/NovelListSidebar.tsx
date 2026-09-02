import React, { useEffect, useState } from "react";
import { Plus, Settings as SettingsIcon, Upload } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { ParticleLogoWithSubtitle } from "./ParticleLogoWithSubtitle";
import type { Quote } from "./novelListUtils";
import { useI18n } from "../../i18n";

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
}) => {
  const { t } = useI18n();
  const [isDesktop, setIsDesktop] = useState(
    () => typeof window === "undefined" || window.matchMedia("(min-width: 768px)").matches,
  );

  useEffect(() => {
    const media = window.matchMedia("(min-width: 768px)");
    const sync = () => setIsDesktop(media.matches);
    sync();
    media.addEventListener("change", sync);
    return () => media.removeEventListener("change", sync);
  }, []);

  if (!isDesktop) return null;

  return (
  <aside className="a4n-home-sidebar flex shrink-0 flex-col overflow-hidden border-b border-book-border/40 bg-book-bg-paper/70 backdrop-blur-xl md:w-[min(31vw,360px)] md:min-w-[288px] md:max-w-[360px] md:border-b-0 md:border-r">
    <div className="flex flex-col px-4 py-2.5 sm:px-5 sm:py-3.5 md:flex-1 md:overflow-y-auto md:px-6 md:py-6 lg:px-7 custom-scrollbar">
      <div className="mb-4 flex justify-end">
        <button
          type="button"
          onClick={onOpenSettings}
          className="pyqt-btn-secondary !min-h-9 px-3.5 text-sm"
        >
          <SettingsIcon size={14} className="mr-1" />
          {t("novel.settings")}
        </button>
      </div>

      {/* Logo 粒子效果（现网 canvas，勿替换为纯文字） */}
      <div className="mb-2 flex justify-center sm:mb-3 md:mb-6">
        <div className="w-full max-w-[250px] sm:max-w-[280px] md:max-w-none">
          <ParticleLogoWithSubtitle />
        </div>
      </div>

      <figure className="a4n-home-quote hidden rounded-2xl bg-book-primary/[0.06] px-4 py-3.5 md:block">
        <p className="text-body-lg text-book-text-main font-serif italic leading-relaxed">
          {quote.zh}
        </p>
        <p className="mt-1.5 hidden text-xs text-book-text-muted leading-relaxed sm:block">{quote.en}</p>
      </figure>

      <div className="mt-8 flex flex-col items-center gap-2">
        <BookButton
          variant="primary"
          size="lg"
          onClick={onCreateProject}
          className="w-full justify-center !min-h-11 !px-4 !text-base"
        >
          <span className="inline-flex items-center gap-2">
            <Plus size={17} />
            {t("novel.createProject")}
          </span>
        </BookButton>

        <BookButton
          variant="secondary"
          size="lg"
          onClick={onImportProject}
          aria-label={t("novel.importProjectButton")}
          title={t("novel.importProjectButton")}
          className="w-full justify-center !min-h-11 !px-4 !text-body-lg"
        >
          <span className="inline-flex items-center gap-2">
            <Upload size={17} />
            <span className="hidden sm:inline">{t("novel.importProjectButton")}</span>
          </span>
        </BookButton>

      </div>

      <div className="flex-1" />
    </div>
  </aside>
  );
};
