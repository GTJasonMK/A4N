import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";

/**
 * Blueprint API wrapper —— 对齐 crates/a4n-tauri/src/commands/blueprint.rs
 *
 * 命令组：
 *  - get/list_*: 用 projectId 平铺参数
 *  - generate_blueprint / refine_blueprint: 用 input 包装
 */

// ============================================================================
// 类型（对齐后端 *Summary）
// ============================================================================

export interface Blueprint {
  id: number;
  project_id: number;
  title: string;
  genre: string;
  full_synopsis: string | null;
  style: string | null;
  tone: string | null;
  target_audience: string | null;
  one_sentence_summary: string | null;
  world_setting?: unknown;
  current_version: string;
  needs_part_outlines: boolean;
  total_chapters: number | null;
  chapters_per_part: number | null;
  avatar_svg: string | null;
  avatar_animal: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface BlueprintVersion {
  id: number;
  project_id: number;
  version: string;
  change_description: string | null;
  created_at: string;
  created_by: string | null;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问蓝图");
  return id;
};

// ============================================================================
// 命令封装
// ============================================================================

export async function getBlueprint(projectId: Id): Promise<Blueprint | null> {
  return apiInvoke<Blueprint | null>("get_blueprint", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

/** 后端 generate_blueprint(input: { project_id, title, genre, target_length, variables?, task_id? }) */
export async function generateBlueprint(input: {
  project_id: number;
  title: string;
  genre: string;
  target_length: number;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<Blueprint> {
  return apiInvoke<Blueprint>("generate_blueprint", { userId: currentUserId(), input });
}

export async function previewGenerateBlueprintRequest(input: {
  project_id: number;
  title: string;
  genre: string;
  target_length: number;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_blueprint_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 refine_blueprint(input: { project_id, focus?, variables? }) */
export async function refineBlueprint(input: {
  project_id: number;
  focus?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<Blueprint> {
  return apiInvoke<Blueprint>("refine_blueprint", { userId: currentUserId(), input });
}

export async function previewRefineBlueprintRequest(input: {
  project_id: number;
  focus?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_refine_blueprint_request", {
    userId: currentUserId(),
    input,
  });
}

export async function listBlueprintVersions(projectId: Id): Promise<BlueprintVersion[]> {
  return apiInvoke<BlueprintVersion[]>("list_blueprint_versions", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

/** 后端 diff_blueprint_versions(input: { project_id, version_a, version_b }) */
export async function diffBlueprintVersions(input: {
  project_id: number;
  version_a: string;
  version_b: string;
}): Promise<unknown> {
  return apiInvoke<unknown>("diff_blueprint_versions", { userId: currentUserId(), input });
}

/** 后端 restore_blueprint_version(input: { project_id, version }) */
export async function restoreBlueprintVersion(input: {
  project_id: number;
  version: string;
}): Promise<Blueprint> {
  return apiInvoke<Blueprint>("restore_blueprint_version", { userId: currentUserId(), input });
}

/** 后端 update_world_setting(input: { project_id, world_setting }) */
export async function updateWorldSetting(input: {
  project_id: number;
  world_setting: unknown;
}): Promise<unknown> {
  return apiInvoke<unknown>("update_world_setting", { userId: currentUserId(), input });
}
