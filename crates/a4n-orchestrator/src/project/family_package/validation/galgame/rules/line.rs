use a4n_core::models::galgame::galgame_line;

use crate::galgame::galgame_line_from_model;

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
    galgame_line_from_model(line.clone())
        .map(|_| ())
        .map_err(|error| format!("项目族导入包 {error}"))
}
