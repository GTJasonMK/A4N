//! Backfills legacy page-image links into the Manga stage artifact graph.

use a4n_db_models::models::{
    ai::generated_image,
    chapter::{chapter_manga_prompt, manga_pipeline_artifact},
};
use a4n_domain::{
    manga::{MangaPageImageArtifactV1, MangaPipelineArtifactStage},
    source_text_hash,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    QueryFilter,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Manga page image artifacts only support PostgreSQL".into(),
            ));
        }
        backfill_manga_page_images(manager.get_connection()).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "manga_page_image_artifacts 不可逆：兼容成图产物可能已被修改".into(),
        ))
    }
}

async fn backfill_manga_page_images<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::PromptType.eq("page_generation"))
        .filter(chapter_manga_prompt::Column::GeneratedImageId.is_not_null())
        .all(db)
        .await?;
    for prompt in prompts {
        let image_id = prompt.generated_image_id.ok_or_else(|| {
            DbErr::Custom(format!(
                "漫画页面提示词 {} 的成图引用缺失，无法迁移阶段产物",
                prompt.id
            ))
        })?;
        if prompt.page_number <= 0 {
            return Err(DbErr::Custom(format!(
                "漫画页面提示词 {} 的页码无效，无法迁移成图产物",
                prompt.id
            )));
        }
        let image = generated_image::Entity::find_by_id(image_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                DbErr::Custom(format!(
                    "漫画页面提示词 {} 引用的成图 {} 不存在，无法迁移阶段产物",
                    prompt.id, image_id
                ))
            })?;
        if image.project_id != prompt.project_id
            || image.image_type != "manga_page"
            || image
                .chapter_id
                .is_some_and(|chapter_id| chapter_id != prompt.chapter_id)
        {
            return Err(DbErr::Custom(format!(
                "漫画页面提示词 {} 引用的图片 {} 不属于当前漫画页",
                prompt.id, image_id
            )));
        }

        let existing = manga_pipeline_artifact::Entity::find()
            .filter(manga_pipeline_artifact::Column::ProjectId.eq(prompt.project_id))
            .filter(manga_pipeline_artifact::Column::ChapterId.eq(prompt.chapter_id))
            .filter(
                manga_pipeline_artifact::Column::Stage.eq(MangaPipelineArtifactStage::Image.key()),
            )
            .filter(manga_pipeline_artifact::Column::PageNumber.eq(prompt.page_number))
            .one(db)
            .await?;
        let payload = MangaPageImageArtifactV1 {
            workflow_run_id: None,
            generated_image_id: image.id,
            width: image.width,
            height: image.height,
        };
        if let Some(existing) = existing {
            let existing_payload: MangaPageImageArtifactV1 =
                serde_json::from_value(existing.payload).map_err(|error| {
                    DbErr::Custom(format!(
                        "漫画页面提示词 {} 的成图阶段产物无效：{error}",
                        prompt.id
                    ))
                })?;
            if existing_payload.generated_image_id != image.id {
                return Err(DbErr::Custom(format!(
                    "漫画页面提示词 {} 的成图阶段产物与页面引用不一致",
                    prompt.id
                )));
            }
            continue;
        }

        let payload = serde_json::to_value(payload)
            .map_err(|error| DbErr::Custom(format!("漫画成图产物无法编码：{error}")))?;
        let payload_json = serde_json::to_string(&payload)
            .map_err(|error| DbErr::Custom(format!("漫画成图产物无法编码：{error}")))?;
        manga_pipeline_artifact::ActiveModel {
            project_id: Set(prompt.project_id),
            chapter_id: Set(prompt.chapter_id),
            stage: Set(MangaPipelineArtifactStage::Image.key().into()),
            page_number: Set(prompt.page_number),
            input_fingerprint: Set(source_text_hash(&format!(
                "legacy:manga_page_image:{}:{}",
                prompt.id, image.id
            ))),
            output_fingerprint: Set(source_text_hash(&payload_json)),
            payload: Set(payload),
            stale: Set(true),
            created_at: Set(prompt.created_at),
            updated_at: Set(prompt.updated_at),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}
