import React from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { type Theme } from "../../../api/theme";
import { ThemeCard } from "./ThemeCard";
import { type ThemeScope } from "./types";

interface ThemeListSectionProps {
  themes: Theme[];
  defaultTheme: Theme | null;
  scope: ThemeScope;
  loading: boolean;
  hasUser: boolean;
  activatingId: number | null;
  deletingId: number | null;
  exportingId: number | null;
  renamingId: number | null;
  renameValue: string;
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
  defaultTheme,
  scope,
  loading,
  hasUser,
  activatingId,
  deletingId,
  exportingId,
  renamingId,
  renameValue,
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
    <div className="flex items-center justify-between mb-3">
      <h3 className="text-sm font-serif text-book-text-main">主题列表</h3>
      <div className="inline-flex items-center gap-2">
        <div className="inline-flex pyqt-tab text-xs">
          <button
            type="button"
            data-active={scope === "all" ? "1" : undefined}
            onClick={() => onScopeChange("all")}
          >
            全部
          </button>
          <button
            type="button"
            data-active={scope === "user" ? "1" : undefined}
            onClick={() => onScopeChange("user")}
            disabled={!hasUser}
          >
            我的
          </button>
        </div>
        <span className="pyqt-pill">{themes.length} 个</span>
        <button
          type="button"
          onClick={onRefresh}
          disabled={loading}
          className="pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
          title="刷新"
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
        </button>
      </div>
    </div>

    {defaultTheme && (
      <div className="text-[11px] text-book-text-muted mb-2">
        系统默认主题：<span className="font-mono">{defaultTheme.name}</span>（{defaultTheme.mode}）
      </div>
    )}

    {loading && themes.length === 0 ? (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <Loader2 size={12} className="animate-spin" /> 加载主题…
      </div>
    ) : themes.length === 0 ? (
      <div className="pyqt-card p-6 text-center text-xs text-book-text-muted">
        尚未配置任何主题。下方可创建第一个主题。
      </div>
    ) : (
      <ul className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {themes.map((theme) => (
          <ThemeCard
            key={theme.id}
            theme={theme}
            activating={activatingId === theme.id}
            activateDisabled={activatingId != null}
            deleting={deletingId === theme.id}
            exporting={exportingId === theme.id}
            renaming={renamingId === theme.id}
            renameValue={renameValue}
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
