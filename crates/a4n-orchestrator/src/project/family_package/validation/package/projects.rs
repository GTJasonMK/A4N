use super::super::super::{PROJECT_FAMILY_PACKAGE_VERSION, ProjectFamilyPackage};
use crate::project::{
    PROJECT_KIND_TEXT, SOURCE_SYNC_CURRENT, is_valid_source_sync_status,
    normalize_derived_project_kind, normalize_project_kind, normalize_project_type,
};
use a4n_domain::project::{ProjectStatus, required_text};
use std::collections::{HashMap, HashSet};

pub(super) fn validate_package_projects(package: &ProjectFamilyPackage) -> Result<(), String> {
    if package.version != PROJECT_FAMILY_PACKAGE_VERSION {
        return Err(format!("项目族导入包版本无效: {}", package.version));
    }
    if package.mode != "project_family" && package.mode != "derived_project" {
        return Err(format!("项目族导入包模式无效: {}", package.mode));
    }
    let root = package
        .projects
        .iter()
        .find(|project| project.id == package.root_project_id)
        .ok_or_else(|| format!("项目族导入包缺少根文本项目: {}", package.root_project_id))?;
    if root.project_kind != PROJECT_KIND_TEXT {
        return Err("项目族导入包根项目必须是 text".to_string());
    }
    if root.root_project_id.is_some() || root.source_snapshot_id.is_some() {
        return Err("项目族导入包根文本项目不能包含派生项目引用字段".to_string());
    }
    let snapshots_by_id = package
        .source_snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot))
        .collect::<HashMap<_, _>>();
    let mut project_ids = HashSet::new();
    for project in &package.projects {
        if !project_ids.insert(project.id) {
            return Err(format!("项目族导入包项目 ID 重复: {}", project.id));
        }
        required_text(&project.title, "项目族导入包项目标题")?;
        normalize_project_type(&project.project_type).map_err(|err| err.to_string())?;
        normalize_project_kind(&project.project_kind).map_err(|err| err.to_string())?;
        ProjectStatus::parse_storage(&project.status).map_err(|err| err.to_string())?;
        if !is_valid_source_sync_status(&project.source_sync_status) {
            return Err(format!(
                "项目族导入包项目源同步状态无效: project_id={}, source_sync_status={}",
                project.id, project.source_sync_status
            ));
        }
        if project.project_kind == PROJECT_KIND_TEXT && project.id != package.root_project_id {
            return Err(format!("项目族导入包包含非根文本项目: {}", project.id));
        }
        if project.project_kind == PROJECT_KIND_TEXT
            && project.source_sync_status != SOURCE_SYNC_CURRENT
        {
            return Err(format!(
                "项目族导入包文本项目源同步状态必须是 current: project_id={}, source_sync_status={}",
                project.id, project.source_sync_status
            ));
        }
        if project.project_kind != PROJECT_KIND_TEXT {
            normalize_derived_project_kind(&project.project_kind).map_err(|err| err.to_string())?;
            if project.root_project_id != Some(package.root_project_id) {
                return Err(format!("派生项目不属于导入包根项目: {}", project.id));
            }
            let snapshot_id = project
                .source_snapshot_id
                .ok_or_else(|| format!("派生项目缺少 source_snapshot_id: {}", project.id))?;
            let snapshot = snapshots_by_id.get(&snapshot_id).ok_or_else(|| {
                format!(
                    "派生项目引用了导入包不存在的源文本快照: project_id={}, snapshot_id={snapshot_id}",
                    project.id
                )
            })?;
            if snapshot.root_project_id != package.root_project_id {
                return Err(format!(
                    "派生项目源文本快照不属于导入包根项目: project_id={}, snapshot_id={snapshot_id}",
                    project.id
                ));
            }
        }
    }
    Ok(())
}
