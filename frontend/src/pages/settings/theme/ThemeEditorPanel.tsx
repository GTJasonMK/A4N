import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useEffect, useLayoutEffect, useMemo, useState } from "react";
import {
  ChevronDown,
  ImagePlus,
  RotateCcw,
  Trash2,
} from "lucide-react";
import {
  type Theme,
  type ThemeBackgroundSettings,
  type ThemeDocument,
  type ThemeMode,
  type ThemeStreamDisplaySettings,
} from "../../../api/theme";
import {
  IMAGE_UPLOAD_MIME_TYPES,
  MAX_THEME_RESOURCE_BYTES,
} from "../../../api/generated/config";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useDirty } from "../../../components/navigation/dirtyRegistry";
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
import { useI18n, type MessageTranslator, type MessageKey } from "../../../i18n";
import { StreamDisplaySettingsPanel } from "../advanced/StreamDisplaySettingsPanel";
import {
  DEFAULT_STREAM_DISPLAY_SETTINGS,
  normalizeStreamDisplaySettings,
} from "../../../utils/streamDisplaySettings";
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
const SUPPORTED_THEME_IMAGE_TYPES = [...IMAGE_UPLOAD_MIME_TYPES, "image/gif"];
export const THEME_EDITOR_FORM_ID = "theme-editor-form";

interface ThemeEditorPanelProps {
  theme: Theme;
  saving: boolean;
  onSave: (
    input: { name?: string; mode?: ThemeMode; document?: ThemeDocument },
    pendingFiles: Record<string, File>,
    deleteResourceIds: number[],
  ) => Promise<Theme | null>;
  onDirtyChange?: (dirty: boolean) => void;
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
    stream_display: normalizeStreamDisplaySettings(theme.document.stream_display),
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

function tokenLabel(token: string, t: MessageTranslator): string {
  return t(`settingsTheme.tokenLabels.${token.slice(2)}` as MessageKey);
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
  onDirtyChange,
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

  useLayoutEffect(() => {
    const urls = Object.fromEntries(Object.entries(pendingFiles).map(([key, file]) => [key, URL.createObjectURL(file)]));
    setPendingUrls(urls);
    return () => Object.values(urls).forEach((url) => URL.revokeObjectURL(url));
  }, [pendingFiles]);

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
  const currentPreviewFile = currentBackground.resource_key
    ? pendingFiles[currentBackground.resource_key] : undefined;
  const currentPreviewUrl = currentBackground.resource_key
    ? pendingUrls[currentBackground.resource_key] : undefined;
  const currentBackgroundUrl =
    currentPreviewUrl ??
    (currentResource && !deleteResourceIds.includes(currentResource.id)
      ? toMediaSrc(currentResource.file_path)
      : "");
  const streamDisplay = normalizeStreamDisplaySettings(document.stream_display);
  const isDirty =
    name !== theme.name ||
    mode !== theme.mode ||
    JSON.stringify(document) !== JSON.stringify(cloneDocument(theme)) ||
    Object.keys(pendingFiles).length > 0 ||
    deleteResourceIds.length > 0;
  useDirty("settings-theme-editor", isDirty, undefined, () => !saving);

  useEffect(() => {
    onDirtyChange?.(isDirty);
  }, [isDirty, onDirtyChange]);

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

  const updateStreamDisplay = (patch: Partial<ThemeStreamDisplaySettings>) => {
    setDocument((current) => ({
      ...current,
      stream_display: {
        ...normalizeStreamDisplaySettings(current.stream_display),
        ...patch,
      },
    }));
  };

  const setOpacityToken = (token: string, value: string) => {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) updateToken(token, String(clamp(parsed, 0, 1)));
  };

