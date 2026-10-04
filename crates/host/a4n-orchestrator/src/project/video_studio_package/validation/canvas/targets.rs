use std::collections::HashSet;

use a4n_domain::generation::GenerationSpecRecord;

pub(in crate::project::video_studio_package::validation) fn validate_generation_spec_target(
    spec: &GenerationSpecRecord,
    node_ids: &HashSet<i64>,
    media_asset_ids: &HashSet<i64>,
    video_segment_ids: &HashSet<i64>,
    assembly_timeline_ids: &HashSet<i64>,
) -> Result<(), String> {
    let Some(target_id) = spec.target_entity_id else {
        return Ok(());
    };
    if target_id <= 0 {
        return Err(format!(
            "生成意图 target_entity_id 必须是正整数: generation_spec_id={}",
            spec.id
        ));
    }
    let (allowed_ids, label) = match spec.target_entity_kind.as_str() {
        "canvas_node" => (node_ids, "画布节点"),
        "media_asset" => (media_asset_ids, "媒体资产"),
        "video_segment" => (video_segment_ids, "视频片段"),
        "assembly_timeline" => (assembly_timeline_ids, "成片时间线"),
        other => {
            return Err(format!(
                "视频画布项目包暂不支持生成意图目标类型: generation_spec_id={}, target_entity_kind={other}",
                spec.id
            ));
        }
    };
    if !allowed_ids.contains(&target_id) {
        return Err(format!(
            "生成意图引用了不存在的{label}: generation_spec_id={}, target_entity_id={target_id}",
            spec.id
        ));
    }
    Ok(())
}
