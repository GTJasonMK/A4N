import React from "react";
import { Plus, RefreshCw } from "lucide-react";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useI18n } from "../../../i18n";

interface ConfigListToolbarProps {
  title: string;
  itemCount: number;
  loading: boolean;
  busy: boolean;
  onCreate: () => void;
  onRefresh: () => void | Promise<void>;
}

/** 配置列表工具条。 */
export const ConfigListToolbar: React.FC<ConfigListToolbarProps> = ({
  title,
  itemCount,
  loading,
  busy,
  onCreate,
  onRefresh,
}) => {
  const { t } = useI18n();
  const createLabel = t("settings.createConfig", { name: title });
  return (
    <header className="flex flex-wrap items-center gap-3 border-b border-book-border/70 pb-4 max-sm:items-start">
      <div className="min-w-0 flex-1 max-sm:basis-full">
        <h2 className="a4n-section-heading">{title}</h2>
        <p className="mt-0.5 text-small text-book-text-muted">
          {itemCount > 0 ? t("settings.configCount", { count: itemCount }) : t("settings.noConfigYet")}
        </p>
      </div>
      <button
        type="button"
        onClick={onCreate}
        disabled={loading || busy}
        className="a4n-touch-target pyqt-btn-primary px-3 text-sm max-sm:flex-1"
      >
        <Plus size={16} />
        {createLabel}
      </button>
      <BookIconButton
        label={t("common.refresh")}
        variant="secondary"
        size="md"
        onClick={() => void onRefresh()}
        disabled={loading || busy}
      >
        <RefreshCw size={16} className={loading ? "animate-spin" : undefined} />
      </BookIconButton>
    </header>
  );
};
