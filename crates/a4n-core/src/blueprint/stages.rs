use serde_json::{Value, json};

use crate::error::AppError;

pub fn blueprint_stage_previous_context(
    stage_index: usize,
    outputs: &[Value],
) -> Result<Option<Value>, AppError> {
    let required = |index: usize, message: &str| {
        outputs
            .get(index)
            .ok_or_else(|| AppError::InvalidArgument(message.to_string()))
    };
    Ok(match stage_index {
        0 => None,
        1 => Some(json!({
            "story_core": required(0, "世界观阶段缺少故事核心上下文")?,
        })),
        2 => Some(json!({
            "story_core": required(0, "角色阶段缺少故事核心上下文")?,
            "world": required(1, "角色阶段缺少世界观上下文")?,
        })),
        3 => Some(json!({
            "story_core": required(0, "关系阶段缺少故事核心上下文")?,
            "world": required(1, "关系阶段缺少世界观上下文")?,
            "characters": required(2, "关系阶段缺少角色上下文")?,
        })),
        _ => {
            return Err(AppError::InvalidArgument(format!(
                "未知蓝图生成阶段索引: {stage_index}"
            )));
        }
    })
}