  const handleFile = (file: File | undefined) => {
    if (!file) return;
    if (!SUPPORTED_THEME_IMAGE_TYPES.includes(file.type) || file.size > MAX_THEME_RESOURCE_BYTES) {
      setError(t("settingsTheme.resourceInvalid", {
        size: MAX_THEME_RESOURCE_BYTES / (1024 * 1024),
      }));
      return;
    }
    setPendingFiles((current) => ({ ...current, [backgroundRoute]: file }));
    const replacedResource = theme.resources.find((resource) => resource.resource_key === backgroundRoute);
    setDeleteResourceIds((current) =>
      current.filter((resourceId) => resourceId !== replacedResource?.id),
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
    if (removableResource) {
      setDeleteResourceIds((current) =>
        current.includes(removableResource.id)
          ? current
          : [...current, removableResource.id],
      );
    }
    updateBackground({ resource_key: null });
  };

  const confirmDiscardDraft = async (action: string): Promise<boolean> => {
    if (!isDirty) return true;
    return confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: theme.name,
        action,
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    });
  };

  const resetDraft = async () => {
    if (!(await confirmDiscardDraft(t("settingsTheme.reset")))) return;
    setName(theme.name);
    setMode(theme.mode);
    setDocument(cloneDocument(theme));
    setPendingFiles({});
    setDeleteResourceIds([]);
    setError(null);
  };

  const restoreDefault = async () => {
    if (!(await confirmDiscardDraft(t("settingsTheme.restoreDefault")))) return;
    setDocument({
      version: THEME_DOCUMENT_VERSION,
      tokens: readThemePreset(mode),
      backgrounds: {},
      stream_display: { ...DEFAULT_STREAM_DISPLAY_SETTINGS },
    });
    setPendingFiles({});
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
    <form
      id={THEME_EDITOR_FORM_ID}
      className="min-w-0"
      aria-label={t("settingsTheme.editorTitle")}
      data-settings-dirty={isDirty ? "true" : undefined}
      onSubmit={(event) => { event.preventDefault(); void handleSave(); }}
    >
      <div className="grid min-w-0 gap-4">
        <div id="theme-editor-title" className="grid min-w-0 gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
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
          <div className="grid min-w-0 grid-cols-1 gap-2 sm:grid-cols-2">
            {THEME_COLOR_TOKEN_NAMES.map((token) => (
              <label key={token} className="flex min-w-0 items-center gap-2 rounded-[var(--pyqt-radius-sm)] border border-book-border/25 bg-book-bg-tertiary/25 px-2 py-1.5">
                <input
                  type="color"
                  value={tokenToHex(document.tokens[token])}
                  onChange={(event) => updateToken(token, hexToToken(event.target.value) ?? "")}
                  disabled={saving}
                  aria-label={tokenLabel(token, t)}
                  title={tokenToHex(document.tokens[token]).toUpperCase()}
                  className="h-8 w-9 shrink-0 cursor-pointer rounded border-0 bg-transparent p-0"
                />
                <span className="min-w-0 flex-1 text-xs text-book-text-sub" title={token}>
                  {tokenLabel(token, t)}
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
            <ThemedSelect
              aria-label={t("settingsTheme.backgroundRoute")}
              value={backgroundRoute}
              onChange={setBackgroundRoute}
              className="pyqt-input !min-h-8 !w-auto max-w-full !px-2 text-xs"
              disabled={saving}
            >
              {THEME_ROUTE_FAMILIES.map((route) => (
                <option key={route} value={route}>{t(`settingsTheme.routeLabels.${route}` as MessageKey)}</option>
              ))}
            </ThemedSelect>
          </div>
          <div className="grid min-w-0 gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(15rem,0.8fr)]">
            <div className="a4n-theme-editor-preview relative aspect-video w-full self-start overflow-hidden rounded-[var(--pyqt-radius-md)] border border-book-border/35 bg-book-bg-tertiary/35">
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
                {currentPreviewFile?.name ?? (currentBackgroundUrl ? currentResource?.file_name : undefined) ?? t("settingsTheme.noBackground")}
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
                <ThemedSelect
                  value={currentBackground.position}
                  onChange={(selectedValue) => updateBackground({ position: selectedValue })}
                  disabled={saving}
                  className="pyqt-input !min-h-8 w-full !px-2 text-xs"
                >
                  {(["center", "top", "bottom", "left", "right"] as const).map((position) => (
                    <option key={position} value={position}>{t(`settingsTheme.positionLabels.${position}` as MessageKey)}</option>
                  ))}
                </ThemedSelect>
              </label>
              <label className="min-w-0 text-xs text-book-text-sub">
                <span className="mb-1 block">{t("settingsTheme.backgroundSize")}</span>
                <ThemedSelect
                  value={currentBackground.size}
                  onChange={(selectedValue) => updateBackground({ size: selectedValue })}
                  disabled={saving}
                  className="pyqt-input !min-h-8 w-full !px-2 text-xs"
                >
                  {(["cover", "contain", "auto"] as const).map((size) => (
                    <option key={size} value={size}>{t(`settingsTheme.sizeLabels.${size}` as MessageKey)}</option>
                  ))}
                </ThemedSelect>
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
                {currentBackgroundUrl ? (
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

        <details className="group">
          <summary className="inline-flex cursor-pointer list-none items-center gap-1.5 py-2 text-xs font-semibold text-book-text-main [&::-webkit-details-marker]:hidden">
            <ChevronDown size={12} aria-hidden="true" className="shrink-0 transition-transform group-open:rotate-180" />
            {t("settingsTheme.streamDisplay")}
          </summary>
          <StreamDisplaySettingsPanel value={streamDisplay} disabled={saving} onChange={updateStreamDisplay} />
        </details>

        <details className="group min-w-0 rounded-[var(--pyqt-radius-md)] border border-book-border/30 bg-book-bg-tertiary/15">
          <summary className="flex w-full cursor-pointer list-none items-center gap-1.5 px-3 py-2 text-xs font-semibold text-book-text-main [&::-webkit-details-marker]:hidden">
            <ChevronDown size={12} aria-hidden="true" className="shrink-0 transition-transform group-open:rotate-180" />
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
            onClick={() => void restoreDefault()}
            disabled={saving || pickingBackground}
          >
            <RotateCcw size={13} /> {t("settingsTheme.restoreDefault")}
          </BookButton>
          <BookButton variant="ghost" onClick={() => void resetDraft()} disabled={saving}>
            <RotateCcw size={13} /> {t("settingsTheme.reset")}
          </BookButton>
        </div>
      </div>
    </form>
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
