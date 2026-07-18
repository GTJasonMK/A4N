use super::super::super::{PROJECT_FAMILY_PACKAGE_VERSION, ProjectFamilyPackage};
use a4n_core::state::validator::StateValidator;
use a4n_orchestrator::project::{
    PROJECT_KIND_TEXT, SOURCE_SYNC_CURRENT, is_valid_derived_project_status,
    is_valid_source_sync_status, normalize_derived_project_kind, normalize_project_kind,
    normalize_project_type,
};
use std::collections::HashSet;

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
    if root.root_project_id.is_some()
        || root.parent_project_id.is_some()
        || root.source_snapshot_id.is_some()
    {
        return Err("项目族导入包根文本项目不能包含派生项目引用字段".to_string());
    }
    let mut project_ids = HashSet::new();
    for project in &package.projects {
        if !project_ids.insert(project.id) {
            return Err(format!("项目族导入包项目 ID 重复: {}", project.id));
        }
        normalize_project_type(&project.project_type).map_err(|err| err.to_string())?;
        normalize_project_kind(&project.project_kind).map_err(|err| err.to_string())?;
        StateValidator::parse_status(&project.status).map_err(|err| err.to_string())?;
        if !is_valid_source_sync_status(&project.source_sync_status) {
            return Err(format!(
                "项目族导入包项目源同步状态无效: project_id={}, source_sync_status={}",
                project.id, project.source_sync_status
            ));
        }
        if project.project_kind == PROJECT_KIND_TEXT && project.id != package.root_project_id {
            return Err(format!("项目族导入包包含非根文本项目: {}", project.id));
        }
        if project.project_kind == PROJECT_KIND_TEXT {
            if project.source_sync_status != SOURCE_SYNC_CURRENT {
                return Err(format!(
                    "项目族导入包文本项目源同步状态必须是 current: project_id={}, source_sync_status={}",
                    project.id, project.source_sync_status
                ));
            }
            if project.derived_project_status.is_some() {
                return Err(format!(
                    "项目族导入包文本项目不能包含派生项目状态: project_id={}",
                    project.id
                ));
            }
        }
        if project.project_kind != PROJECT_KIND_TEXT {
            normalize_derived_project_kind(&project.project_kind).map_err(|err| err.to_string())?;
            if project.root_project_id != Some(package.root_project_id) {
                return Err(format!("派生项目不属于导入包根项目: {}", project.id));
            }
            if project.parent_project_id != Some(package.root_project_id) {
                return Err(format!("派生项目 parent_project_id 无效: {}", project.id));
            }
            if project.source_snapshot_id.is_none() {
                return Err(format!("派生项目缺少 source_snapshot_id: {}", project.id));
            }
            match project.derived_project_status.as_deref() {
                Some(status) if is_valid_derived_project_status(status) => {}
                Some(status) => {
                    return Err(format!(
                        "项目族导入包派生项目状态无效: project_id={}, derived_project_status={status}",
                        project.id
                    ));
                }
                None => {
                    return Err(format!(
                        "项目族导入包派生项目缺少 derived_project_status: project_id={}",
                        project.id
                    ));
                }
            }
        }
    }
    Ok(())
}
