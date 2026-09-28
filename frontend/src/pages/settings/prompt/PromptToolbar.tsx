import React from "react";
import { DownloadCloud, Loader2, RotateCcw, Search } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookIconButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Dropdown } from "../../../components/ui/Dropdown";
import { useI18n } from "../../../i18n";

interface PromptToolbarProps {
  keyword: string;
  loading: boolean;
  mutationBusy: boolean;
  reloadingDefaults: boolean;
  onKeywordChange: (keyword: string) => void;
  onLoadDefaults: () => void;
  onRefresh: () => void;
}

export const PromptToolbar: React.FC<PromptToolbarProps> = ({
  keyword,
  loading,
  mutationBusy,
  reloadingDefaults,
  onKeywordChange,
  onLoadDefaults,
  onRefresh,
}) => {
  const { t } = useI18n();
  return (
  <div className="flex items-center gap-2 shrink-0">
    <div className="flex-1 relative">
      <Search
        size={14}
        className="absolute left-2.5 top-1/2 -translate-y-1/2 text-book-text-muted pointer-events-none"
      />
      <BookInput
        aria-label={t("settingsPrompt.searchPlaceholder")}
        value={keyword}
        onChange={(event) => onKeywordChange(event.target.value)}
        placeholder={t("settingsPrompt.searchPlaceholder")}
        className="!pl-8"
      />
    </div>
    <BookIconButton
      label={t("settingsPrompt.refresh")}
      onClick={onRefresh}
      size="sm"
      disabled={mutationBusy}
    >
      <Loader2 size={14} className={loading ? "animate-spin" : "hidden"} />
      {!loading && <RotateCcw size={14} />}
    </BookIconButton>
    {reloadingDefaults ? (
      <span
        className="inline-flex h-10 w-10 items-center justify-center text-book-text-muted sm:h-8 sm:w-8"
        title={t("settingsPrompt.loadingDefaults")}
      >
        <LoadingSpinner size="sm" />
      </span>
    ) : (
      <Dropdown
        items={[
          {
            label: t("settingsPrompt.loadDefaults"),
            icon: <DownloadCloud size={12} />,
            disabled: mutationBusy,
            onClick: onLoadDefaults,
          },
        ]}
      />
    )}
  </div>
  );
};
