use crate::state::ServerState;
use a4n_core::event::channel::TaskType;
use serde_json::{Value, json};

use super::super::super::{
    ApiError, finish_web_task, optional_string_field, portrait_generation_actions,
    register_web_task, sanitize_optional_task_id, string_action_result, web_progress_reporter,
};
use super::input::{
    age_variant_input, generate_portrait_input, missing_portraits_input, regenerate_portrait_input,
};
use super::task::portrait_web_generation_command;

portrait_web_generation_command!(
    generate_portrait,
    |args| generate_portrait_input(args, "generate_portrait")?,
    portrait_generation_actions::generate_portrait_task_scope,
    "正在准备人物三视图拆解生成",
    "人物三视图拆解生成完成",
    portrait_generation_actions::generate_portrait,
);

portrait_web_generation_command!(
    regenerate_portrait,
    |args| regenerate_portrait_input(args, "regenerate_portrait")?,
    portrait_generation_actions::regenerate_portrait_task_scope,
    "正在准备重绘人物三视图拆解",
    "人物三视图拆解重绘完成",
    portrait_generation_actions::regenerate_portrait,
);

portrait_web_generation_command!(
    generate_age_variant,
    |args| age_variant_input(args, "generate_age_variant")?,
    portrait_generation_actions::generate_age_variant_task_scope,
    "正在准备年龄变体",
    "年龄变体生成完成",
    portrait_generation_actions::generate_age_variant,
);

portrait_web_generation_command!(
    generate_missing_portraits,
    |args| missing_portraits_input(args)?,
    portrait_generation_actions::generate_missing_portraits_task_scope,
    "准备补齐人物三视图拆解",
    "人物三视图拆解补齐完成",
    portrait_generation_actions::generate_missing_portraits,
);
