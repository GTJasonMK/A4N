use crate::galgame::galgame_line_from_record;

pub(in super::super) fn validate_galgame_line_semantics(
    line: &a4n_domain::project::ProjectFamilyGalgameLineRecord,
) -> Result<crate::galgame::GalgameLine, String> {
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
    galgame_line_from_record(a4n_domain::galgame::GalgameLineRecord {
        id: line.id,
        scene_id: line.scene_id,
        line_index: line.line_index,
        line_type: line.line_type.clone(),
        speaker_profile_id: line.speaker_profile_id,
        text: line.text.clone(),
        expression: line.expression.clone(),
        pose: line.pose.clone(),
        position: line.position.clone(),
        presentation_json: line.presentation_json.clone(),
        text_anchor_id: line.text_anchor_id,
    })
    .map_err(|error| format!("项目族导入包 {error}"))
}
