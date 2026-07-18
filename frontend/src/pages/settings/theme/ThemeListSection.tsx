import React from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { type Theme } from "../../../api/theme";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { ThemeCard } from "./ThemeCard";
import { type ThemeScope } from "./types";

interface ThemeListSectionProps {
  themes: Theme[];
  scope: ThemeScope;
  loading: boolean;
  hasUser: boolean;
  activatingId: number | null;
  deletingId: number | null;
  exportingId: number | null;
  renamingId: number | null;
  renameValue: string;
  renameSaving: boolean;
  mutationBusy: boolean;
  onScopeChange: (scope: ThemeScope) => void;
  onRefresh: () => void;
  onRenameInput: (value: string) => void;
  onRenameStart: (theme: Theme) => void;
  onRenameConfirm: () => void;
  onRenameCancel: () => void;
  onActivate: (themeId: number) => void;
  onDelete: (themeId: number, name: string) => void;
  onExport: (themeId: number) => void;
  onViewCss: (themeId: number) => void;
}

export const ThemeListSection: React.FC<ThemeListSectionProps> = ({
  themes,
  scope,
  loading,
  hasUser,
  activatingId,
  deletingId,
  exportingId,
  renamingId,
  renameValue,
  renameSaving,
  mutationBusy,
  onScopeChange,
  onRefresh,
  onRenameInput,
  onRenameStart,
  onRenameConfirm,
  onRenameCancel,
  onActivate,
  onDelete,
  onExport,
  onViewCss,
}) => (
  <div>
    <div className="mb-3 flex min-w-0 items-center gap-2">
      <SegmentedControl
        ariaLabel="主题范围"
        value={scope}
        onChange={onScopeChange}
        options={[
          { id: "all", label: "全部", disabled: mutationBusy },
          { id: "user", label: "我的", disabled: !hasUser || mutationBusy },
        ]}
      />
      <span className="min-w-0 flex-1 truncate text-[11px] text-book-text-muted">
        {loading ? "正在更新…" : `${themes.length} 个主题`}
      </span>
      <button
        type="button"
        onClick={onRefresh}
        disabled={loading || mutationBusy}
        className="pyqt-btn-icon !h-8 !w-8 shrink-0"
        title="刷新主题"
        aria-label="刷新主题"
      >
        <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
      </button>
    </div>

    {loading && themes.length === 0 ? (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <Loader2 size={12} className="animate-spin" /> 加载主题…
      </div>
    ) : themes.length === 0 ? (
      <div className="a4n-empty-state a4n-empty-state--framed text-xs">
        尚未配置任何主题。下方可创建第一个主题。
      </div>
    ) : (
      <ul className="grid grid-cols-1 items-start gap-2 xl:grid-cols-2">
        {themes.map((theme) => (
          <ThemeCard
            key={theme.id}
            theme={theme}
            activating={activatingId === theme.id}
            activateDisabled={mutationBusy}
            deleting={deletingId === theme.id}
            exporting={exportingId === theme.id}
            renaming={renamingId === theme.id}
            renameValue={renameValue}
            renameSaving={renameSaving && renamingId === theme.id}
            mutationBusy={mutationBusy}
            onRenameInput={onRenameInput}
            onRenameStart={() => onRenameStart(theme)}
            onRenameConfirm={onRenameConfirm}
            onRenameCancel={onRenameCancel}
            onActivate={() => onActivate(theme.id)}
            onDelete={() => onDelete(theme.id, theme.name)}
            onExport={() => onExport(theme.id)}
            onViewCss={() => onViewCss(theme.id)}
          />
        ))}
      </ul>
    )}
  </div>
);
