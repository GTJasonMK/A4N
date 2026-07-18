use super::super::*;

pub(super) fn validate_galgame_scripts(
    package: &ProjectFamilyPackage,
    projects: &HashMap<i64, &novel_project::Model>,
    snapshots: &HashMap<i64, &project_source_snapshot::Model>,
) -> Result<(), String> {
    for script in &package.galgame_scripts {
        let project = projects.get(&script.project_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 剧本引用了不存在的项目: script_id={}, project_id={}",
                script.id, script.project_id
            )
        })?;
        if project.project_kind != "galgame" {
            return Err(format!(
                "项目族导入包 Galgame 剧本项目类型无效: script_id={}, project_kind={}",
                script.id, project.project_kind
            ));
        }
        if script.root_project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包 Galgame 剧本 root_project_id 无效: script_id={}",
                script.id
            ));
        }
        if project.source_snapshot_id != Some(script.source_snapshot_id) {
            return Err(format!(
                "项目族导入包 Galgame 剧本源快照与项目不一致: script_id={}, source_snapshot_id={}",
                script.id, script.source_snapshot_id
            ));
        }
        if !snapshots.contains_key(&script.source_snapshot_id) {
            return Err(format!(
                "项目族导入包 Galgame 剧本引用了不存在的源快照: script_id={}, source_snapshot_id={}",
                script.id, script.source_snapshot_id
            ));
        }
    }
    Ok(())
}
