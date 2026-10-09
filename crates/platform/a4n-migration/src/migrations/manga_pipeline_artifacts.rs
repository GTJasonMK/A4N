//! Stores the latest Manga pipeline output independently for each stage.

use a4n_db_models::models::{
    chapter::{chapter_manga_prompt, manga_pipeline_artifact},
    generation::{workflow_checkpoint, workflow_run},
};
use a4n_domain::{manga::MangaPipelineArtifactStage, source_text_hash};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Schema, Statement,
};
use sea_orm_migration::prelude::*;
use serde_json::Value;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Manga pipeline artifacts only support PostgreSQL".into(),
            ));
        }
        let schema = Schema::new(DbBackend::Postgres);
        let mut table = schema.create_table_from_entity(manga_pipeline_artifact::Entity);
        table.if_not_exists();
        manager.create_table(table).await?;

        for sql in [
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_manga_pipeline_artifact_scope ON manga_pipeline_artifact (project_id, chapter_id, stage, page_number)",
            "ALTER TABLE manga_pipeline_artifact DROP CONSTRAINT IF EXISTS fk_manga_pipeline_artifact_project",
            "ALTER TABLE manga_pipeline_artifact ADD CONSTRAINT fk_manga_pipeline_artifact_project FOREIGN KEY (project_id) REFERENCES novel_project(id) ON DELETE CASCADE",
            "ALTER TABLE manga_pipeline_artifact DROP CONSTRAINT IF EXISTS ck_manga_pipeline_artifact_stage_scope",
            "ALTER TABLE manga_pipeline_artifact ADD CONSTRAINT ck_manga_pipeline_artifact_stage_scope CHECK ((stage IN ('page_content', 'adaptation', 'storyboard', 'reference_discovery', 'reference_bindings', 'drawing_prompt', 'image') AND page_number > 0) OR (stage IN ('character_binding', 'speech_selection', 'visual_anchor_extraction', 'item_extraction', 'comic_synthesis', 'page_slots') AND page_number = 0))",
            "ALTER TABLE manga_pipeline_artifact DROP CONSTRAINT IF EXISTS ck_manga_pipeline_artifact_fingerprints",
            "ALTER TABLE manga_pipeline_artifact ADD CONSTRAINT ck_manga_pipeline_artifact_fingerprints CHECK (length(input_fingerprint) > 0 AND length(output_fingerprint) > 0)",
            "CREATE INDEX IF NOT EXISTS idx_manga_pipeline_artifact_chapter ON manga_pipeline_artifact (project_id, chapter_id, stale, stage, page_number)",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }
        migrate_page_layouts(manager.get_connection()).await?;
        migrate_chapter_checkpoints(manager.get_connection()).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "manga_pipeline_artifacts 不可逆：章节级阶段产物来自 workflow checkpoints".into(),
        ))
    }
}

