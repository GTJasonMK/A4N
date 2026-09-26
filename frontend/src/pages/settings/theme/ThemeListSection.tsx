import React, { useEffect, useState } from "react";
import { Archive, CheckCheck, Plus, RefreshCw, Square, Trash2, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { type Theme } from "../../../api/theme";
import { BookButton } from "../../../components/ui/BookButton";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { ThemeCard } from "./ThemeCard";
import { isDeletableTheme, type ThemeScope } from "./types";
import { useI18n } from "../../../i18n";

type ThemeDeleteTarget = Pick<Theme, "id" | "name">;

interface ThemeListSectionProps {
  themes: Theme[];
  scope: ThemeScope;
  loading: boolean;
  loadError: string | null;
  hasUser: boolean;
  activatingId: number | null;
  deletingId: number | null;
  exportingId: number | null;
  editingId: number | null;
  renamingId: number | null;
  renameValue: string;
  renameSaving: boolean;
  mutationBusy: boolean;
  bulkDeleting: boolean;
  onScopeChange: (scope: ThemeScope) => void;
  onRefresh: () => void;
  onRenameInput: (value: string) => void;
  onRenameStart: (theme: Theme) => void;
  onRenameConfirm: () => void;
  onRenameCancel: () => void;
  onActivate: (themeId: number) => void;
  onEdit: (theme: Theme) => void;
  onDelete: (themeId: number, name: string) => void;
  onDeleteMany: (themes: ThemeDeleteTarget[]) => void;
  onExport: (themeId: number) => void;
  onCreate: () => void;
  onImport: () => void;
  onPackage: () => void;
}

export const ThemeListSection: React.FC<ThemeListSectionProps> = ({
  themes,
  scope,
  loading,
  loadError,
  hasUser,
  activatingId,
  deletingId,
  exportingId,
  editingId,
  renamingId,
  renameValue,
  renameSaving,
  mutationBusy,
  bulkDeleting,
  onScopeChange,
  onRefresh,
  onRenameInput,
  onRenameStart,
  onRenameConfirm,
  onRenameCancel,
  onActivate,
  onEdit,
  onDelete,
  onDeleteMany,
  onExport,
  onCreate,
  onImport,
  onPackage,
}) => {
  const { t } = useI18n();
  const [selectedIds, setSelectedIds] = useState<Set<number>>(() => new Set());
  const deletableThemes = themes.filter(isDeletableTheme);
  const selectedThemes = deletableThemes.filter((theme) => selectedIds.has(theme.id));
  const allSelected = deletableThemes.length > 0 && selectedThemes.length === deletableThemes.length;

  useEffect(() => {
    const validIds = new Set(deletableThemes.map((theme) => theme.id));
    setSelectedIds((current) => {
      const next = new Set([...current].filter((id) => validIds.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [themes]);

  const toggleSelected = (themeId: number) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(themeId)) next.delete(themeId);
      else next.add(themeId);
      return next;
    });
  };

  const toggleSelectAll = () => {
    setSelectedIds(allSelected ? new Set() : new Set(deletableThemes.map((theme) => theme.id)));
  };

  return <div>
    <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2">
      <SegmentedControl
        ariaLabel={t("settingsTheme.scope")}
        value={scope}
        onChange={onScopeChange}
        options={[
          { id: "all", label: t("settingsTheme.all"), disabled: mutationBusy },
          { id: "user", label: t("settingsTheme.mine"), disabled: !hasUser || mutationBusy },
        ]}
      />
      <span className="min-w-0 flex-1 truncate text-small text-book-text-muted max-sm:flex-none max-sm:whitespace-nowrap">
        {loading
          ? t("settingsTheme.updating")
          : loadError && themes.length === 0 ? null : selectedThemes.length > 0
            ? `${t("settingsTheme.count", { count: themes.length })} · ${t("settingsTheme.selectedCount", { count: selectedThemes.length })}`
            : t("settingsTheme.count", { count: themes.length })}
      </span>
      {deletableThemes.length > 0 ? (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <BookButton
            variant="ghost"
            size="sm"
            onClick={toggleSelectAll}
            disabled={mutationBusy}
          >
            {allSelected ? <Square size={13} /> : <CheckCheck size={13} />}
            {allSelected ? t("settingsTheme.deselectAll") : t("settingsTheme.selectAll")}
          </BookButton>
          {selectedThemes.length > 0 ? (
            <BookButton
              variant="danger"
              size="sm"
              onClick={() => onDeleteMany(selectedThemes.map(({ id, name }) => ({ id, name })))}
              disabled={mutationBusy}
            >
              {bulkDeleting ? <LoadingSpinner size="sm" /> : <Trash2 size={13} />}
              {t("settingsTheme.deleteSelected", { count: selectedThemes.length })}
            </BookButton>
          ) : null}
        </div>
      ) : null}
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <BookButton variant="ghost" size="sm" onClick={onCreate} disabled={mutationBusy}>
          <Plus size={13} /> {t("settingsTheme.create")}
        </BookButton>
        <BookButton variant="ghost" size="sm" onClick={onImport} disabled={mutationBusy}>
          <Upload size={13} /> {t("settingsTheme.import")}
        </BookButton>
        <BookButton variant="ghost" size="sm" onClick={onPackage} disabled={mutationBusy}>
          <Archive size={13} /> {t("settingsTheme.package")}
        </BookButton>
      </div>
      <button
        type="button"
        onClick={onRefresh}
        disabled={loading || mutationBusy}
        className="pyqt-btn-icon !h-8 !w-8 shrink-0"
        title={t("settingsTheme.refresh")}
        aria-label={t("settingsTheme.refresh")}
      >
        <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
      </button>
    </div>

    {loadError ? <p role="alert" className="mb-3 text-sm text-error">{loadError}</p> : null}
    {loading && themes.length === 0 ? (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("settingsTheme.loading")}
      </div>
    ) : themes.length === 0 ? (loadError ? null : (
      <div className="a4n-empty-state a4n-empty-state--framed text-xs">
        {t("settingsTheme.empty")}
      </div>
    )) : (
      <ul className="grid grid-cols-1 items-start gap-2 xl:grid-cols-2">
        {themes.map((theme) => (
          <ThemeCard
            key={theme.id}
            theme={theme}
            activating={activatingId === theme.id}
            deleting={deletingId === theme.id}
            exporting={exportingId === theme.id}
            editing={editingId === theme.id}
            renaming={renamingId === theme.id}
            renameValue={renameValue}
            renameSaving={renameSaving && renamingId === theme.id}
            mutationBusy={mutationBusy}
            selectable={isDeletableTheme(theme)}
            selected={selectedIds.has(theme.id)}
            onRenameInput={onRenameInput}
            onRenameStart={() => onRenameStart(theme)}
            onRenameConfirm={onRenameConfirm}
            onRenameCancel={onRenameCancel}
            onActivate={() => onActivate(theme.id)}
            onEdit={() => onEdit(theme)}
            onDelete={() => onDelete(theme.id, theme.name)}
            onExport={() => onExport(theme.id)}
            onSelect={() => toggleSelected(theme.id)}
          />
        ))}
      </ul>
    )}
  </div>;
};
