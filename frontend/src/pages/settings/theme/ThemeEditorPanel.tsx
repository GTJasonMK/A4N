import React, { useEffect, useMemo, useState } from "react";
import {
  ImagePlus,
  RotateCcw,
  Save,
  Trash2,
  X,
} from "lucide-react";
import {
  type Theme,
  type ThemeBackgroundSettings,
  type ThemeDocument,
  type ThemeMode,
} from "../../../api/theme";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import {
  applyThemePreview,
  restoreActiveTheme,
} from "../../../theme/activeTheme";
import {
  THEME_ADVANCED_TOKEN_NAMES,
  THEME_COLOR_TOKEN_NAMES,
  THEME_DOCUMENT_VERSION,
  THEME_GLOBAL_ROUTE_FAMILY,
  THEME_ROUTE_FAMILIES,
} from "../../../theme/themeTokens";
import { parseCssRgb } from "../../../utils/cssColor";
import { pickLocalFile } from "../../../utils/fileUpload";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { useI18n } from "../../../i18n";
import { readThemePreset } from "./presets";

const DEFAULT_BACKGROUND: ThemeBackgroundSettings = {
  resource_key: null,
  opacity: 0.22,
  overlay: "#000000",
  overlay_opacity: 0.16,
  blur_px: 0,
  position: "center",
  size: "cover",
};
const SUPPORTED_THEME_IMAGE_TYPES = [
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/gif",
];

interface ThemeEditorPanelProps {
  theme: Theme;
  saving: boolean;
  onSave: (
    input: { name?: string; mode?: ThemeMode; document?: ThemeDocument },
    pendingFiles: Record<string, File>,
    deleteResourceIds: number[],
  ) => Promise<Theme | null>;
  onCancel: () => void;
}

