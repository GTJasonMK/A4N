import {
  THEME_DOCUMENT_VERSION,
  THEME_GLOBAL_ROUTE_FAMILY,
  THEME_ROUTE_FAMILIES,
  THEME_TOKEN_NAMES,
  type ThemeRouteFamily,
} from "../api/generated/theme";

export {
  THEME_DOCUMENT_VERSION,
  THEME_GLOBAL_ROUTE_FAMILY,
  THEME_ROUTE_FAMILIES,
  THEME_TOKEN_NAMES,
};
export type { ThemeRouteFamily };

export const THEME_COLOR_TOKEN_NAMES = THEME_TOKEN_NAMES.filter(
  (name) => name.startsWith("--color-") || name.startsWith("--status-"),
);

export const THEME_LAYER_OPACITY_TOKEN_NAMES = THEME_TOKEN_NAMES.filter(
  (name) => name.startsWith("--a4n-") && name.endsWith("-opacity"),
);

const THEME_CONTROLLED_TOKEN_NAMES = new Set([
  "--a4n-surface-blur",
  "--a4n-shadow-strength",
  ...THEME_LAYER_OPACITY_TOKEN_NAMES,
  ...THEME_TOKEN_NAMES.filter((name) => name.startsWith("--pyqt-radius-")),
]);

export const THEME_ADVANCED_TOKEN_NAMES = THEME_TOKEN_NAMES.filter(
  (name) => !THEME_COLOR_TOKEN_NAMES.includes(name) && !THEME_CONTROLLED_TOKEN_NAMES.has(name),
);
