import { type ThemeMode } from "../../../api/theme";

const THEME_VARIABLE_NAMES = [
  "--color-primary",
  "--color-primary-light",
  "--color-primary-dark",
  "--color-primary-pale",
  "--color-accent",
  "--color-accent-light",
  "--color-accent-dark",
  "--color-accent-pale",
  "--color-text-primary",
  "--color-text-secondary",
  "--color-text-tertiary",
  "--color-text-placeholder",
  "--color-text-disabled",
  "--color-bg-primary",
  "--color-bg-secondary",
  "--color-bg-tertiary",
  "--color-bg-card-hover",
  "--color-bg-muted",
  "--color-bg-accent",
  "--color-border",
  "--color-border-light",
  "--color-border-dark",
  "--status-success",
  "--status-success-bg",
  "--status-error",
  "--status-error-bg",
  "--status-warning",
  "--status-warning-bg",
  "--status-info",
  "--status-info-bg",
  "--color-button-text",
  "--color-button-text-secondary",
] as const;

export function readThemePreset(mode: ThemeMode): Record<string, string> {
  const root = document.documentElement;
  const wasDark = root.classList.contains("dark");
  const inlineValues = THEME_VARIABLE_NAMES.map((name) => ({
    name,
    value: root.style.getPropertyValue(name),
    priority: root.style.getPropertyPriority(name),
  }));

  try {
    for (const { name } of inlineValues) root.style.removeProperty(name);
    root.classList.toggle("dark", mode === "dark");
    const styles = getComputedStyle(root);
    return Object.fromEntries(
      THEME_VARIABLE_NAMES.map((name) => {
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