function cloneDocument(theme: Theme): ThemeDocument {
  return {
    version: theme.document.version,
    tokens: { ...theme.document.tokens },
    backgrounds: Object.fromEntries(
      Object.entries(theme.document.backgrounds).map(([route, background]) => [
        route,
        { ...DEFAULT_BACKGROUND, ...background },
      ]),
    ),
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function opacityTokenValue(
  tokens: Record<string, string>,
  token: string,
  fallback: number,
): number {
  const raw = tokens[token];
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? clamp(parsed, 0, 1) : fallback;
}

function tokenToHex(value: string | undefined): string {
  const [red, green, blue] = parseCssRgb(value ?? "", [128, 128, 128]);
  return `#${[red, green, blue]
    .map((channel) => clamp(Math.round(channel), 0, 255).toString(16).padStart(2, "0"))
    .join("")}`;
}

function hexToToken(value: string): string | null {
  const match = value.trim().match(/^#?([0-9a-f]{6})$/i);
  if (!match) return null;
  return [0, 2, 4]
    .map((offset) => Number.parseInt(match[1].slice(offset, offset + 2), 16))
    .join(" ");
}

function hexColor(value: string | undefined): string {
  const match = (value ?? "").trim().match(/^#([0-9a-f]{3,8})$/i);
  if (!match) return "#000000";
  const raw = match[1];
  if (raw.length === 3) {
    return `#${raw
      .split("")
      .map((channel) => `${channel}${channel}`)
      .join("")}`;
  }
  return `#${raw.slice(0, 6)}`;
}

function tokenLabel(token: string): string {
  return token.replace(/^--/, "").replace(/-/g, " ");
}

function draftPreviewTheme(
  theme: Theme,
  document: ThemeDocument,
  pendingFiles: Record<string, File>,
  pendingUrls: Record<string, string>,
  deleteResourceIds: number[],
): Theme {
  const deleted = new Set(deleteResourceIds);
  const resources = theme.resources
    .filter((resource) => !deleted.has(resource.id))
    .filter((resource) => !pendingUrls[resource.resource_key]);
  for (const [resourceKey, file] of Object.entries(pendingFiles)) {
    const filePath = pendingUrls[resourceKey];
    if (!filePath) continue;
    resources.push({
      id: -resourceKey.length,
      theme_id: theme.id,
      resource_key: resourceKey,
      file_name: file.name,
      mime_type: file.type,
      file_path: filePath,
      byte_size: file.size,
    });
  }
  return { ...theme, document, resources };
}

export const ThemeEditorPanel: React.FC<ThemeEditorPanelProps> = ({
  theme,
  saving,
  onSave,
  onCancel,
}) => {
  const { t } = useI18n();
  const [name, setName] = useState(theme.name);
  const [mode, setMode] = useState<ThemeMode>(theme.mode);
  const [document, setDocument] = useState<ThemeDocument>(() => cloneDocument(theme));
  const [backgroundRoute, setBackgroundRoute] = useState(THEME_GLOBAL_ROUTE_FAMILY);
  const [pendingFiles, setPendingFiles] = useState<Record<string, File>>({});
  const [pendingUrls, setPendingUrls] = useState<Record<string, string>>({});
  const [deleteResourceIds, setDeleteResourceIds] = useState<number[]>([]);
  const [pickingBackground, setPickingBackground] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const preview = draftPreviewTheme(
      { ...theme, mode },
      { ...document, tokens: { ...document.tokens } },
      pendingFiles,
      pendingUrls,
      deleteResourceIds,
    );
    applyThemePreview(preview);
  }, [deleteResourceIds, document, mode, pendingFiles, pendingUrls, theme]);

  useEffect(() => {
    const urls = Object.values(pendingUrls);
    return () => urls.forEach((url) => URL.revokeObjectURL(url));
  }, [pendingUrls]);

  useEffect(() => () => restoreActiveTheme(), []);

  const currentBackground = useMemo(() => {
    const configured =
      document.backgrounds[backgroundRoute] ??
      (backgroundRoute === THEME_GLOBAL_ROUTE_FAMILY
        ? undefined
        : document.backgrounds[THEME_GLOBAL_ROUTE_FAMILY]);
    return { ...DEFAULT_BACKGROUND, ...configured };
  }, [backgroundRoute, document.backgrounds]);
  const currentResource = theme.resources.find(
    (resource) => resource.resource_key === currentBackground.resource_key,
  );
  const removableResource =
    currentResource?.resource_key === backgroundRoute ? currentResource : undefined;
  const currentPreviewUrl = pendingUrls[backgroundRoute];
  const currentBackgroundUrl =
    currentPreviewUrl ??
    (currentResource && !deleteResourceIds.includes(currentResource.id)
      ? toMediaSrc(currentResource.file_path)
      : "");

  const updateToken = (token: string, value: string) => {
    setDocument((current) => {
      const tokens = { ...current.tokens };
      if (value.trim()) tokens[token] = value;
      else delete tokens[token];
      return { ...current, tokens };
    });
  };

  const updateBackground = (patch: Partial<ThemeBackgroundSettings>) => {
    setDocument((current) => ({
      ...current,
      backgrounds: {
        ...current.backgrounds,
        [backgroundRoute]: {
          ...DEFAULT_BACKGROUND,
          ...(current.backgrounds[backgroundRoute] ??
            (backgroundRoute === THEME_GLOBAL_ROUTE_FAMILY
              ? {}
              : current.backgrounds[THEME_GLOBAL_ROUTE_FAMILY] ?? {})),
          ...patch,
        },
      },
    }));
  };

  const setNumberToken = (token: string, fallback: number, value: string) => {
    const parsed = Number(value);
    updateToken(token, Number.isFinite(parsed) ? String(parsed) : String(fallback));
  };

  const setOpacityToken = (token: string, value: string) => {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) updateToken(token, String(clamp(parsed, 0, 1)));
  };

  const handleFile = (file: File | undefined) => {
    if (!file) return;
    if (!SUPPORTED_THEME_IMAGE_TYPES.includes(file.type) || file.size > 20 * 1024 * 1024) {
      setError(t("settingsTheme.resourceInvalid"));
      return;
    }
    const nextUrl = URL.createObjectURL(file);
    setPendingFiles((current) => ({ ...current, [backgroundRoute]: file }));
    setPendingUrls((current) => ({ ...current, [backgroundRoute]: nextUrl }));
    setDeleteResourceIds((current) =>
      current.filter((resourceId) => resourceId !== removableResource?.id),
    );
    updateBackground({ resource_key: backgroundRoute });
    setError(null);
  };

  const handlePickBackground = async () => {
    if (saving || pickingBackground) return;
    setPickingBackground(true);
    try {
      const file = await pickLocalFile(SUPPORTED_THEME_IMAGE_TYPES.join(","));
      handleFile(file ?? undefined);
    } catch {
      setError(t("settingsTheme.filePickerFailed"));
    } finally {
      setPickingBackground(false);
    }
  };

  const handleRemoveBackground = () => {
    setPendingFiles((current) => {
      const next = { ...current };
      delete next[backgroundRoute];
      return next;
    });
    setPendingUrls((current) => {
      const next = { ...current };
      delete next[backgroundRoute];
      return next;
    });
    if (removableResource) {
      setDeleteResourceIds((current) =>
        current.includes(removableResource.id)
          ? current
          : [...current, removableResource.id],
      );
    }
    updateBackground({ resource_key: null });
  };

  const resetDraft = () => {
    setName(theme.name);
    setMode(theme.mode);
    setDocument(cloneDocument(theme));
    setPendingFiles({});
    setPendingUrls({});
    setDeleteResourceIds([]);
    setError(null);
  };

  const restoreDefault = () => {
    setDocument({
      version: THEME_DOCUMENT_VERSION,
      tokens: readThemePreset(mode),
      backgrounds: {},
    });
    setPendingFiles({});
    setPendingUrls({});
    setDeleteResourceIds(theme.resources.map((resource) => resource.id));
    setError(null);
  };

  const handleSave = async () => {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setError(t("settingsTheme.nameRequired"));
      return;
    }
    setError(null);
    const saved = await onSave(
      { name: trimmedName, mode, document },
      pendingFiles,
      deleteResourceIds,
    );
    if (saved) {
      setName(saved.name);
      setMode(saved.mode);
      setDocument(cloneDocument(saved));
      setPendingFiles({});
      setPendingUrls({});
      setDeleteResourceIds([]);
    }
  };

  const setBackgroundNumber = (
    key: "opacity" | "overlay_opacity" | "blur_px",
    value: string,
  ) => {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) updateBackground({ [key]: parsed });
  };

  return (
    <section className="a4n-flat-surface min-w-0 overflow-hidden" aria-labelledby="theme-editor-title">
      <div className="flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/35 px-3 py-3 sm:px-4">
        <div className="min-w-0 flex-1">
          <h2 id="theme-editor-title" className="truncate text-sm font-semibold text-book-text-main">
            {t("settingsTheme.editorTitle")}: {theme.name}
          </h2>
          <p className="mt-0.5 text-small text-book-text-muted">{t("settingsTheme.editorHint")}</p>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-1.5">
          <BookButton variant="primary" size="sm" onClick={handleSave} disabled={saving}>
            {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
            {t("common.save")}
          </BookButton>
          <BookButton variant="ghost" size="sm" onClick={onCancel} disabled={saving}>
            <X size={13} /> {t("settingsTheme.cancel")}
          </BookButton>
        </div>
      </div>

      <div className="grid min-w-0 gap-4 p-3 sm:p-4">
        <div className="grid min-w-0 gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <BookInput
            label={t("settingsTheme.name")}
            value={name}
            onChange={(event) => setName(event.target.value)}
            disabled={saving}
          />
          <SegmentedControl
            ariaLabel={t("settingsTheme.mode")}
            value={mode}
            onChange={setMode}
            options={[
              { id: "light", label: t("settingsTheme.light"), disabled: saving },
              { id: "dark", label: t("settingsTheme.dark"), disabled: saving },
            ]}
          />
        </div>

        <section aria-labelledby="theme-editor-colors">
          <h3 id="theme-editor-colors" className="mb-2 text-xs font-semibold text-book-text-main">
            {t("settingsTheme.colors")}
          </h3>
          <div className="grid min-w-0 grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-3">
            {THEME_COLOR_TOKEN_NAMES.map((token) => (
              <label key={token} className="flex min-w-0 items-center gap-2 rounded-[var(--pyqt-radius-sm)] border border-book-border/25 bg-book-bg-tertiary/25 px-2 py-1.5">
                <input
                  type="color"
                  value={tokenToHex(document.tokens[token])}
                  onChange={(event) => updateToken(token, hexToToken(event.target.value) ?? "")}
                  disabled={saving}
                  aria-label={tokenLabel(token)}
                  title={tokenToHex(document.tokens[token]).toUpperCase()}
                  className="h-8 w-9 shrink-0 cursor-pointer rounded border-0 bg-transparent p-0"
                />
                <span className="min-w-0 flex-1 truncate font-mono text-[0.68rem] text-book-text-sub" title={token}>
                  {tokenLabel(token)}
                </span>
                <code className="shrink-0 font-mono text-[0.65rem] text-book-text-muted">
                  {tokenToHex(document.tokens[token]).toUpperCase()}
                </code>
              </label>
            ))}
          </div>
        </section>

        <section aria-labelledby="theme-editor-effects" className="grid min-w-0 gap-3">
          <h3 id="theme-editor-effects" className="text-xs font-semibold text-book-text-main">
            {t("settingsTheme.effects")}
          </h3>
          <div className="grid min-w-0 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <RangeField
              label={t("settingsTheme.workspaceOpacity")}
              value={opacityTokenValue(document.tokens, "--a4n-workspace-opacity", 0.78)}
              min={0}
              max={1}
              step={0.01}
              suffix=""
              disabled={saving}
              onChange={(value) => setOpacityToken("--a4n-workspace-opacity", value)}
            />
            <RangeField
              label={t("settingsTheme.panelOpacity")}
              value={opacityTokenValue(document.tokens, "--a4n-panel-opacity", 0.82)}
              min={0}
              max={1}
              step={0.01}
              suffix=""
              disabled={saving}
              onChange={(value) => setOpacityToken("--a4n-panel-opacity", value)}
            />
            <RangeField
              label={t("settingsTheme.toolbarOpacity")}
              value={opacityTokenValue(document.tokens, "--a4n-toolbar-opacity", 0.8)}
              min={0}
              max={1}
              step={0.01}
              suffix=""
              disabled={saving}
              onChange={(value) => setOpacityToken("--a4n-toolbar-opacity", value)}
            />
            <RangeField
              label={t("settingsTheme.cardOpacity")}
              value={opacityTokenValue(document.tokens, "--a4n-card-opacity", 0.9)}
              min={0}
              max={1}
              step={0.01}
              suffix=""
              disabled={saving}
              onChange={(value) => setOpacityToken("--a4n-card-opacity", value)}
            />
            <RangeField
              label={t("settingsTheme.controlOpacity")}
              value={opacityTokenValue(document.tokens, "--a4n-control-opacity", 0.48)}
              min={0}
              max={1}
              step={0.01}
              suffix=""
              disabled={saving}
              onChange={(value) => setOpacityToken("--a4n-control-opacity", value)}
            />
            <RangeField
              label={t("settingsTheme.dialogOpacity")}
              value={opacityTokenValue(document.tokens, "--a4n-dialog-opacity", 0.94)}
              min={0}
              max={1}
              step={0.01}
              suffix=""
              disabled={saving}
              onChange={(value) => setOpacityToken("--a4n-dialog-opacity", value)}
            />
            <RangeField
              label={t("settingsTheme.surfaceBlur")}
              value={Number.parseFloat(document.tokens["--a4n-surface-blur"] ?? "0") || 0}
              min={0}
              max={24}
              step={1}
              suffix="px"
              disabled={saving}
              onChange={(value) => setNumberToken("--a4n-surface-blur", 0, value)}
            />
            <RangeField
              label={t("settingsTheme.shadowStrength")}
              value={Number(document.tokens["--a4n-shadow-strength"] ?? 1)}
              min={0}
              max={1.5}
              step={0.05}
              suffix=""
              disabled={saving}
              onChange={(value) => setNumberToken("--a4n-shadow-strength", 1, value)}
            />
            <RangeField
              label={t("settingsTheme.radius")}
              value={Number.parseFloat(document.tokens["--pyqt-radius-md"] ?? "12") || 12}
              min={0}
              max={24}
              step={1}
              suffix="px"
              disabled={saving}
              onChange={(value) => {
                const number = Number(value);
                setNumberToken("--pyqt-radius-xs", Math.max(number - 6, 0), String(Math.max(number - 6, 0)));
                setNumberToken("--pyqt-radius-sm", Math.max(number - 2, 0), String(Math.max(number - 2, 0)));
                setNumberToken("--pyqt-radius-md", 12, value);
                setNumberToken("--pyqt-radius-lg", number + 4, String(number + 4));
                setNumberToken("--pyqt-radius-xl", number + 10, String(number + 10));
              }}
            />
          </div>
        </section>

        <section aria-labelledby="theme-editor-background" className="grid min-w-0 gap-3">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <h3 id="theme-editor-background" className="min-w-0 flex-1 text-xs font-semibold text-book-text-main">
              {t("settingsTheme.backgrounds")}
            </h3>
            <select
              aria-label={t("settingsTheme.backgroundRoute")}
              value={backgroundRoute}
              onChange={(event) => setBackgroundRoute(event.target.value)}
              className="pyqt-input !min-h-8 !w-auto max-w-full !px-2 text-xs"
              disabled={saving}
            >
              {THEME_ROUTE_FAMILIES.map((route) => (
                <option key={route} value={route}>{route}</option>
              ))}
            </select>
          </div>
          <div className="grid min-w-0 gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(15rem,0.8fr)]">
            <div className="a4n-theme-editor-preview relative min-h-36 overflow-hidden rounded-[var(--pyqt-radius-md)] border border-book-border/35 bg-book-bg-tertiary/35">
              {currentBackgroundUrl ? (
                <span
                  className="absolute -inset-8 bg-center bg-no-repeat"
                  aria-hidden="true"
                  style={{
                    backgroundImage: `url(${JSON.stringify(currentBackgroundUrl)})`,
                    backgroundPosition: currentBackground.position,
                    backgroundSize: currentBackground.size,
                    filter: `blur(${currentBackground.blur_px}px)`,
                    opacity: currentBackground.opacity,
                  }}
                />
              ) : null}
              <span
                className="absolute inset-0"
                aria-hidden="true"
                style={{
                  backgroundColor: currentBackground.overlay,
                  opacity: currentBackground.overlay_opacity,
                }}
              />
              <div className="absolute inset-0 flex items-end bg-gradient-to-t from-black/65 via-black/10 to-transparent p-3 text-xs text-white">
                {currentResource?.file_name ?? pendingFiles[backgroundRoute]?.name ?? t("settingsTheme.noBackground")}
              </div>
            </div>
            <div className="grid min-w-0 gap-2 sm:grid-cols-2 lg:grid-cols-1">
              <label className="text-xs text-book-text-sub">
                {t("settingsTheme.backgroundOpacity")} · {Math.round(currentBackground.opacity * 100)}%
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={currentBackground.opacity}
                  onChange={(event) => setBackgroundNumber("opacity", event.target.value)}
                  disabled={saving}
                  className="a4n-range mt-1 w-full"
                />
              </label>
              <label className="text-xs text-book-text-sub">
                {t("settingsTheme.overlayOpacity")} · {Math.round(currentBackground.overlay_opacity * 100)}%
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={currentBackground.overlay_opacity}
                  onChange={(event) => setBackgroundNumber("overlay_opacity", event.target.value)}
                  disabled={saving}
                  className="a4n-range mt-1 w-full"
                />
              </label>
              <label className="text-xs text-book-text-sub">
                {t("settingsTheme.backgroundBlur")} · {currentBackground.blur_px}px
                <input
                  type="range"
                  min="0"
                  max="40"
                  step="1"
                  value={currentBackground.blur_px}
                  onChange={(event) => setBackgroundNumber("blur_px", event.target.value)}
                  disabled={saving}
                  className="a4n-range mt-1 w-full"
                />
              </label>
              <label className="flex min-w-0 items-center justify-between gap-2 text-xs text-book-text-sub">
                <span>{t("settingsTheme.backgroundOverlay")}</span>
                <input
                  type="color"
                  value={hexColor(currentBackground.overlay)}
                  onChange={(event) => updateBackground({ overlay: event.target.value })}
                  disabled={saving}
                  aria-label={t("settingsTheme.backgroundOverlay")}
                  className="h-8 w-10 shrink-0 cursor-pointer rounded border-0 bg-transparent p-0"
                />
              </label>
              <label className="min-w-0 text-xs text-book-text-sub">
                <span className="mb-1 block">{t("settingsTheme.backgroundPosition")}</span>
                <select
                  value={currentBackground.position}
                  onChange={(event) => updateBackground({ position: event.target.value })}
                  disabled={saving}
                  className="pyqt-input !min-h-8 w-full !px-2 text-xs"
                >
                  {(["center", "top", "bottom", "left", "right"] as const).map((position) => (
                    <option key={position} value={position}>{position}</option>
                  ))}
                </select>
              </label>
              <label className="min-w-0 text-xs text-book-text-sub">
                <span className="mb-1 block">{t("settingsTheme.backgroundSize")}</span>
                <select
                  value={currentBackground.size}
                  onChange={(event) => updateBackground({ size: event.target.value })}
                  disabled={saving}
                  className="pyqt-input !min-h-8 w-full !px-2 text-xs"
                >
                  {(["cover", "contain", "auto"] as const).map((size) => (
                    <option key={size} value={size}>{size}</option>
                  ))}
                </select>
              </label>
              <div className="flex min-w-0 flex-wrap gap-2 sm:col-span-2 lg:col-span-1">
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => void handlePickBackground()}
                  disabled={saving || pickingBackground}
                >
                  {pickingBackground ? <LoadingSpinner size="sm" /> : <ImagePlus size={13} />}
                  {t("settingsTheme.chooseBackground")}
                </BookButton>
                {currentResource || pendingFiles[backgroundRoute] ? (
                  <BookButton
                    variant="ghost"
                    size="sm"
                    onClick={handleRemoveBackground}
                    disabled={saving || pickingBackground}
                  >
                    <Trash2 size={13} /> {t("settingsTheme.removeBackground")}
                  </BookButton>
                ) : null}
              </div>
            </div>
          </div>
        </section>

        <details className="min-w-0 rounded-[var(--pyqt-radius-md)] border border-book-border/30 bg-book-bg-tertiary/15">
          <summary className="cursor-pointer list-none px-3 py-2 text-xs font-semibold text-book-text-main [&::-webkit-details-marker]:hidden">
            {t("settingsTheme.advancedTokens")}
          </summary>
          <div className="grid min-w-0 gap-2 border-t border-book-border/25 p-3 sm:grid-cols-2">
            {THEME_ADVANCED_TOKEN_NAMES.map((token) => (
              <label key={token} className="min-w-0 text-xs text-book-text-sub">
                <span className="mb-1 block truncate font-mono" title={token}>{token}</span>
                <input
                  value={document.tokens[token] ?? ""}
                  onChange={(event) => updateToken(token, event.target.value)}
                  placeholder={t("settingsTheme.tokenFallback")}
                  disabled={saving}
                  className="pyqt-input w-full !min-h-8 !px-2 text-xs"
                />
              </label>
            ))}
          </div>
        </details>

        {error ? <p role="alert" className="text-xs text-error">{error}</p> : null}
        <div className="flex min-w-0 flex-wrap items-center justify-end gap-2 border-t border-book-border/30 pt-3">
          <BookButton
            variant="secondary"
            onClick={restoreDefault}
            disabled={saving || pickingBackground}
          >
            <RotateCcw size={13} /> {t("settingsTheme.restoreDefault")}
          </BookButton>
          <BookButton variant="ghost" onClick={resetDraft} disabled={saving}>
            <RotateCcw size={13} /> {t("settingsTheme.reset")}
          </BookButton>
          <BookButton variant="primary" onClick={handleSave} disabled={saving}>
            {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
            {saving ? t("settingsTheme.saving") : t("common.save")}
          </BookButton>
        </div>
      </div>
    </section>
  );
};

const RangeField: React.FC<{
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  suffix: string;
  disabled: boolean;
  onChange: (value: string) => void;
}> = ({ label, value, min, max, step, suffix, disabled, onChange }) => (
  <label className="min-w-0 text-xs text-book-text-sub">
    <span className="flex items-center justify-between gap-2">
      <span className="truncate">{label}</span>
      <span className="shrink-0 font-mono text-book-text-muted">
        {value.toFixed(step < 1 ? 2 : 0)}{suffix}
      </span>
    </span>
    <input
      type="range"
      min={min}
      max={max}
      step={step}
      value={Number.isFinite(value) ? value : min}
      onChange={(event) => onChange(event.target.value)}
      disabled={disabled}
      className="a4n-range mt-1 w-full"
    />
  </label>
);