async fn migrate_page_layouts<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let rows = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::PromptType.eq("page_generation"))
        .all(db)
        .await?;
    for row in rows {
        let Some(layout) = row.layout.as_ref() else {
            continue;
        };
        let page_number = <u32 as std::convert::TryFrom<i32>>::try_from(row.page_number)
            .map_err(|_| DbErr::Custom("漫画页码无效，无法迁移阶段产物".into()))?;
        let mut artifacts = Vec::new();
        if let Some(pipeline) = layout.get("pipeline") {
            for (stage, field) in [
                (MangaPipelineArtifactStage::PageContent, "page_content"),
                (
                    MangaPipelineArtifactStage::PageAdaptation,
                    "page_adaptation",
                ),
                (MangaPipelineArtifactStage::Storyboard, "storyboard"),
                (
                    MangaPipelineArtifactStage::ReferenceDiscovery,
                    "reference_discovery",
                ),
                (MangaPipelineArtifactStage::DrawingPrompt, "drawing_prompt"),
                (MangaPipelineArtifactStage::Image, "image"),
            ] {
                if let Some(payload) = pipeline.get(field).filter(|value| !value.is_null()) {
                    let input_fingerprint = pipeline
                        .get("stage_fingerprints")
                        .and_then(|fingerprints| fingerprints.get(stage.fingerprint_key()))
                        .and_then(Value::as_str);
                    artifacts.push((
                        stage,
                        Some(page_number),
                        payload.clone(),
                        input_fingerprint.map(str::to_owned),
                    ));
                }
            }
        } else {
            for (stage, field) in [
                (MangaPipelineArtifactStage::Storyboard, "page_plan"),
                (MangaPipelineArtifactStage::DrawingPrompt, "drawing_prompt"),
            ] {
                if let Some(payload) = layout.get(field).filter(|value| !value.is_null()) {
                    artifacts.push((stage, Some(page_number), payload.clone(), None));
                }
            }
        }
        if let Some(payload) = layout
            .get("reference_bindings")
            .filter(|value| value.as_array().is_some_and(|items| !items.is_empty()))
        {
            artifacts.push((
                MangaPipelineArtifactStage::ReferenceBindings,
                Some(page_number),
                payload.clone(),
                Some(legacy_input_fingerprint("reference_bindings", layout)),
            ));
        }
        for (stage, page_number, payload, input_fingerprint) in artifacts {
            let output_fingerprint = source_text_hash(
                &serde_json::to_string(&payload)
                    .map_err(|error| DbErr::Custom(format!("漫画页面产物无法编码：{error}")))?,
            );
            manga_pipeline_artifact::ActiveModel {
                project_id: Set(row.project_id),
                chapter_id: Set(row.chapter_id),
                stage: Set(stage.key().to_string()),
                page_number: Set(page_number.map_or(0, |page| page as i32)),
                input_fingerprint: Set(input_fingerprint
                    .clone()
                    .unwrap_or_else(|| legacy_input_fingerprint(stage.key(), layout))),
                output_fingerprint: Set(output_fingerprint),
                payload: Set(payload),
                stale: Set(input_fingerprint.is_none()),
                created_at: Set(row.created_at),
                updated_at: Set(row.updated_at),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

async fn migrate_chapter_checkpoints<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let runs = workflow_run::Entity::find()
        .filter(workflow_run::Column::WorkflowKind.eq("manga_pipeline"))
        .filter(workflow_run::Column::ProjectId.is_not_null())
        .filter(workflow_run::Column::ChapterId.is_not_null())
        .order_by_asc(workflow_run::Column::Id)
        .all(db)
        .await?;
    let mut latest =
        std::collections::BTreeMap::<(i64, i64, String), workflow_checkpoint::Model>::new();
    for run in runs {
        let (Some(project_id), Some(chapter_id)) = (run.project_id, run.chapter_id) else {
            continue;
        };
        let prefix = format!("manga_pipeline:{chapter_id}:");
        let checkpoints = workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(run.id))
            .filter(workflow_checkpoint::Column::Status.eq("completed"))
            .order_by_asc(workflow_checkpoint::Column::StepOrder)
            .order_by_asc(workflow_checkpoint::Column::Id)
            .all(db)
            .await?;
        for checkpoint in checkpoints {
            let Some(stage_key) = chapter_stage_for_key(checkpoint.step_key.strip_prefix(&prefix))
            else {
                continue;
            };
            let key = (project_id, chapter_id, stage_key.to_string());
            let replace = latest.get(&key).is_none_or(|previous| {
                (checkpoint.updated_at, checkpoint.id) > (previous.updated_at, previous.id)
            });
            if replace {
                latest.insert(key, checkpoint);
            }
        }
    }
    for ((project_id, chapter_id, stage_key), checkpoint) in latest {
        let payload = checkpoint
            .checkpoint_json
            .as_ref()
            .and_then(|value| value.get("parsed_output"))
            .filter(|value| !value.is_null())
            .cloned()
            .ok_or_else(|| {
                DbErr::Custom(format!(
                    "已完成的漫画 checkpoint {} 缺少 parsed_output，无法迁移",
                    checkpoint.step_key
                ))
            })?;
        let input_fingerprint = checkpoint.input_fingerprint_sha256.clone();
        let output_fingerprint = source_text_hash(
            &serde_json::to_string(&payload)
                .map_err(|error| DbErr::Custom(format!("漫画 checkpoint 无法编码：{error}")))?,
        );
        let stale = input_fingerprint.is_none();
        let input_fingerprint =
            input_fingerprint.unwrap_or_else(|| legacy_input_fingerprint(&stage_key, &payload));
        manga_pipeline_artifact::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter_id),
            stage: Set(stage_key),
            page_number: Set(0),
            input_fingerprint: Set(input_fingerprint),
            output_fingerprint: Set(output_fingerprint),
            payload: Set(payload),
            stale: Set(stale),
            created_at: Set(checkpoint.created_at),
            updated_at: Set(checkpoint.updated_at),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

fn chapter_stage_for_key(key: Option<&str>) -> Option<&'static str> {
    match key? {
        "manga_character_binding" => Some("character_binding"),
        "manga_speech_selection" => Some("speech_selection"),
        "manga_visual_anchor_extraction" => Some("visual_anchor_extraction"),
        "manga_item_extraction" => Some("item_extraction"),
        "manga_chapter_comic_synthesis" => Some("comic_synthesis"),
        "page_slots" => Some("page_slots"),
        _ => None,
    }
}

fn legacy_input_fingerprint(stage: &str, source: &Value) -> String {
    source_text_hash(&format!("legacy:{stage}:{source}"))
}
