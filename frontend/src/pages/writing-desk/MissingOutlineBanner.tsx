import React from "react";
import { BookButton } from "../../components/ui/BookButton";
import { useI18n } from "../../i18n";

interface MissingOutlineBannerProps {
  onOpenProjectDetail: () => void | Promise<void>;
}

/** 无大纲不阻断手工写作，只提示可选的资料补充入口。 */
export const MissingOutlineBanner: React.FC<MissingOutlineBannerProps> = ({
  onOpenProjectDetail,
}) => {
  const { t } = useI18n();
  return (
  <div
    className="flex shrink-0 flex-col gap-2 border-b border-warning/30 bg-warning-bg px-3 py-2.5 sm:flex-row sm:items-center"
    role="status"
  >
    <div className="min-w-0 flex-1">
      <div className="text-xs font-semibold text-warning">{t("writing.noChapterOutline")}</div>
      <div className="mt-0.5 text-small leading-relaxed text-warning/90">
        {t("writing.noChapterOutlineDescription")}
      </div>
    </div>
    <BookButton
      variant="primary"
      size="sm"
      onClick={() => void onOpenProjectDetail()}
      className="w-full sm:w-auto"
    >
      {t("writing.addProjectMaterials")}
    </BookButton>
  </div>
  );
};
