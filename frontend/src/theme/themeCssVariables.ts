function themeCssVariableName(key: string): string | null {
  const trimmed = key.trim();
  if (!trimmed) return null;
  return trimmed.startsWith("--") ? trimmed : `--${trimmed}`;
}

export function themeCssVariableStringMap(vars: unknown): Record<string, string> {
  if (!vars || typeof vars !== "object" || Array.isArray(vars)) return {};

  const normalized: Record<string, string> = {};
  for (const [key, value] of Object.entries(vars as Record<string, unknown>)) {
    if (typeof value !== "string" || !value.trim()) continue;
    const name = themeCssVariableName(key);
    if (name) normalized[name] = value;
  }
  return normalized;
}

export function getThemeCssVariable(vars: unknown, key: string): string | undefined {
  const name = themeCssVariableName(key);
  if (!name) return undefined;
  return themeCssVariableStringMap(vars)[name];
}
