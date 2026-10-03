import React from "react";
import { ListPlus, Sparkles } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { useI18n } from "../../i18n";

interface CanvasEmptyStateProps {
  disabled: boolean;
  showTextToVideoTemplate?: boolean;
  templateDisabled?: boolean;
  mobile?: boolean;
  onOpenNodeLibrary?: () => void;
  onUseTextToVideoTemplate?: () => void;
}

export const CanvasEmptyState: React.FC<CanvasEmptyStateProps> = ({
  disabled,
  showTextToVideoTemplate = false,
  templateDisabled = false,
  mobile = false,
  onOpenNodeLibrary,
  onUseTextToVideoTemplate,
}) => {
  const { t } = useI18n();
  return (
  <div className={`pointer-events-none absolute inset-0 flex items-center justify-center p-6 ${mobile ? "pb-24" : ""}`}>
    <div className="a4n-empty-state a4n-empty-state--framed w-full max-w-sm">
      <div className="font-serif text-sm font-semibold text-book-text-main">
        {disabled ? t("video.noEditPermission") : t("video.emptyCanvas")}
      </div>
      <div className="mt-1 text-xs leading-relaxed text-book-text-muted">
        {disabled
          ? t("video.viewOnlyWorkflow")
          : mobile
            ? t("video.mobileCanvasHint")
            : t("video.canvasDragHint")}
      </div>
      {showTextToVideoTemplate && !disabled && onUseTextToVideoTemplate ? (
        <BookButton
          variant="primary"
          size="sm"
          onClick={onUseTextToVideoTemplate}
          disabled={templateDisabled}
          className="pointer-events-auto mt-3"
        >
          <Sparkles size={14} />
          {t("video.canvas.useTextToVideoTemplate")}
        </BookButton>
      ) : null}
      {mobile && !disabled && onOpenNodeLibrary ? (
        <BookButton
          variant="primary"
          size="sm"
          onClick={onOpenNodeLibrary}
          className="pointer-events-auto mt-2"
        >
          <ListPlus size={14} />
          {t("video.openNodeLibrary")}
        </BookButton>
      ) : null}
    </div>
  </div>
  );
};
