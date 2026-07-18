use a4n_core::models::galgame::galgame_line;

use super::presentation::validate_galgame_line_presentation_json;
use super::vocabulary::{
    is_valid_galgame_expression, is_valid_galgame_position, normalized_galgame_expression,
    normalized_galgame_position,
};

pub(in super::super) fn validate_galgame_line_semantics(
    line: &galgame_line::Model,
) -> Result<(), String> {
    if line.line_index <= 0 {
        return Err(format!(
            "项目族导入包 Galgame 行 line_index 必须是正整数: line_id={}, line_index={}",
            line.id, line.line_index
        ));
    }
    if line.text.trim().is_empty() {
        return Err(format!(
            "项目族导入包 Galgame 行 text 不能为空: line_id={}",
            line.id
        ));
    }
    match line.line_type.as_str() {
        "dialogue" | "monologue" if line.speaker_profile_id.is_none() => Err(format!(
            "项目族导入包 Galgame {} 行必须绑定 speaker_profile_id: line_id={}",
            line.line_type, line.id
        )),
        "dialogue" | "monologue" if normalized_galgame_expression(line).is_none() => Err(format!(
            "项目族导入包 Galgame {} 行必须显式提供 expression: line_id={}",
            line.line_type, line.id
        )),
        "dialogue" | "monologue" => Ok(()),
        "narration" | "stage_direction" | "choice" if line.speaker_profile_id.is_some() => {
            Err(format!(
                "项目族导入包 Galgame {} 行不能绑定 speaker_profile_id: line_id={}",
                line.line_type, line.id
            ))
        }
        "narration" | "stage_direction" | "choice" => {
            normalized_galgame_expression(line);
            Ok(())
        }
        other => Err(format!(
            "项目族导入包 Galgame 行类型无效: line_id={}, line_type={other}",
            line.id
        )),
    }?;
    validate_galgame_line_presentation_json(line)?;
    if let Some(expression) = normalized_galgame_expression(line)
        && !is_valid_galgame_expression(expression)
    {
        return Err(format!(
            "项目族导入包 Galgame 行 expression 无效: line_id={}, expression={expression}",
            line.id
        ));
    }
    if let Some(position) = normalized_galgame_position(line)
        && !is_valid_galgame_position(position)
    {
        return Err(format!(
            "项目族导入包 Galgame 行 position 无效: line_id={}, position={position}",
            line.id
        ));
    }
    Ok(())
}
