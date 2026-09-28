use super::super::*;

pub(super) fn validate_galgame_scripts(
    package: &ProjectFamilyPackage,
    projects: &HashMap<i64, &novel_project::Model>,
    snapshots: &HashMap<i64, &project_source_snapshot::Model>,
) -> Result<(), String> {
    for script in &package.galgame_scripts {
        if let Some(id) = script.branch_plan_id {
            let plan = package
                .authoring_documents
                .iter()
                .find(|row| row.id == id)
                .ok_or("游戏引用的分支计划不存在")?;
            if plan.project_id != script.project_id
                || plan.kind != "galgame_branch_plan"
                || plan.source_snapshot_id != script.source_snapshot_id
                || !matches!(plan.status.as_str(), "adopted" | "superseded")
            {
                return Err("游戏分支计划的归属或版本无效".into());
            }
        }
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
        if project.root_project_id != Some(script.root_project_id) {
            return Err(format!(
                "项目族导入包 Galgame 剧本根项目与项目不一致: script_id={}",
                script.id
            ));
        }
        if !snapshots
            .get(&script.source_snapshot_id)
            .is_some_and(|snapshot| snapshot.root_project_id == script.root_project_id)
        {
            return Err(format!(
                "项目族导入包 Galgame 剧本初始源快照不存在或不属于根项目: script_id={}, source_snapshot_id={}",
                script.id, script.source_snapshot_id
            ));
        }
    }
    Ok(())
}
