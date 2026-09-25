import React, { useEffect, useState } from "react";
import { Plus, Settings as SettingsIcon, Upload } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { ParticleLogoWithSubtitle } from "./ParticleLogoWithSubtitle";
import type { Quote } from "./novelListUtils";
import { useI18n } from "../../i18n";
import { usePackagePreferences } from "../../store/packagePreferences";

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
  const { isVisible } = usePackagePreferences();
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
  <aside className="a4n-home-sidebar">
    <div className="a4n-home-sidebar__body custom-scrollbar">
      <div className="flex items-center justify-between gap-2">
        <span className="a4n-home-wordmark">A4N</span>
        <button
          type="button"
          onClick={onOpenSettings}
          className="a4n-btn-ghost min-h-9 px-2 text-sm"
        >
          <SettingsIcon size={14} className="mr-1" />
          {t("novel.settings")}
        </button>
      </div>

      {/* Logo 粒子效果（现网 canvas，勿替换为纯文字） */}
      <div className="a4n-home-logo">
        <ParticleLogoWithSubtitle />
      </div>

      <div className="flex flex-col items-center gap-3">
        <BookButton
          variant="primary"
          size="lg"
          onClick={onCreateProject}
          className="w-full justify-center"
        >
          <span className="inline-flex items-center gap-2">
            <Plus size={17} />
            {t("novel.createProject")}
          </span>
        </BookButton>

        {isVisible("external-import") ? <BookButton
          variant="secondary"
          size="lg"
          onClick={onImportProject}
          aria-label={t("novel.importProjectButton")}
          title={t("novel.importProjectButton")}
          className="w-full justify-center"
        >
          <span className="inline-flex items-center gap-2">
            <Upload size={17} />
            {t("novel.importProjectButton")}
          </span>
        </BookButton> : null}
      </div>

      <figure className="a4n-home-quote">
        <blockquote className="font-serif text-lg leading-relaxed">{quote.zh}</blockquote>
        <figcaption className="mt-3 text-small leading-relaxed text-book-text-muted">{quote.en}</figcaption>
      </figure>
    </div>
  </aside>
  );
};
