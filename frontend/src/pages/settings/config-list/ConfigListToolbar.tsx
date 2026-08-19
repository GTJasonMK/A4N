import React from "react";
import { Plus, RefreshCw } from "lucide-react";
import { BookIconButton } from "../../../components/ui/BookButton";
import { configCreateLabel } from "./configText";

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
  const createLabel = configCreateLabel(title);
  return (
    <div className="a4n-flat-surface flex flex-wrap items-center gap-2 px-3 py-2.5">
      <div className="min-w-0 flex-1">
        <h3 className="text-xs font-semibold text-book-text-main">{title}</h3>
        <p className="mt-0.5 text-small text-book-text-muted">
          {itemCount > 0 ? `${itemCount} 项配置` : "尚未添加配置"}
        </p>
      </div>
      <button
        type="button"
        onClick={onCreate}
        disabled={loading || busy}
        className="a4n-touch-target pyqt-btn-primary !px-3 text-xs"
      >
        <Plus size={14} />
        {createLabel}
      </button>
      <BookIconButton
        label="刷新"
        variant="secondary"
        size="sm"
        onClick={() => void onRefresh()}
        disabled={loading || busy}
      >
        <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
      </BookIconButton>
    </div>
  );
};
