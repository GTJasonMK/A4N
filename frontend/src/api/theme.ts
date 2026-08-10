import { apiInvoke } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";
import type {
  CreateThemeInput,
  Theme,
  UpdateThemeInput,
} from "./generated/theme";

export type { CreateThemeInput, Theme, ThemeMode, UpdateThemeInput } from "./generated/theme";

/**
 * Theme API wrapper —— 对齐 crates/a4n-tauri/src/commands/theme.rs
 *
 * 关键约定：
 *  - theme 由当前登录用户、name、mode、css_variables 描述，不是 css_variables only
 *  - mode 仅允许 "light" / "dark"
 *  - update_theme(theme_id, input: { name?, mode?, css_variables? }) 平铺 + input
 *  - delete_theme / activate_theme / export_theme / get_theme_css_variables 用 theme_id
 *  - create_theme / import_theme 的主题 owner 来自当前登录用户
 *  - list_themes(user_id) 返回当前用户可见主题：当前用户主题 + 全局主题
 *  - import_all_configs(input: { settings_json, overwrite })
 */

// ============================================================================
// 主题查询
// ============================================================================

export async function listThemes(userId: Id): Promise<Theme[]> {
  return apiInvoke<Theme[]>("list_themes", { userId: idNum(userId) });
}

/** 后端 get_theme_css_variables(theme_id) —— 返回 CSS 文本 */
export async function getThemeCssVariables(themeId: Id): Promise<string> {
  return apiInvoke<string>("get_theme_css_variables", {
    userId: currentUserId(),
    themeId: idNum(themeId),
  });
}

// ============================================================================
// 主题创建与更新
// ============================================================================

/** 后端 create_theme(input: { name, mode, css_variables }) */
export async function createTheme(input: CreateThemeInput): Promise<Theme> {
  return apiInvoke<Theme>("create_theme", { userId: currentUserId(), input });
}

/** 后端 update_theme(theme_id, input: { name?, mode?, css_variables? }) —— 平铺 + input */
export async function updateTheme(
  themeId: Id,
  input: UpdateThemeInput,
): Promise<Theme> {
  return apiInvoke<Theme>("update_theme", {
    userId: currentUserId(),
    themeId: idNum(themeId),
    input,
  });
}

/** 后端 delete_theme(theme_id) */
export async function deleteTheme(themeId: Id): Promise<void> {
  return apiInvoke<void>("delete_theme", { userId: currentUserId(), themeId: idNum(themeId) });
}

/** 后端 activate_theme(theme_id) —— 同用户其他主题自动取消激活 */
export async function activateTheme(themeId: Id): Promise<Theme> {
  return apiInvoke<Theme>("activate_theme", { userId: currentUserId(), themeId: idNum(themeId) });
}

// ============================================================================
// 导入导出
// ============================================================================

/** 后端 export_theme(theme_id) —— 返回 JSON 字符串 */
export async function exportTheme(themeId: Id): Promise<string> {
  return apiInvoke<string>("export_theme", { userId: currentUserId(), themeId: idNum(themeId) });
}

/** 后端 import_theme(input: CreateThemeInput) */
export async function importTheme(themeJson: string): Promise<Theme> {
  const input = JSON.parse(themeJson) as unknown;
  return apiInvoke<Theme>("import_theme", {
    userId: currentUserId(),
    input,
  });
}

// ============================================================================
// 全量配置一键导入导出
// 覆盖：LLM/Embedding/Image/Video/Prompts(已修改)/Themes/SystemSettings
// ============================================================================

interface ImportAllConfigsResult {
  success: boolean;
  message: string;
  details: string[];
}

export async function exportAllConfigsJson(): Promise<string> {
  return apiInvoke<string>("export_all_configs_json", {
    callerUserId: currentUserId(),
  });
}

/** 后端 import_all_configs(input: { settings_json, overwrite }) */
export async function importAllConfigs(
  settingsJson: string,
  overwrite: boolean,
): Promise<ImportAllConfigsResult> {
  return apiInvoke<ImportAllConfigsResult>("import_all_configs", {
    callerUserId: currentUserId(),
    input: { settings_json: settingsJson, overwrite },
  });
}
