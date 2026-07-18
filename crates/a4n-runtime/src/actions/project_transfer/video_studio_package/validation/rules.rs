use super::*;

pub(super) fn validate_freeform_video_project(
    project: &novel_project::Model,
    label: &str,
) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_VIDEO || project.source_mode != SOURCE_MODE_FREEFORM {
        return Err(format!("{label}必须是 source_mode=freeform 的 video 项目"));
    }
    if project.root_project_id.is_some()
        || project.parent_project_id.is_some()
        || project.source_snapshot_id.is_some()
        || project.derived_project_status.is_some()
    {
        return Err(format!("{label}不能包含文本派生项目引用字段"));
    }
    Ok(())
}

pub(super) fn validate_unique_ids<I>(ids: I, label: &str) -> Result<(), String>
where
    I: IntoIterator<Item = i64>,
{
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("视频画布项目包 ID 重复: {label}={id}"));
        }
    }
    Ok(())
}

pub(super) fn parse_canvas_cache_key(value: &str) -> Option<(i64, i64)> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() == 4
        && parts[0] == "canvas_graph"
        && parts[2] == "node"
        && let (Ok(graph_id), Ok(node_id)) = (parts[1].parse::<i64>(), parts[3].parse::<i64>())
    {
        return Some((graph_id, node_id));
    }
    None
}
