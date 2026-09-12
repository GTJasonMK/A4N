import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useEffect, useState } from "react";
import { Download, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { type Theme, type ThemeMode } from "../../../api/theme";
import { pickLocalFile } from "../../../utils/fileUpload";
import { useI18n } from "../../../i18n";

interface CreateThemePanelProps {
  name: string;
  mode: ThemeMode;
  creating: boolean;
  disabled: boolean;
  onNameChange: (value: string) => void;
  onModeChange: (mode: ThemeMode) => void;
  onCreate: () => void;
}

export const CreateThemePanel: React.FC<CreateThemePanelProps> = ({
  name,
  mode,
  creating,
  disabled,
  onNameChange,
  onModeChange,
  onCreate,
}) => {
  const { t } = useI18n();
  return (
    <div className="grid gap-3">
      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
        <BookInput
          label={t("settingsTheme.name")}
          value={name}
          onChange={(event) => onNameChange(event.target.value)}
          placeholder={t("settingsTheme.namePlaceholder")}
          disabled={disabled}
        />
        <div className="flex flex-wrap items-center gap-2">
          <SegmentedControl
            ariaLabel={t("settingsTheme.mode")}
            value={mode}
            onChange={onModeChange}
            options={[
              { id: "light", label: t("settingsTheme.light"), disabled },
              { id: "dark", label: t("settingsTheme.dark"), disabled },
            ]}
          />
          <BookButton
            variant="primary"
            onClick={onCreate}
            disabled={disabled || name.trim().length === 0}
          >
            {creating ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> {t("settingsTheme.creating")}
              </span>
            ) : (
              t("settingsTheme.create")
            )}
          </BookButton>
        </div>
      </div>
      <p className="text-small text-book-text-muted">{t("settingsTheme.createAfter")}</p>
    </div>
  );
};

interface SingleThemeImportPanelProps {
  importText: string;
  importing: boolean;
  disabled: boolean;
  onImportTextChange: (value: string) => void;
  onImport: () => void;
}

export const SingleThemeImportPanel: React.FC<SingleThemeImportPanelProps> = ({
  importText,
  importing,
  disabled,
  onImportTextChange,
  onImport,
}) => {
  const { t } = useI18n();
  return (
    <div className="grid gap-3">
      <textarea
        value={importText}
        onChange={(event) => onImportTextChange(event.target.value)}
        disabled={disabled}
        aria-label={t("settingsTheme.import")}
        placeholder={t("settingsTheme.importPlaceholder")}
        className="pyqt-input h-24 w-full resize-y text-xs font-mono"
      />
      <div className="flex justify-end">
        <BookButton
          variant="primary"
          onClick={onImport}
          disabled={disabled || importText.trim().length === 0}
        >
          {importing ? (
            <span className="inline-flex items-center gap-1.5">
              <LoadingSpinner size="sm" /> {t("settingsTheme.importing")}
            </span>
          ) : (
            t("settingsTheme.import")
          )}
        </BookButton>
      </div>
    </div>
  );
};

interface ThemePackagePanelProps {
  themes: Theme[];
  exportingId: number | null;
  importing: boolean;
  disabled: boolean;
  onExport: (themeId: number) => void;
  onImport: (file: File) => void;
}

export const ThemePackagePanel: React.FC<ThemePackagePanelProps> = ({
  themes,
  exportingId,
  importing,
  disabled,
  onExport,
  onImport,
}) => {
  const { t } = useI18n();
  const [selectedId, setSelectedId] = useState<number | null>(themes[0]?.id ?? null);
  const [picking, setPicking] = useState(false);

  useEffect(() => {
    if (selectedId == null || !themes.some((theme) => theme.id === selectedId)) {
      setSelectedId(themes[0]?.id ?? null);
    }
  }, [selectedId, themes]);

  const handleImport = async () => {
    if (disabled || importing || picking) return;
    setPicking(true);
    try {
      const file = await pickLocalFile(".zip,application/zip,application/x-zip-compressed");
      if (file) onImport(file);
    } finally {
      setPicking(false);
    }
  };

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <div className="min-w-0">
        <label htmlFor="theme-package-theme" className="mb-1 block text-xs text-book-text-sub">
          {t("settingsTheme.packageTheme")}
        </label>
        <div className="flex min-w-0 flex-wrap gap-2">
          <ThemedSelect
            id="theme-package-theme"
            value={selectedId ?? ""}
            onChange={(event) => setSelectedId(Number(event.target.value))}
            disabled={disabled || themes.length === 0}
            className="pyqt-input min-w-0 flex-1 !min-h-9 !px-2 text-xs"
          >
            {themes.map((theme) => (
              <option key={theme.id} value={theme.id}>{theme.name}</option>
            ))}
          </ThemedSelect>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => selectedId != null && onExport(selectedId)}
            disabled={disabled || selectedId == null || exportingId != null}
          >
            {exportingId != null ? <LoadingSpinner size="sm" /> : <Download size={13} />}
            {exportingId != null ? t("settingsTheme.exportingPackage") : t("settingsTheme.exportPackage")}
          </BookButton>
        </div>
        <p className="mt-2 text-small text-book-text-muted">{t("settingsTheme.packageFormatHint")}</p>
      </div>
      <div className="flex min-w-0 flex-col justify-between gap-2 rounded-[var(--pyqt-radius-md)] border border-dashed border-book-border/45 bg-book-bg-tertiary/20 p-3">
        <div>
          <div className="text-xs font-medium text-book-text-sub">{t("settingsTheme.importPackage")}</div>
          <p className="mt-1 text-small leading-relaxed text-book-text-muted">{t("settingsTheme.importPackageHint")}</p>
        </div>
        <BookButton
          variant="secondary"
          size="sm"
          onClick={() => void handleImport()}
          disabled={disabled || importing || picking}
        >
          {importing || picking ? <LoadingSpinner size="sm" /> : <Upload size={13} />}
          {importing || picking ? t("settingsTheme.importingPackage") : t("settingsTheme.importPackage")}
        </BookButton>
      </div>
    </div>
  );
};
