use super::context::{VideoProjectIdentity, video_chapter_project_id_record, video_project_record};
use super::rules::validate_positive_image_i64;
use crate::video::VideoDbError;
use a4n_db_models::models::{ai::generated_image, project::novel_project};
use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use a4n_domain::project::ProjectSourceMode;
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
    let (project_result, chapter_project_result) = futures::join!(
        video_project_record(db, project_id),
        video_chapter_project_id_record(db, chapter_id),
    );
    let project = project_result?;
    let chapter_project_id = chapter_project_result?;
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

    let projects_query = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .filter(novel_project::Column::Id.is_in(normalized_project_ids.iter().copied()))
        .into_model::<VideoProjectIdentity>()
        .all(db);
    let chapter_query = video_chapter_project_id_record(db, chapter_id);
    let (projects_result, chapter_result) = futures::join!(projects_query, chapter_query);
    let projects = projects_result?;
    let projects_by_id = projects
        .into_iter()
        .map(|project| (project.id, project))
        .collect::<HashMap<_, _>>();
    for project_id in &normalized_project_ids {
        if !projects_by_id.contains_key(project_id) {
            return Err(VideoDbError::NotFound(format!("项目不存在: {project_id}")));
        }
    }

    let chapter_project_id = chapter_result?;
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
        // Manga page outputs are downstream artifacts of the same workflow;
        // treating them as new reference inputs changes the workflow
        // fingerprint after the first page and invalidates checkpoint resume.
        .filter(generated_image::Column::ImageType.ne("manga_page"))
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
    let source_mode = ProjectSourceMode::from_identity(
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
    .map_err(|error| VideoDbError::InvalidArgument(format!("项目 {project_id}: {error}")))?;
    match source_mode {
        ProjectSourceMode::Self_ if chapter_project_id == project.id => Ok(()),
        ProjectSourceMode::Self_ => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于文本项目 {project_id}"
        ))),
        ProjectSourceMode::Standalone if chapter_project_id == project.id => Ok(()),
        ProjectSourceMode::Standalone => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于独立漫画项目 {project_id}"
        ))),
        ProjectSourceMode::Derived if project.root_project_id == Some(chapter_project_id) => Ok(()),
        ProjectSourceMode::Derived => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于派生项目 {project_id} 绑定的根文本项目"
        ))),
        ProjectSourceMode::Freeform => Err(VideoDbError::InvalidArgument(format!(
            "图片链路不支持自由画布项目 {project_id}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(
        id: i64,
        project_kind: &str,
        root_project_id: Option<i64>,
        source_snapshot_id: Option<i64>,
    ) -> VideoProjectIdentity {
        VideoProjectIdentity {
            id,
            project_kind: project_kind.to_string(),
            root_project_id,
            source_snapshot_id,
        }
    }

    #[test]
    fn standalone_comic_accepts_its_own_chapter() {
        let standalone = project(12, "comic", None, None);

        ensure_image_chapter_project_match(&standalone, 12, 12, 5)
            .expect("独立漫画章节应绑定到独立漫画项目");
    }

    #[test]
    fn standalone_comic_rejects_a_chapter_from_another_project() {
        let standalone = project(12, "comic", None, None);

        let error = ensure_image_chapter_project_match(&standalone, 3, 12, 5)
            .expect_err("独立漫画不能绑定其他项目章节");
        assert!(error.to_string().contains("不属于独立漫画项目 12"));
    }

    #[test]
    fn derived_comic_still_accepts_a_root_chapter() {
        let derived = project(12, "comic", Some(3), Some(10));

        ensure_image_chapter_project_match(&derived, 3, 12, 5)
            .expect("派生漫画章节应绑定到根文本项目");
    }
}
