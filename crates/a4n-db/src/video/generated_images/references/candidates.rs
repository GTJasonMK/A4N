use super::context::{VideoProjectIdentity, video_chapter_project_id_record, video_project_record};
use super::rules::validate_positive_image_i64;
use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_image, project::novel_project};
use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::HashMap;

pub async fn ensure_image_chapter_belongs_to_project_record<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let project = video_project_record(db, project_id).await?;
    let chapter_project_id = video_chapter_project_id_record(db, chapter_id).await?;
    ensure_image_chapter_project_match(&project, chapter_project_id, project_id, chapter_id)
}

async fn ensure_image_chapter_belongs_to_projects_record<C>(
    db: &C,
    project_ids: &[i64],
    chapter_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let chapter_id = validate_positive_image_i64(chapter_id, "chapter_id")?;
    let mut normalized_project_ids = Vec::with_capacity(project_ids.len());
    for project_id in project_ids {
        normalized_project_ids.push(validate_positive_image_i64(*project_id, "project_id")?);
    }
    normalized_project_ids.sort_unstable();
    normalized_project_ids.dedup();

    let projects = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .filter(novel_project::Column::Id.is_in(normalized_project_ids.iter().copied()))
        .into_model::<VideoProjectIdentity>()
        .all(db)
        .await?;
    let projects_by_id = projects
        .into_iter()
        .map(|project| (project.id, project))
        .collect::<HashMap<_, _>>();
    for project_id in &normalized_project_ids {
        if !projects_by_id.contains_key(project_id) {
            return Err(VideoDbError::NotFound(format!("项目不存在: {project_id}")));
        }
    }

    let chapter_project_id = video_chapter_project_id_record(db, chapter_id).await?;
    for project_id in normalized_project_ids {
        let project = projects_by_id
            .get(&project_id)
            .expect("missing projects checked above");
        ensure_image_chapter_project_match(project, chapter_project_id, project_id, chapter_id)?;
    }
    Ok(())
}

pub async fn video_reference_candidate_image_records<C>(
    db: &C,
    project_ids: &[i64],
    chapter_id: i64,
) -> Result<Vec<generated_image::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    ensure_image_chapter_belongs_to_projects_record(db, project_ids, chapter_id).await?;
    generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(
            Condition::any()
                .add(generated_image::Column::ChapterId.eq(chapter_id))
                .add(generated_image::Column::ChapterId.is_null())
                .add(
                    generated_image::Column::ImageType
                        .is_in([IMAGE_TYPE_SCENE_MATERIAL, IMAGE_TYPE_OBJECT_MATERIAL]),
                ),
        )
        .order_by_desc(generated_image::Column::CreatedAt)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

fn ensure_image_chapter_project_match(
    project: &VideoProjectIdentity,
    chapter_project_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), VideoDbError> {
    match project.project_kind.as_str() {
        "text" if chapter_project_id == project.id => Ok(()),
        "text" => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于文本项目 {project_id}"
        ))),
        "comic" | "video" | "galgame" if project.root_project_id == Some(chapter_project_id) => {
            Ok(())
        }
        "comic" | "video" | "galgame" => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于派生项目 {project_id} 绑定的根文本项目"
        ))),
        other => Err(VideoDbError::InvalidArgument(format!(
            "图片链路不支持项目类型 {other}: project_id={project_id}"
        ))),
    }
}
