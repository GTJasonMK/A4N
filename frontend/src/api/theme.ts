import { apiInvoke } from "./index";
import { idNum, type ApiId as Id } from "./shared";
import type {
  CreateThemeInput,
  Theme,
  UpdateThemeInput,
} from "./generated/theme";

export type {
  CreateThemeInput,
  Theme,
  ThemeMode,
  UpdateThemeInput,
} from "./generated/theme";

/**
 * Theme API wrapper。
 *
 * 关键约定：
 *  - theme 由当前登录用户、name、mode、css_variables 描述，不是 css_variables only
 *  - mode 仅允许 "light" / "dark"
 *  - update_theme(theme_id, input: { name?, mode?, css_variables? }) 平铺 + input
 *  - delete_theme / activate_theme / export_theme 用 theme_id
 *  - create_theme / import_theme 的主题 owner 来自当前登录用户
 *  - 用户身份只来自 Host 认证上下文，不进入业务参数
 */

// ============================================================================
// 主题查询
// ============================================================================

export async function listThemes(): Promise<Theme[]> {
  return apiInvoke<Theme[]>("list_themes");
}

// ============================================================================
// 主题创建与更新
// ============================================================================

/** 后端 create_theme(input: { name, mode, css_variables }) */
export async function createTheme(input: CreateThemeInput): Promise<Theme> {
  return apiInvoke<Theme>("create_theme", { input });
}

/** 后端 update_theme(theme_id, input: { name?, mode?, css_variables? }) —— 平铺 + input */
export async function updateTheme(
  themeId: Id,
  input: UpdateThemeInput,
): Promise<Theme> {
  return apiInvoke<Theme>("update_theme", {
    themeId: idNum(themeId),
    input,
  });
}

/** 后端 delete_theme(theme_id) */
export async function deleteTheme(themeId: Id): Promise<void> {
  return apiInvoke<void>("delete_theme", {
    themeId: idNum(themeId),
  });
}

/** 后端 activate_theme(theme_id) —— 同用户其他主题自动取消激活 */
export async function activateTheme(themeId: Id): Promise<Theme> {
  return apiInvoke<Theme>("activate_theme", {
    themeId: idNum(themeId),
  });
}

// ============================================================================
// 导入导出
// ============================================================================

/** 后端 export_theme(theme_id) —— 返回 JSON 字符串 */
export async function exportTheme(themeId: Id): Promise<string> {
  return apiInvoke<string>("export_theme", {
    themeId: idNum(themeId),
  });
}

/** 后端 import_theme(input: CreateThemeInput) */
export async function importTheme(themeJson: string): Promise<Theme> {
  const input = JSON.parse(themeJson) as unknown;
  return apiInvoke<Theme>("import_theme", {
    input,
  });
}
