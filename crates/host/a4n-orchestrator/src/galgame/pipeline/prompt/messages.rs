use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use super::{super::super::GalgameSpeakerProfile, contracts::GalgamePipelinePromptChapter};

const JSON_RESPONSE_REQUIREMENT: &str =
    "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段";

pub fn galgame_script_adaptation_user_message(
    chapters: &[GalgamePipelinePromptChapter],
    shooting_script_projection: &Value,
    branch_design: &Value,
) -> Result<String, AppError> {
    build_galgame_script_adaptation_user_message(
        chapters,
        shooting_script_projection.clone(),
        branch_design,
    )
}

pub(in crate::galgame) fn galgame_script_adaptation_session_user_message(
    chapters: &[GalgamePipelinePromptChapter],
    branch_design: &Value,
) -> Result<String, AppError> {
    build_galgame_script_adaptation_user_message(
        chapters,
        shared_shooting_script_projection_ref(),
        branch_design,
    )
}

fn build_galgame_script_adaptation_user_message(
    chapters: &[GalgamePipelinePromptChapter],
    shooting_script_projection: Value,
    branch_design: &Value,
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_script_adaptation",
        "把共享导演场景、节拍和分支设计改编为主线及真实支线剧本行",
        json!({
            "chapters": chapters,
            "previous_steps": {
                "shooting_script_projection": shooting_script_projection,
                "branch_design": branch_design,
            }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["scene_lines", "branch_scene_lines"],
            "line_types": ["dialogue", "narration", "monologue", "stage_direction"],
            "beat_ref_rule": "每个主线和支线行必须携带 beat_ref，并引用 shooting_script_projection.scenes[].beats[] 中存在的 scene_index + beat_index；同一导演节拍可改编为多行，但不能倒序",
            "branch_rule": "branch_scene_lines 必须与 branch_design.options[].branch_id 一一对应并输出真实可演出的多行支线；禁止把 branch_objective 或 dramatic_action 直接压成单条摘要旁白",
            "source_span_rule": "每个 lines[].source_span_ids 必须从 input.chapters[].source_spans[].id 中选择，至少 1 个，不要输出空数组或自造 ID",
            "index_rule": "scene_lines[].lines[].index 从 1 开始连续递增"
        }),
    )
}

pub fn galgame_character_staging_user_message(
    script_adaptation: &Value,
    speaker_profiles: &[GalgameSpeakerProfile],
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_character_staging",
        "绑定剧本行说话人，并输出唯一权威的逐行角色演出状态",
        json!({
            "previous_step": { "script_adaptation": script_adaptation },
            "speaker_profiles": speaker_profiles,
        }),
        json!({
            "format": "json_object",
            "required_fields": ["scene_lines", "branch_scene_lines"],
            "branch_rule": "必须完整保留 script_adaptation.branch_scene_lines、branch_id、beat_ref、文本和 source_span_ids，并为每条支线行输出 presentation",
            "profile_rule": "speaker_profile_id 只能使用 input.speaker_profiles[].id；无法可靠绑定时为 null",
            "presentation_schema": "galgame_line_presentation_v2",
            "presentation_rule": "每行必须输出 presentation；visible_cast 是该行画面完成后的完整在场角色状态；enter 和 focus 必须引用 visible_cast 角色 ID；exit 表示本行完成后已离场角色，必须是不在 visible_cast 中的不重复正整数角色 ID；hold 表示是否保持当前画面节奏；player_knowledge 记录该行后玩家新增或被强化的认知",
            "expression_values": ["neutral", "smile", "sad", "angry", "surprised", "shy", "serious"],
            "position_values": ["left", "center", "right"],
            "facing_values": ["left", "right", "front"],
            "layer_values": ["back", "middle", "front"],
            "source_span_rule": "必须原样保留每行非空 source_span_ids，不要输出空数组或自造 ID"
        }),
    )
}

pub fn galgame_visual_planning_user_message(
    shooting_script_projection: &Value,
    character_staging: &Value,
) -> Result<String, AppError> {
    build_galgame_visual_planning_user_message(
        shooting_script_projection.clone(),
        character_staging,
    )
}

pub(in crate::galgame) fn galgame_visual_planning_session_user_message(
    character_staging: &Value,
) -> Result<String, AppError> {
    build_galgame_visual_planning_user_message(
        shared_shooting_script_projection_ref(),
        character_staging,
    )
}

fn build_galgame_visual_planning_user_message(
    shooting_script_projection: Value,
    character_staging: &Value,
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_visual_planning",
        "为 Galgame 场景规划背景需求，并挑选事件 CG 候选行",
        json!({
            "previous_steps": {
                "shooting_script_projection": shooting_script_projection,
                "character_staging": character_staging,
            }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["backgrounds", "event_cgs"],
            "background_rule": "每个 scene_index 必须有一个 backgrounds 条目",
            "event_cg_rule": "event_cgs[].line_index 必须引用输入剧本行；普通对白不要生成 CG",
            "importance_values": ["normal", "high", "critical"]
        }),
    )
}

pub fn galgame_branch_design_user_message(
    shooting_script_projection: &Value,
) -> Result<String, AppError> {
    build_galgame_branch_design_user_message(shooting_script_projection.clone())
}

pub(in crate::galgame) fn galgame_branch_design_session_user_message() -> Result<String, AppError> {
    build_galgame_branch_design_user_message(shared_shooting_script_projection_ref())
}

fn build_galgame_branch_design_user_message(
    shooting_script_projection: Value,
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_branch_design",
        "基于共享导演节拍为 Galgame 设计珍珠串结构选择支和回收点",
        json!({
            "previous_step": { "shooting_script_projection": shooting_script_projection }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["choices"],
            "choice_rule": "每个选择点使用 after_beat_ref 精确引用 shooting_script_projection.scenes[].beats[]；每个选择点 2 到 3 个选项；branch_id 全局唯一；每个选项定义可见 dramatic_action、角色 reaction 和 player_knowledge，并通过 returns_to_scene_index 回到已有主线场",
            "phase": "可返回空 choices 数组；非空 choices 会持久化为 choice 行、branch 场与 galgame_choice 选项"
        }),
    )
}

fn shared_shooting_script_projection_ref() -> Value {
    json!({"$ref": "system.shared_context.shooting_script_projection"})
}

fn build_galgame_step_user_message(
    prompt_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<String, AppError> {
    build_json_task_user_message(
        "执行 Galgame 管线步骤",
        JSON_RESPONSE_REQUIREMENT,
        &json!({
            "task": { "prompt_name": prompt_name, "instruction": instruction },
            "output_contract": output_contract,
            "input": input,
        }),
    )
}
