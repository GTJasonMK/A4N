//! PostgreSQL baseline schema.

use a4n_db_models::models::{
    ai::{
        embedding_config, generated_image, generated_video, image_gen_config, llm_config,
        optimization_patch, optimization_patch_set, optimization_session,
        optimization_session_message, optimization_turn, optimization_turn_event, system_settings,
        video_gen_config,
    },
    asset::{object_profile, profile_primary_material},
    canvas::{
        canvas_edge, canvas_graph, canvas_node, canvas_text_asset, canvas_text_asset_version,
    },
    chapter::{
        chapter, chapter_manga_prompt, chapter_outline, chapter_outline_provenance, chapter_scene,
        chapter_shooting_script, chapter_video_prompt, chapter_visual_plan, part_outline,
    },
    character::{
        attribute_event, character_portrait, character_state_index, foreshadowing_index,
        protagonist_behavior_record, protagonist_deletion_mark, protagonist_profile,
        snapshot_reference,
    },
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_run, generation_run_target, generation_spec, generation_spec_entity_ref,
        generation_spec_media_ref, generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, import_review_queue,
        novel_blueprint, novel_project, profile_evidence, project_source_snapshot,
        project_status_history, source_snapshot_chapter, source_snapshot_outline,
        source_snapshot_profile, standalone_comic_text_source, text_anchor,
    },
    scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    task::{cleanup_outbox, task_dependency, task_queue},
    user::{
        conversation, invite_code, llm_usage_log, message, theme_config, theme_preference,
        theme_resource, usage_stats, user, user_session,
    },
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use sea_orm::{ConnectionTrait, DbBackend, EntityName, Schema, Statement};
use sea_orm_migration::prelude::*;

macro_rules! create_entity_table {
    ($manager:expr, $schema:expr, $entity:expr) => {{
        let mut table = $schema.create_table_from_entity($entity);
        table.if_not_exists();
        $manager.create_table(table).await?;
    }};
}

macro_rules! drop_entity_table {
    ($manager:expr, $entity:expr) => {{
        $manager
            .drop_table(
                Table::drop()
                    .table(Alias::new($entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
    }};
}

macro_rules! create_entity_index {
    ($manager:expr, $name:expr, $entity:expr, [$($column:expr),+], unique) => {{
        let mut index = Index::create();
        index
            .name($name)
            .table(Alias::new($entity.table_name()))
            .if_not_exists();
        $(index.col($column);)+
        index.unique();
        $manager.create_index(index.to_owned()).await?;
    }};
    ($manager:expr, $name:expr, $entity:expr, [$($column:expr),+]) => {{
        let mut index = Index::create();
        index
            .name($name)
            .table(Alias::new($entity.table_name()))
            .if_not_exists();
        $(index.col($column);)+
        $manager.create_index(index.to_owned()).await?;
    }};
}

mod indexes;
mod integrity;
mod schema;
#[cfg(test)]
mod tests;

use indexes::create_baseline_indexes;
#[cfg(test)]
use integrity::{
    PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES, PROJECT_FAMILY_INTEGRITY_SQL,
    PROJECT_FAMILY_TRIGGER_BASE_NAMES, TEXT_SOURCE_SYNC_FUNCTION_NAMES, TEXT_SOURCE_SYNC_SQL,
    TEXT_SOURCE_SYNC_TRIGGER_BASE_NAMES,
};
use integrity::{
    create_project_family_integrity_triggers, create_text_source_sync_triggers,
    drop_project_family_integrity_functions, drop_text_source_sync_functions, execute_pg_sql,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        schema::up(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        schema::down(manager).await
    }
}
