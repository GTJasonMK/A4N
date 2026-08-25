import { type ThemeMode } from "../../../api/theme";
import { THEME_COLOR_TOKEN_NAMES } from "../../../theme/themeTokens";

export function readThemePreset(mode: ThemeMode): Record<string, string> {
  const root = document.documentElement;
  const wasDark = root.classList.contains("dark");
  const inlineValues = THEME_COLOR_TOKEN_NAMES.map((name) => ({
    name,
    value: root.style.getPropertyValue(name),
    priority: root.style.getPropertyPriority(name),
  }));

  try {
    for (const { name } of inlineValues) root.style.removeProperty(name);
    root.classList.toggle("dark", mode === "dark");
    const styles = getComputedStyle(root);
    return Object.fromEntries(
      THEME_COLOR_TOKEN_NAMES.map((name) => {
        const value = styles.getPropertyValue(name).trim();
        if (!value) throw new Error(`缺少内置主题变量：${name}`);
        return [name, value];
      }),
    );
  } finally {
    root.classList.toggle("dark", wasDark);
    for (const { name, value, priority } of inlineValues) {
      if (value) root.style.setProperty(name, value, priority);
      else root.style.removeProperty(name);
    }
  }
}
