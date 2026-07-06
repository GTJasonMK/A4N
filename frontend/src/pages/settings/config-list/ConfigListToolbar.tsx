import React from "react";
import { Plus, RefreshCw } from "lucide-react";
import { BookIconButton } from "../../../components/ui/BookButton";

interface ConfigListToolbarProps {
  title: string;
  itemCount: number;
  loading: boolean;
  onCreate: () => void;
  onRefresh: () => void | Promise<void>;
}

export const ConfigListToolbar: React.FC<ConfigListToolbarProps> = ({
  title,
  itemCount,
  loading,
  onCreate,
  onRefresh,
}) => {
  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={onCreate}
        className="pyqt-btn-primary !min-h-[36px] !px-4 text-xs"
      >
        <span className="inline-flex items-center gap-1.5">
          <Plus size={14} />
          新增{title}
        </span>
      </button>
      <BookIconButton
        label="刷新"
        variant="secondary"
        size="sm"
        onClick={() => void onRefresh()}
        disabled={loading}
      >
        <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
      </BookIconButton>
      <div className="flex-1" />
      {itemCount > 0 && (
        <span className="text-xs text-book-text-muted">共 {itemCount} 项</span>
      )}
    </div>
  );
};
