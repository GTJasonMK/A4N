import { listThemes, type Theme } from "../api/theme";
import { readCurrentAccessToken } from "../api/authStorage";
import { writePortableItem } from "../utils/portableStorage";
import { shouldUseHttpBackend } from "../utils/runtimeMode";
import { themeCssVariableStringMap } from "./themeCssVariables";

/**
 * 后端主题（theme_config 表）应用到 DOM 的工具
 *
 * 这里也是 App 启动时加载已激活主题的统一入口；用 CustomEvent 通知 Layout
 * 同步暗色/亮色状态（Layout 内的 isDark 由便携前端状态持久化）。
 */
export const THEME_APPLIED_EVENT = "a4n:theme-applied";
export const THEME_MODE_STORAGE_KEY = "a4n-theme-mode";

const appliedThemeVariables = new Set<string>();

function dispatchThemeApplied(detail: {
  mode: string | null;
  themeId: number | null;
  themeName: string | null;
}): void {
  window.dispatchEvent(
    new CustomEvent(THEME_APPLIED_EVENT, {
      detail,
    }),
  );
}

function clearAppliedThemeVariables(): void {
  if (appliedThemeVariables.size === 0) return;

  const root = document.documentElement;
  for (const key of appliedThemeVariables) root.style.removeProperty(key);
  appliedThemeVariables.clear();

  dispatchThemeApplied({
    mode: null,
    themeId: null,
    themeName: null,
  });
}

/**
 * 把后端 Theme 的 css_variables 写入 :root.style，并按 mode 切换 .dark 类。
 * dispatch THEME_APPLIED_EVENT，让 App.tsx 的 Layout 同步 isDark。
 */
export function applyActiveTheme(theme: Theme): void {
  const root = document.documentElement;

  const vars = themeCssVariableStringMap(theme.css_variables);
  const nextKeys = new Set(Object.keys(vars));
  for (const key of appliedThemeVariables) {
    if (!nextKeys.has(key)) root.style.removeProperty(key);
  }

  for (const [key, value] of Object.entries(vars)) {
    root.style.setProperty(key, value);
  }
  appliedThemeVariables.clear();
  for (const key of nextKeys) appliedThemeVariables.add(key);

  const isDark = theme.mode === "dark";
  root.classList.toggle("dark", isDark);

  writePortableItem(THEME_MODE_STORAGE_KEY, isDark ? "dark" : "light");

  dispatchThemeApplied({
    mode: theme.mode,
    themeId: theme.id,
    themeName: theme.name,
  });
}

function selectActiveTheme(themes: Theme[], userId?: number | null): Theme | null {
  if (userId != null) {
    const userTheme = themes.find((t) => t.is_active && t.user_id === userId);
    if (userTheme) return userTheme;
  }
  return themes.find((t) => t.is_active && t.user_id == null) ?? null;
}

/** 应用启动时调用：拉取主题列表，优先应用当前用户的 active 主题，否则应用全局 active 主题。 */
export async function loadAndApplyActiveTheme(userId?: number | null): Promise<void> {
  if (shouldUseHttpBackend() && !readCurrentAccessToken()) {
    clearAppliedThemeVariables();
    return;
  }
  if (userId == null) {
    clearAppliedThemeVariables();
    return;
  }

  clearAppliedThemeVariables();
  const themes = await listThemes(userId);
  const active = selectActiveTheme(themes, userId);
  if (active) {
    applyActiveTheme(active);
  } else {
    clearAppliedThemeVariables();
  }
}
