use super::super::{super::ProjectFamilyPackage, source_snapshots::parse_chapter_scope_json};
use crate::project::PROJECT_KIND_TEXT;
use std::collections::HashSet;

pub(super) fn validate_package_derivations(package: &ProjectFamilyPackage) -> Result<(), String> {
    let snapshot_ids = package
        .source_snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    for project in &package.projects {
        if let Some(snapshot_id) = project.source_snapshot_id
            && !snapshot_ids.contains(&snapshot_id)
        {
            return Err(format!(
                "派生项目引用了导入包不存在的源文本快照: project_id={}, snapshot_id={}",
                project.id, snapshot_id
            ));
        }
    }
    let derivation_project_ids = package
        .project_derivations
        .iter()
        .map(|record| record.derived_project_id)
        .collect::<HashSet<_>>();
    for project in &package.projects {
        if project.project_kind != PROJECT_KIND_TEXT
            && !derivation_project_ids.contains(&project.id)
        {
            return Err(format!("派生项目缺少 project_derivation: {}", project.id));
        }
    }
    for derivation in &package.project_derivations {
        if derivation.root_project_id != package.root_project_id {
            return Err(format!("派生关系根项目无效: {}", derivation.id));
        }
        parse_chapter_scope_json(
            &derivation.chapter_scope_json,
            "派生关系 chapter_scope_json",
        )?;
        if !derivation.creation_options_json.is_object() {
            return Err(format!(
                "项目族导入包派生关系 creation_options_json 必须是 JSON 对象: project_derivation_id={}",
                derivation.id
            ));
        }
        let derived = package
            .projects
            .iter()
            .find(|project| project.id == derivation.derived_project_id)
            .ok_or_else(|| format!("派生关系引用了不存在的派生项目: {}", derivation.id))?;
        if derived.project_kind == PROJECT_KIND_TEXT {
            return Err(format!("派生关系引用了文本项目: {}", derived.id));
        }
        if derivation.derived_kind != derived.project_kind {
            return Err(format!(
                "派生关系类型与派生项目类型不一致: {}",
                derivation.id
            ));
        }
        let snapshot = package
            .source_snapshots
            .iter()
            .find(|snapshot| snapshot.id == derivation.source_snapshot_id)
            .ok_or_else(|| {
                format!(
                    "派生关系引用了导入包不存在的源文本快照: {}",
                    derivation.source_snapshot_id
                )
            })?;
        if derivation.chapter_scope_json != snapshot.chapter_scope_json {
            return Err(format!(
                "项目族导入包派生关系章节范围与源文本快照不一致: project_derivation_id={}, source_snapshot_id={}",
                derivation.id, snapshot.id
            ));
        }
    }
    Ok(())
}
