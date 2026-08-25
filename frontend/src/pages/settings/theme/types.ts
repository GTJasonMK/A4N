import type { Theme } from "../../../api/theme";

export type ThemeScope = "all" | "user";

export const isDeletableTheme = (
  theme: Pick<Theme, "user_id" | "is_active" | "is_system_default">,
): boolean => theme.user_id != null && !theme.is_active && !theme.is_system_default;
