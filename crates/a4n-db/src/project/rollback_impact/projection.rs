use super::contracts::{ProjectRollbackImpactQuery, ProjectRollbackImpactSummary};
use super::facts::ProjectRollbackImpactFacts;

pub(in crate::project::rollback_impact) fn rollback_impacts_from_facts(
    query: &ProjectRollbackImpactQuery,
    facts: &ProjectRollbackImpactFacts,
) -> Result<Vec<ProjectRollbackImpactSummary>, super::contracts::ProjectRollbackImpactReadError> {
    let mut impacts = Vec::new();

    if query.delete_chapters {
        impacts.push(ProjectRollbackImpactSummary {
            kind: "delete_chapters".to_string(),
            title: "删除章节正文与派生数据".to_string(),
            tables: vec![
                "chapters".to_string(),
                "chapter_manga_prompts".to_string(),
                "chapter_video_prompts".to_string(),
                "chapter_scenes".to_string(),
                "object_profile".to_string(),
                "scene_profile".to_string(),
                "scene_occurrence".to_string(),
                "scene_transition".to_string(),
                "scene_entity_state".to_string(),
                "generated_images".to_string(),
                "generated_videos".to_string(),
                "task_queue".to_string(),
                "task_dependency".to_string(),
                "rag_index".to_string(),
            ],
            count: Some(impact_len_i32(
                facts.chapter_numbers.len(),
                "章节数量",
            )?),
            chapter_start: facts.chapter_numbers.iter().min().copied(),
            chapter_end: facts.chapter_numbers.iter().max().copied(),
            part_start: None,
            part_end: None,
            note: Some(format!(
                "将同步清理 {} 张章节图片、{} 条视频记录对应的本地文件、{} 个物体档案、{} 个场景档案、{} 条场景出场、{} 条场景位移、{} 条场景实体状态、项目 RAG 索引，并删除 {} 个项目任务。",
                facts.media_image_count,
                facts.media_video_count,
                facts.object_profile_count,
                facts.scene_profile_count,
                facts.scene_occurrence_count,
                facts.scene_transition_count,
                facts.scene_entity_state_count,
                facts.task_count
            )),
        });
    }

    if query.delete_chapter_outlines {
        impacts.push(ProjectRollbackImpactSummary {
            kind: "delete_chapter_outlines".to_string(),
            title: "删除章节大纲".to_string(),
            tables: vec!["chapter_outlines".to_string()],
            count: Some(impact_len_i32(facts.outline_numbers.len(), "章节大纲数量")?),
            chapter_start: facts.outline_numbers.iter().min().copied(),
            chapter_end: facts.outline_numbers.iter().max().copied(),
            part_start: None,
            part_end: None,
            note: None,
        });
    }

    if query.delete_part_outlines {
        impacts.push(ProjectRollbackImpactSummary {
            kind: "delete_part_outlines".to_string(),
            title: "删除分部大纲".to_string(),
            tables: vec!["part_outlines".to_string()],
            count: Some(impact_len_i32(facts.part_numbers.len(), "分部大纲数量")?),
            chapter_start: None,
            chapter_end: None,
            part_start: facts.part_numbers.iter().min().copied(),
            part_end: facts.part_numbers.iter().max().copied(),
            note: None,
        });
    }

    if query.delete_blueprint_versions {
        impacts.push(ProjectRollbackImpactSummary {
            kind: "delete_blueprint_versions".to_string(),
            title: "删除蓝图与版本".to_string(),
            tables: vec![
                "novel_blueprint".to_string(),
                "blueprint_version".to_string(),
                "blueprint_character".to_string(),
                "blueprint_relationship".to_string(),
            ],
            count: Some(
                [
                    facts.novel_blueprint_count,
                    facts.blueprint_version_count,
                    facts.blueprint_character_count,
                    facts.blueprint_relationship_count,
                ]
                .into_iter()
                .try_fold(0_i32, |total, count| {
                    total.checked_add(count).ok_or_else(|| {
                        super::contracts::ProjectRollbackImpactReadError::InvalidArgument(
                            "蓝图回滚影响总数超出 i32 范围".to_string(),
                        )
                    })
                })?,
            ),
            chapter_start: None,
            chapter_end: None,
            part_start: None,
            part_end: None,
            note: Some("会重置蓝图、角色和关系版本数据。".to_string()),
        });
    }

    Ok(impacts)
}

fn impact_len_i32(
    len: usize,
    label: &str,
) -> Result<i32, super::contracts::ProjectRollbackImpactReadError> {
    i32::try_from(len).map_err(|_| {
        super::contracts::ProjectRollbackImpactReadError::InvalidArgument(format!(
            "回滚影响{label}超出 i32 范围"
        ))
    })
}
