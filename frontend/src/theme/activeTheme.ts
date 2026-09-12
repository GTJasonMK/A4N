import {
  getThemePreference,
  listThemes,
  updateThemePreference,
  type Theme,
  type ThemeAppearance,
  type ThemeMode,
  type ThemePreferenceSummary,
  type ThemeStreamDisplaySettings,
} from "../api/theme";
import { toMediaSrc } from "../utils/mediaSrc";
import { normalizeStreamDisplaySettings } from "../utils/streamDisplaySettings";
import {
  THEME_DOCUMENT_VERSION,
  THEME_GLOBAL_ROUTE_FAMILY,
  THEME_TOKEN_NAMES,
  type ThemeRouteFamily,
} from "./themeTokens";

export const THEME_APPLIED_EVENT = "a4n:theme-applied";
const THEME_CACHE_PREFIX = "a4n:theme-runtime:v2:";
const appliedThemeVariables = new Set<string>();
const themeTokenSet = new Set<string>(THEME_TOKEN_NAMES);

let runtimeUserId: number | null = null;
let runtimeThemes: Theme[] = [];
let runtimeAppearance: ThemeAppearance = "system";
let runtimeActiveTheme: Theme | null = null;
let runtimeAppliedTheme: Theme | null = null;
let runtimeRouteFamily: ThemeRouteFamily = THEME_GLOBAL_ROUTE_FAMILY;
let syncChannel: BroadcastChannel | null = null;
let systemMediaQuery: MediaQueryList | null = null;
let syncStorageListenerInstalled = false;

