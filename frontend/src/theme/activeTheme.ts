import { listThemes, type Theme, type ThemeMode } from "../api/theme";
import { readCurrentAccessToken } from "../api/authStorage";
import { shouldUseHttpBackend } from "../utils/runtimeMode";

/**
 * 后端主题（theme_config 表）应用到 DOM 的工具
 *
 * 这里也是 App 启动时加载已激活主题的统一入口；CustomEvent 用于通知画布等
 * 非 React 主题消费者重新读取 CSS 变量。
 */
export const THEME_APPLIED_EVENT = "a4n:theme-applied";

const appliedThemeVariables = new Set<string>();

interface ThemeAppliedDetail {
  mode: ThemeMode | null;
}

function dispatchThemeApplied(detail: ThemeAppliedDetail): void {
  window.dispatchEvent(
    new CustomEvent(THEME_APPLIED_EVENT, {
      detail,
    }),
  );
}

function clearAppliedThemeVariables(): void {
  const root = document.documentElement;
  for (const key of appliedThemeVariables) root.style.removeProperty(key);
  appliedThemeVariables.clear();
  root.classList.remove("dark");

  dispatchThemeApplied({ mode: null });
}

/**
 * 把后端 Theme 的 css_variables 写入 :root.style，并按 mode 切换 .dark 类。
 */
export function applyActiveTheme(theme: Theme): void {
  const root = document.documentElement;

  const vars = theme.css_variables;
  const nextKeys = new Set(Object.keys(vars));
  for (const key of appliedThemeVariables) {
    if (!nextKeys.has(key)) root.style.removeProperty(key);
  }

  for (const [key, value] of Object.entries(vars)) {
    root.style.setProperty(key, value);
  }
  appliedThemeVariables.clear();
  for (const key of nextKeys) appliedThemeVariables.add(key);

  root.classList.toggle("dark", theme.mode === "dark");

  dispatchThemeApplied({ mode: theme.mode });
}

function selectActiveTheme(
  themes: Theme[],
  userId?: number | null,
): Theme | null {
  if (userId != null) {
    const userTheme = themes.find((t) => t.is_active && t.user_id === userId);
    if (userTheme) return userTheme;
  }
  return themes.find((t) => t.is_active && t.user_id == null) ?? null;
}

/** 应用启动时调用：拉取主题列表，优先应用当前用户的 active 主题，否则应用全局 active 主题。 */
export async function loadAndApplyActiveTheme(
  userId?: number | null,
): Promise<void> {
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
  }
}
