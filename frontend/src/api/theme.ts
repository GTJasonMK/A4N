import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";

/**
 * Theme API wrapper —— 对齐 crates/a4n-tauri/src/commands/theme.rs
 *
 * 关键约定：
 *  - theme 由当前登录用户、name、mode、css_variables 描述，不是 css_variables only
 *  - mode: "light" / "dark" 等
 *  - update_theme(theme_id, input: { name?, mode?, css_variables? }) 平铺 + input
 *  - delete_theme / activate_theme / export_theme / get_theme_css_variables 用 theme_id
 *  - create_theme / import_theme 的主题 owner 来自当前登录用户
 *  - list_themes(user_id) 返回当前用户可见主题：当前用户主题 + 全局主题
 *  - list_user_themes(user_id) 只返回当前用户主题
 *  - import_all_settings(input: { settings_json })
 */

export interface Theme {
  id: number;
  user_id: number | null;
  name: string;
  mode: string;
  css_variables: unknown;
  is_system_default: boolean;
  is_active: boolean;
  created_at: string | null;
  updated_at: string | null;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问主题");
  return id;
};

// ============================================================================
// 主题查询
// ============================================================================

export async function listThemes(userId?: Id | null): Promise<Theme[]> {
  const visibleUserId = userId == null ? currentUserId() : idNum(userId);
  return apiInvoke<Theme[]>("list_themes", { userId: visibleUserId });
}

/** 后端 list_user_themes(user_id) —— 必须传 user_id */
export async function listUserThemes(userId: Id): Promise<Theme[]> {
  return apiInvoke<Theme[]>("list_user_themes", { userId: idNum(userId) });
}

export async function getSystemDefaultTheme(): Promise<Theme | null> {
  return apiInvoke<Theme | null>("get_system_default_theme");
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
export async function createTheme(input: {
  name: string;
  mode: string;
  css_variables: unknown;
}): Promise<Theme> {
  return apiInvoke<Theme>("create_theme", { userId: currentUserId(), input });
}

/** 后端 update_theme(theme_id, input: { name?, mode?, css_variables? }) —— 平铺 + input */
export async function updateTheme(
  themeId: Id,
  input: {
    name?: string | null;
    mode?: string | null;
    css_variables?: unknown | null;
  },
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

/** 后端 import_theme(input: { theme_json }) */
export async function importTheme(themeJson: string): Promise<Theme> {
  return apiInvoke<Theme>("import_theme", {
    userId: currentUserId(),
    input: {
      theme_json: themeJson,
    },
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
  return apiInvoke<string>("export_all_configs_json");
}

/** 后端 import_all_configs(input: { settings_json, overwrite? }) */
export async function importAllConfigs(
  settingsJson: string,
  overwrite: boolean = false,
): Promise<ImportAllConfigsResult> {
  return apiInvoke<ImportAllConfigsResult>("import_all_configs", {
    input: { settings_json: settingsJson, overwrite },
  });
}