function dispatchThemeApplied(): void {
  window.dispatchEvent(new Event(THEME_APPLIED_EVENT));
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function systemThemeMode(): ThemeMode {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function preferredThemeMode(appearance: ThemeAppearance): ThemeMode {
  return appearance === "system" ? systemThemeMode() : appearance;
}

function selectActiveTheme(
  themes: Theme[],
  userId: number,
  mode: ThemeMode,
): Theme | null {
  return (
    themes.find((theme) => theme.is_active && theme.user_id === userId && theme.mode === mode) ??
    themes.find((theme) => theme.is_active && theme.user_id == null && theme.mode === mode) ??
    null
  );
}

function cacheKey(userId: number): string {
  return `${THEME_CACHE_PREFIX}${userId}`;
}

function readThemeCache(userId: number): { appearance: ThemeAppearance; themes: Theme[] } | null {
  try {
    const raw = localStorage.getItem(cacheKey(userId));
    if (!raw) return null;
    const value = JSON.parse(raw) as { appearance?: ThemeAppearance; themes?: Theme[] };
    if (!Array.isArray(value.themes)) return null;
    if (value.appearance !== "system" && value.appearance !== "light" && value.appearance !== "dark") {
      return null;
    }
    return { appearance: value.appearance, themes: value.themes };
  } catch {
    return null;
  }
}

function writeThemeCache(userId: number, themes: Theme[], appearance: ThemeAppearance): void {
  try {
    localStorage.setItem(cacheKey(userId), JSON.stringify({ appearance, themes }));
  } catch {
    // 缓存不可用不影响主题应用。
  }
}

function broadcastThemeSync(): void {
  const message = JSON.stringify({ userId: runtimeUserId, at: Date.now() });
  try {
    if (syncChannel) syncChannel.postMessage(message);
    else localStorage.setItem("a4n:theme-sync", message);
  } catch {
    // 跨标签同步受限时保留当前标签页行为。
  }
}

function ensureSyncListeners(): void {
  if (typeof window === "undefined") return;
  if ("BroadcastChannel" in window && !syncChannel) {
    syncChannel = new BroadcastChannel("a4n-theme-sync");
    syncChannel.onmessage = (event) => {
      try {
        const message = JSON.parse(String(event.data)) as { userId?: number | null };
        if (message.userId === runtimeUserId) void reloadRuntimeTheme();
      } catch {
        // Ignore unrelated channel messages.
      }
    };
  }
  if (!systemMediaQuery) {
    systemMediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const onSystemThemeChange = () => {
      if (runtimeAppearance === "system") applyRuntimeTheme();
    };
    systemMediaQuery.addEventListener?.("change", onSystemThemeChange);
  }
  if (!syncChannel && !syncStorageListenerInstalled) {
    window.addEventListener("storage", (event) => {
      if (event.key !== "a4n:theme-sync" || !event.newValue) return;
      try {
        const message = JSON.parse(event.newValue) as { userId?: number | null };
        if (message.userId === runtimeUserId) void reloadRuntimeTheme();
      } catch {
        // Ignore malformed cross-tab notifications.
      }
    });
    syncStorageListenerInstalled = true;
  }
}

function applyBackground(theme: Theme | null): void {
  const root = document.documentElement;
  const documentValue: Theme["document"] =
    theme?.document ?? {
      version: THEME_DOCUMENT_VERSION,
      tokens: {},
      backgrounds: {},
      stream_display: normalizeStreamDisplaySettings(undefined),
    };
  const background =
    documentValue.backgrounds[runtimeRouteFamily] ??
    documentValue.backgrounds[THEME_GLOBAL_ROUTE_FAMILY];
  const resource = background?.resource_key
    ? theme?.resources?.find((item) => item.resource_key === background.resource_key)
    : undefined;
  const imageSource = resource ? toMediaSrc(resource.file_path) : "";
  const values: Record<string, string> = {
    "--a4n-theme-bg-image": imageSource ? `url(${JSON.stringify(imageSource)})` : "none",
    "--a4n-theme-bg-opacity": String(clamp(background?.opacity ?? 0, 0, 1)),
    "--a4n-theme-bg-overlay": background?.overlay ?? "#000000",
    "--a4n-theme-bg-overlay-opacity": String(clamp(background?.overlay_opacity ?? 0, 0, 1)),
    "--a4n-theme-bg-blur": `${clamp(background?.blur_px ?? 0, 0, 40)}px`,
    "--a4n-theme-bg-position": background?.position ?? "center",
    "--a4n-theme-bg-size": background?.size ?? "cover",
  };
  for (const [name, value] of Object.entries(values)) root.style.setProperty(name, value);
  root.dataset.themeRoute = runtimeRouteFamily;
}

function applyThemeState(theme: Theme | null, mode: ThemeMode): void {
  runtimeAppliedTheme = theme;
  const root = document.documentElement;
  const vars = theme?.document.tokens ?? {};
  const nextKeys = new Set<string>(Object.keys(vars).filter((key) => themeTokenSet.has(key)));
  for (const key of appliedThemeVariables) {
    if (!nextKeys.has(key)) root.style.removeProperty(key);
  }
  for (const [key, value] of Object.entries(vars)) {
    if (themeTokenSet.has(key)) root.style.setProperty(key, value);
  }
  appliedThemeVariables.clear();
  for (const key of nextKeys) appliedThemeVariables.add(key);
  root.classList.toggle("dark", mode === "dark");
  root.dataset.themeMode = mode;
  applyBackground(theme);
  dispatchThemeApplied();
}

function applyRuntimeTheme(): void {
  const mode = preferredThemeMode(runtimeAppearance);
  runtimeActiveTheme =
    runtimeUserId == null ? null : selectActiveTheme(runtimeThemes, runtimeUserId, mode);
  applyThemeState(runtimeActiveTheme, mode);
}

async function reloadRuntimeTheme(): Promise<void> {
  if (runtimeUserId == null) return;
  try {
    const [themes, preference] = await Promise.all([listThemes(), getThemePreference()]);
    if (preference.user_id !== runtimeUserId) return;
    runtimeThemes = themes;
    runtimeAppearance = preference.appearance;
    writeThemeCache(runtimeUserId, themes, preference.appearance);
    applyRuntimeTheme();
  } catch (error) {
    console.error("Failed to reload the active theme after a cross-tab update", error);
  }
}

export function themeRouteFamilyForPath(pathname: string): ThemeRouteFamily {
  if (pathname === "/") return "home";
  if (/^\/inspiration\/\d+(?:\/|$)/.test(pathname)) return "inspiration";
  if (/^\/novel\/\d+(?:\/|$)/.test(pathname)) return "novel_detail";
  if (/^\/write\/\d+(?:\/|$)/.test(pathname)) return "writing_desk";
  if (/^\/video-project\/\d+\/studio(?:\/|$)/.test(pathname)) return "video_studio";
  if (/^\/video-project\/\d+(?:\/|$)/.test(pathname)) return "video_workspace";
  if (/^\/comic-project\/\d+(?:\/|$)/.test(pathname)) return "comic_workspace";
  if (/^\/galgame-project\/\d+(?:\/|$)/.test(pathname)) return "galgame_workspace";
  if (/^\/settings(?:\/|$)/.test(pathname)) return "settings";
  if (/^\/agents(?:\/|$)/.test(pathname)) return "agents";
  return THEME_GLOBAL_ROUTE_FAMILY;
}

export function setThemeRouteFamily(routeFamily: ThemeRouteFamily): void {
  runtimeRouteFamily = routeFamily;
  applyBackground(runtimeAppliedTheme);
  dispatchThemeApplied();
}

export function getActiveThemeStreamDisplaySettings(): ThemeStreamDisplaySettings {
  return normalizeStreamDisplaySettings(runtimeAppliedTheme?.document.stream_display);
}

export function applyActiveTheme(theme: Theme): void {
  runtimeActiveTheme = theme;
  runtimeThemes = runtimeThemes.some((item) => item.id === theme.id)
    ? runtimeThemes.map((item) =>
        item.id === theme.id
          ? theme
          : item.user_id === theme.user_id && item.mode === theme.mode
            ? { ...item, is_active: false }
            : item,
      )
    : [...runtimeThemes, theme];
  if (preferredThemeMode(runtimeAppearance) === theme.mode) {
    applyThemeState(theme, theme.mode);
  } else {
    applyRuntimeTheme();
  }
}

export function applyThemePreview(theme: Theme): void {
  const mode = preferredThemeMode(runtimeAppearance);
  if (theme.mode !== mode) {
    applyRuntimeTheme();
    return;
  }
  applyThemeState(theme, mode);
}

export function restoreActiveTheme(): void {
  applyRuntimeTheme();
}

export function setThemeAppearance(appearance: ThemeAppearance): void {
  runtimeAppearance = appearance;
  applyRuntimeTheme();
}

export async function persistThemeAppearance(
  appearance: ThemeAppearance,
  canApply: () => boolean = () => true,
): Promise<ThemePreferenceSummary> {
  const result = await updateThemePreference({ appearance });
  if (canApply()) {
    setThemeAppearance(result.appearance);
    notifyThemeChanged();
  }
  return result;
}

export function notifyThemeChanged(): void {
  broadcastThemeSync();
}

export async function loadAndApplyActiveTheme(userId: number | null): Promise<void> {
  ensureSyncListeners();
  runtimeUserId = userId;
  runtimeThemes = [];
  runtimeActiveTheme = null;
  if (userId == null) {
    runtimeAppearance = "system";
    applyThemeState(null, "light");
    return;
  }

  const cached = readThemeCache(userId);
  if (cached) {
    runtimeThemes = cached.themes;
    runtimeAppearance = cached.appearance;
    applyRuntimeTheme();
  } else {
    applyThemeState(null, preferredThemeMode(runtimeAppearance));
  }
  const [themes, preference] = await Promise.all([listThemes(), getThemePreference()]);
  if (runtimeUserId !== userId || preference.user_id !== userId) return;
  runtimeThemes = themes;
  runtimeAppearance = preference.appearance;
  writeThemeCache(userId, themes, preference.appearance);
  applyRuntimeTheme();
}
