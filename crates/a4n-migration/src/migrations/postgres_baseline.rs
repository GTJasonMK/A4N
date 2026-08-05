//! PostgreSQL baseline schema.

use a4n_core::models::{
    ai::{
        embedding_config, generated_image, generated_video, image_gen_config, llm_config,
        optimization_patch, optimization_patch_set, optimization_session,
        optimization_session_message, optimization_turn, optimization_turn_event, system_settings,
        video_gen_config,
    },
    asset::{object_profile, profile_primary_material},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    chapter::{
        chapter, chapter_manga_prompt, chapter_outline, chapter_scene, chapter_shooting_script,
        chapter_video_prompt, part_outline,
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
        novel_blueprint, novel_project, profile_evidence, project_derivation,
        project_source_snapshot, project_status_history, source_snapshot_chapter,
        source_snapshot_outline, source_snapshot_profile, text_anchor,
    },
    scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    task::{cleanup_outbox, task_dependency, task_queue},
    user::{
        conversation, invite_code, llm_usage_log, message, theme_config, usage_stats, user,
        user_session,
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

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: impl Into<String>) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql))
        .await?;
    Ok(())
}

async fn create_project_family_integrity_triggers(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    for sql in PROJECT_FAMILY_INTEGRITY_SQL {
        execute_pg_sql(manager, *sql).await?;
    }
    Ok(())
}

async fn create_text_source_sync_triggers(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for sql in TEXT_SOURCE_SYNC_SQL {
        execute_pg_sql(manager, *sql).await?;
    }
    Ok(())
}

async fn drop_project_family_integrity_functions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for name in PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES.iter().rev() {
        execute_pg_sql(manager, format!("DROP FUNCTION IF EXISTS {name}()")).await?;
    }
    Ok(())
}

async fn drop_text_source_sync_functions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for name in TEXT_SOURCE_SYNC_FUNCTION_NAMES.iter().rev() {
        execute_pg_sql(manager, format!("DROP FUNCTION IF EXISTS {name}()")).await?;
    }
    Ok(())
}

async fn create_baseline_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    create_entity_index!(
        manager,
        "idx_user_username_unique",
        user::Entity,
        [user::Column::Username],
        unique
    );
    create_entity_index!(
        manager,
        "idx_user_email_unique",
        user::Entity,
        [user::Column::Email],
        unique
    );
    create_entity_index!(
        manager,
        "idx_user_oauth_identity_unique",
        user::Entity,
        [user::Column::OauthProvider, user::Column::OauthSubject],
        unique
    );
    create_entity_index!(
        manager,
        "idx_invite_code_code_unique",
        invite_code::Entity,
        [invite_code::Column::Code],
        unique
    );
    create_entity_index!(
        manager,
        "idx_invite_code_created_by_user",
        invite_code::Entity,
        [invite_code::Column::CreatedByUserId]
    );
    create_entity_index!(
        manager,
        "idx_invite_code_used_by_user",
        invite_code::Entity,
        [invite_code::Column::UsedByUserId]
    );

    create_entity_index!(
        manager,
        "idx_novel_project_kind",
        novel_project::Entity,
        [novel_project::Column::ProjectKind]
    );
    create_entity_index!(
        manager,
        "idx_novel_project_source_mode",
        novel_project::Entity,
        [novel_project::Column::SourceMode]
    );
    create_entity_index!(
        manager,
        "idx_novel_project_root",
        novel_project::Entity,
        [novel_project::Column::RootProjectId]
    );
    create_entity_index!(
        manager,
        "idx_novel_project_source_snapshot",
        novel_project::Entity,
        [novel_project::Column::SourceSnapshotId]
    );
    create_entity_index!(
        manager,
        "idx_project_derivation_derived_unique",
        project_derivation::Entity,
        [project_derivation::Column::DerivedProjectId],
        unique
    );
    create_entity_index!(
        manager,
        "idx_project_derivation_root",
        project_derivation::Entity,
        [project_derivation::Column::RootProjectId]
    );
    create_entity_index!(
        manager,
        "idx_project_source_snapshot_root",
        project_source_snapshot::Entity,
        [project_source_snapshot::Column::RootProjectId]
    );
    create_entity_index!(
        manager,
        "idx_source_snapshot_chapter_snapshot",
        source_snapshot_chapter::Entity,
        [source_snapshot_chapter::Column::SnapshotId]
    );
    create_entity_index!(
        manager,
        "idx_text_anchor_project_source",
        text_anchor::Entity,
        [
            text_anchor::Column::ProjectId,
            text_anchor::Column::SourceKind,
            text_anchor::Column::SourceId
        ]
    );
    create_entity_index!(
        manager,
        "idx_text_anchor_snapshot_chapter",
        text_anchor::Entity,
        [
            text_anchor::Column::SourceSnapshotId,
            text_anchor::Column::SourceSnapshotChapterId
        ]
    );
    create_entity_index!(
        manager,
        "idx_profile_evidence_profile",
        profile_evidence::Entity,
        [
            profile_evidence::Column::ProfileKind,
            profile_evidence::Column::ProfileId
        ]
    );

    create_entity_index!(
        manager,
        "idx_chapter_outline_project_number_unique",
        chapter_outline::Entity,
        [
            chapter_outline::Column::ProjectId,
            chapter_outline::Column::ChapterNumber
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_chapter_project_number_unique",
        chapter::Entity,
        [chapter::Column::ProjectId, chapter::Column::ChapterNumber],
        unique
    );
    create_entity_index!(
        manager,
        "idx_chapter_scene_number_unique",
        chapter_scene::Entity,
        [
            chapter_scene::Column::ChapterId,
            chapter_scene::Column::SceneNumber
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_protagonist_profile_project_name_unique",
        protagonist_profile::Entity,
        [
            protagonist_profile::Column::ProjectId,
            protagonist_profile::Column::Name
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_snapshot_reference_profile_chapter_unique",
        snapshot_reference::Entity,
        [
            snapshot_reference::Column::ProfileId,
            snapshot_reference::Column::ChapterNumber
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_deletion_mark_profile_category_key_unique",
        protagonist_deletion_mark::Entity,
        [
            protagonist_deletion_mark::Column::ProfileId,
            protagonist_deletion_mark::Column::Category,
            protagonist_deletion_mark::Column::Key
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_character_state_index_profile_chapter_unique",
        character_state_index::Entity,
        [
            character_state_index::Column::ProfileId,
            character_state_index::Column::ChapterNumber
        ],
        unique
    );

    create_entity_index!(
        manager,
        "idx_object_profile_project_normalized_name_unique",
        object_profile::Entity,
        [
            object_profile::Column::ProjectId,
            object_profile::Column::NormalizedName
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_object_profile_project_last_seen",
        object_profile::Entity,
        [
            object_profile::Column::ProjectId,
            object_profile::Column::LastSeenChapter
        ]
    );
    create_entity_index!(
        manager,
        "idx_scene_profile_project_normalized_name_unique",
        scene_profile::Entity,
        [
            scene_profile::Column::ProjectId,
            scene_profile::Column::NormalizedName
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_scene_profile_project_last_seen",
        scene_profile::Entity,
        [
            scene_profile::Column::ProjectId,
            scene_profile::Column::LastSeenChapter
        ]
    );
    create_entity_index!(
        manager,
        "idx_scene_occurrence_project_chapter_order",
        scene_occurrence::Entity,
        [
            scene_occurrence::Column::ProjectId,
            scene_occurrence::Column::ChapterId,
            scene_occurrence::Column::OccurrenceOrder
        ]
    );
    create_entity_index!(
        manager,
        "idx_scene_transition_project_chapter",
        scene_transition::Entity,
        [
            scene_transition::Column::ProjectId,
            scene_transition::Column::ChapterNumber
        ]
    );
    create_entity_index!(
        manager,
        "idx_scene_entity_state_project_scene_chapter",
        scene_entity_state::Entity,
        [
            scene_entity_state::Column::ProjectId,
            scene_entity_state::Column::SceneProfileId,
            scene_entity_state::Column::ChapterNumber
        ]
    );

    create_entity_index!(
        manager,
        "idx_optimization_session_chapter_name_lookup",
        optimization_session::Entity,
        [
            optimization_session::Column::ProjectId,
            optimization_session::Column::ChapterId,
            optimization_session::Column::Name
        ]
    );
    create_entity_index!(
        manager,
        "idx_generation_run_project_created",
        generation_run::Entity,
        [
            generation_run::Column::ProjectId,
            generation_run::Column::CreatedAt
        ]
    );
    create_entity_index!(
        manager,
        "idx_generation_run_workflow_step",
        generation_run::Entity,
        [
            generation_run::Column::Workflow,
            generation_run::Column::StepKey
        ]
    );
    create_entity_index!(
        manager,
        "idx_generation_run_target_lookup",
        generation_run_target::Entity,
        [
            generation_run_target::Column::TargetTable,
            generation_run_target::Column::TargetId
        ]
    );
    create_entity_index!(
        manager,
        "idx_generation_run_target_run",
        generation_run_target::Entity,
        [generation_run_target::Column::GenerationRunId]
    );
    create_entity_index!(
        manager,
        "idx_generation_spec_project_snapshot",
        generation_spec::Entity,
        [
            generation_spec::Column::ProjectId,
            generation_spec::Column::SourceSnapshotId
        ]
    );
    create_entity_index!(
        manager,
        "idx_generation_spec_source_context",
        generation_spec::Entity,
        [generation_spec::Column::SourceContextKind]
    );
    create_entity_index!(
        manager,
        "idx_generation_spec_canvas_text_asset",
        generation_spec::Entity,
        [generation_spec::Column::CanvasTextAssetId]
    );
    create_entity_index!(
        manager,
        "idx_prompt_artifact_spec",
        prompt_artifact::Entity,
        [prompt_artifact::Column::GenerationSpecId]
    );
    create_entity_index!(
        manager,
        "idx_media_asset_project",
        media_asset::Entity,
        [media_asset::Column::ProjectId]
    );
    create_entity_index!(
        manager,
        "idx_media_asset_source_snapshot",
        media_asset::Entity,
        [media_asset::Column::SourceSnapshotId]
    );
    create_entity_index!(
        manager,
        "idx_media_binding_target",
        media_binding::Entity,
        [
            media_binding::Column::TargetKind,
            media_binding::Column::TargetId,
            media_binding::Column::Role
        ]
    );
    create_entity_index!(
        manager,
        "idx_video_segment_project_order",
        video_segment::Entity,
        [
            video_segment::Column::ProjectId,
            video_segment::Column::OrderIndex
        ]
    );
    create_entity_index!(
        manager,
        "uidx_galgame_script_project",
        galgame_script::Entity,
        [galgame_script::Column::ProjectId],
        unique
    );
    create_entity_index!(
        manager,
        "idx_galgame_script_root_snapshot",
        galgame_script::Entity,
        [
            galgame_script::Column::RootProjectId,
            galgame_script::Column::SourceSnapshotId
        ]
    );
    create_entity_index!(
        manager,
        "uidx_galgame_scene_script_index",
        galgame_scene::Entity,
        [
            galgame_scene::Column::ScriptId,
            galgame_scene::Column::SceneIndex
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_galgame_scene_snapshot_chapter",
        galgame_scene::Entity,
        [galgame_scene::Column::SourceSnapshotChapterId]
    );
    create_entity_index!(
        manager,
        "idx_galgame_line_scene_index",
        galgame_line::Entity,
        [
            galgame_line::Column::SceneId,
            galgame_line::Column::LineIndex
        ]
    );
    create_entity_index!(
        manager,
        "idx_galgame_line_text_anchor",
        galgame_line::Entity,
        [galgame_line::Column::TextAnchorId]
    );
    create_entity_index!(
        manager,
        "uidx_galgame_choice_line_index",
        galgame_choice::Entity,
        [
            galgame_choice::Column::LineId,
            galgame_choice::Column::OptionIndex
        ],
        unique
    );
    create_entity_index!(
        manager,
        "idx_assembly_clip_timeline_order",
        assembly_clip::Entity,
        [
            assembly_clip::Column::TimelineId,
            assembly_clip::Column::OrderIndex
        ]
    );
    create_entity_index!(
        manager,
        "idx_source_sync_conflict_run",
        source_sync_conflict::Entity,
        [source_sync_conflict::Column::SyncRunId]
    );

    Ok(())
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N baseline schema only supports PostgreSQL".to_string(),
            ));
        }

        let schema = Schema::new(DbBackend::Postgres);

        create_entity_table!(manager, schema, user::Entity);
        create_entity_table!(manager, schema, invite_code::Entity);
        create_entity_table!(manager, schema, user_session::Entity);
        create_entity_table!(manager, schema, theme_config::Entity);
        create_entity_table!(manager, schema, llm_usage_log::Entity);
        create_entity_table!(manager, schema, usage_stats::Entity);

        create_entity_table!(manager, schema, novel_project::Entity);
        create_entity_table!(manager, schema, conversation::Entity);
        create_entity_table!(manager, schema, message::Entity);
        create_entity_table!(manager, schema, novel_blueprint::Entity);
        create_entity_index!(
            manager,
            "idx_novel_blueprint_project_unique",
            novel_blueprint::Entity,
            [novel_blueprint::Column::ProjectId],
            unique
        );
        create_entity_table!(manager, schema, blueprint_version::Entity);
        create_entity_table!(manager, schema, blueprint_character::Entity);
        create_entity_table!(manager, schema, blueprint_relationship::Entity);
        create_entity_table!(manager, schema, import_review_queue::Entity);
        create_entity_table!(manager, schema, project_status_history::Entity);
        create_entity_table!(manager, schema, profile_evidence::Entity);
        create_entity_table!(manager, schema, canvas_graph::Entity);
        create_entity_table!(manager, schema, canvas_node::Entity);
        create_entity_table!(manager, schema, canvas_edge::Entity);
        create_entity_table!(manager, schema, canvas_text_asset::Entity);
        create_entity_table!(manager, schema, project_source_snapshot::Entity);
        create_entity_table!(manager, schema, source_snapshot_chapter::Entity);
        create_entity_table!(manager, schema, source_snapshot_outline::Entity);
        create_entity_table!(manager, schema, source_snapshot_profile::Entity);

        create_entity_index!(
            manager,
            "idx_canvas_graph_project_kind",
            canvas_graph::Entity,
            [
                canvas_graph::Column::ProjectId,
                canvas_graph::Column::GraphKind
            ]
        );
        create_entity_index!(
            manager,
            "idx_canvas_node_graph",
            canvas_node::Entity,
            [canvas_node::Column::GraphId]
        );
        create_entity_index!(
            manager,
            "idx_canvas_node_ref",
            canvas_node::Entity,
            [canvas_node::Column::RefType, canvas_node::Column::RefId]
        );
        create_entity_index!(
            manager,
            "idx_canvas_edge_graph",
            canvas_edge::Entity,
            [canvas_edge::Column::GraphId]
        );
        create_entity_index!(
            manager,
            "idx_canvas_text_asset_project",
            canvas_text_asset::Entity,
            [canvas_text_asset::Column::ProjectId]
        );
        create_entity_table!(manager, schema, text_anchor::Entity);
        create_entity_table!(manager, schema, project_derivation::Entity);

        create_entity_table!(manager, schema, part_outline::Entity);
        create_entity_table!(manager, schema, chapter_outline::Entity);
        create_entity_table!(manager, schema, chapter::Entity);
        create_entity_table!(manager, schema, chapter_scene::Entity);
        create_entity_table!(manager, schema, chapter_shooting_script::Entity);
        create_entity_index!(
            manager,
            "uidx_chapter_shooting_script_source_revision",
            chapter_shooting_script::Entity,
            [
                chapter_shooting_script::Column::RootProjectId,
                chapter_shooting_script::Column::SourceChapterId,
                chapter_shooting_script::Column::SourceRevisionKey
            ],
            unique
        );
        create_entity_index!(
            manager,
            "idx_chapter_shooting_script_snapshot_chapter",
            chapter_shooting_script::Entity,
            [chapter_shooting_script::Column::SourceSnapshotChapterId]
        );
        create_entity_table!(manager, schema, chapter_manga_prompt::Entity);
        create_entity_table!(manager, schema, chapter_video_prompt::Entity);

        create_entity_table!(manager, schema, protagonist_profile::Entity);
        create_entity_table!(manager, schema, attribute_event::Entity);
        create_entity_table!(manager, schema, character_state_index::Entity);
        create_entity_table!(manager, schema, character_portrait::Entity);
        create_entity_table!(manager, schema, protagonist_behavior_record::Entity);
        create_entity_table!(manager, schema, protagonist_deletion_mark::Entity);
        create_entity_table!(manager, schema, snapshot_reference::Entity);
        create_entity_table!(manager, schema, foreshadowing_index::Entity);

        create_entity_table!(manager, schema, object_profile::Entity);
        create_entity_table!(manager, schema, profile_primary_material::Entity);
        create_entity_index!(
            manager,
            "idx_profile_primary_material_unique",
            profile_primary_material::Entity,
            [
                profile_primary_material::Column::ProjectId,
                profile_primary_material::Column::ProfileKind,
                profile_primary_material::Column::ProfileId
            ],
            unique
        );
        create_entity_table!(manager, schema, scene_profile::Entity);
        create_entity_table!(manager, schema, scene_occurrence::Entity);
        create_entity_table!(manager, schema, scene_transition::Entity);
        create_entity_table!(manager, schema, scene_entity_state::Entity);

        create_entity_table!(manager, schema, task_queue::Entity);
        create_entity_table!(manager, schema, task_dependency::Entity);
        create_entity_table!(manager, schema, cleanup_outbox::Entity);
        create_entity_index!(
            manager,
            "uidx_project_cleanup_outbox_dedupe",
            cleanup_outbox::Entity,
            [cleanup_outbox::Column::DedupeKey],
            unique
        );
        create_entity_index!(
            manager,
            "idx_project_cleanup_outbox_claim",
            cleanup_outbox::Entity,
            [
                cleanup_outbox::Column::Status,
                cleanup_outbox::Column::NextAttemptAt,
                cleanup_outbox::Column::LeaseExpiresAt
            ]
        );

        create_entity_table!(manager, schema, llm_config::Entity);
        create_entity_table!(manager, schema, embedding_config::Entity);
        create_entity_table!(manager, schema, image_gen_config::Entity);
        create_entity_table!(manager, schema, video_gen_config::Entity);
        create_entity_table!(manager, schema, system_settings::Entity);
        create_entity_table!(manager, schema, generated_image::Entity);
        create_entity_table!(manager, schema, generated_video::Entity);
        create_entity_table!(manager, schema, optimization_session::Entity);
        create_entity_table!(manager, schema, optimization_session_message::Entity);
        create_entity_table!(manager, schema, optimization_turn::Entity);
        create_entity_table!(manager, schema, optimization_turn_event::Entity);
        create_entity_table!(manager, schema, optimization_patch_set::Entity);
        create_entity_table!(manager, schema, optimization_patch::Entity);

        create_entity_table!(manager, schema, generation_run::Entity);
        create_entity_table!(manager, schema, generation_run_target::Entity);
        create_entity_table!(manager, schema, generation_spec::Entity);
        create_entity_table!(manager, schema, generation_spec_entity_ref::Entity);
        create_entity_table!(manager, schema, generation_spec_media_ref::Entity);
        create_entity_table!(manager, schema, generation_spec_text_anchor::Entity);
        create_entity_table!(manager, schema, prompt_artifact::Entity);

        create_entity_table!(manager, schema, media_asset::Entity);
        create_entity_table!(manager, schema, media_provenance::Entity);
        create_entity_table!(manager, schema, media_binding::Entity);

        create_entity_table!(manager, schema, video_segment::Entity);
        create_entity_table!(manager, schema, assembly_timeline::Entity);
        create_entity_table!(manager, schema, assembly_clip::Entity);

        create_entity_table!(manager, schema, galgame_script::Entity);
        create_entity_table!(manager, schema, galgame_scene::Entity);
        create_entity_table!(manager, schema, galgame_line::Entity);
        create_entity_table!(manager, schema, galgame_choice::Entity);

        create_entity_table!(manager, schema, source_sync_run::Entity);
        create_entity_table!(manager, schema, source_sync_conflict::Entity);
        create_entity_table!(manager, schema, source_sync_decision::Entity);

        create_baseline_indexes(manager).await?;
        create_project_family_integrity_triggers(manager).await?;
        create_text_source_sync_triggers(manager).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N baseline schema only supports PostgreSQL".to_string(),
            ));
        }

        drop_entity_table!(manager, source_sync_decision::Entity);
        drop_entity_table!(manager, source_sync_conflict::Entity);
        drop_entity_table!(manager, source_sync_run::Entity);
        drop_entity_table!(manager, assembly_clip::Entity);
        drop_entity_table!(manager, assembly_timeline::Entity);
        drop_entity_table!(manager, video_segment::Entity);
        drop_entity_table!(manager, galgame_choice::Entity);
        drop_entity_table!(manager, galgame_line::Entity);
        drop_entity_table!(manager, galgame_scene::Entity);
        drop_entity_table!(manager, galgame_script::Entity);
        drop_entity_table!(manager, media_binding::Entity);
        drop_entity_table!(manager, media_provenance::Entity);
        drop_entity_table!(manager, media_asset::Entity);
        drop_entity_table!(manager, prompt_artifact::Entity);
        drop_entity_table!(manager, generation_spec_text_anchor::Entity);
        drop_entity_table!(manager, generation_spec_media_ref::Entity);
        drop_entity_table!(manager, generation_spec_entity_ref::Entity);
        drop_entity_table!(manager, generation_spec::Entity);
        drop_entity_table!(manager, generation_run_target::Entity);
        drop_entity_table!(manager, generation_run::Entity);
        drop_entity_table!(manager, optimization_turn_event::Entity);
        drop_entity_table!(manager, optimization_turn::Entity);
        drop_entity_table!(manager, optimization_patch::Entity);
        drop_entity_table!(manager, optimization_patch_set::Entity);
        drop_entity_table!(manager, optimization_session_message::Entity);
        drop_entity_table!(manager, optimization_session::Entity);
        drop_entity_table!(manager, generated_video::Entity);
        drop_entity_table!(manager, generated_image::Entity);
        drop_entity_table!(manager, system_settings::Entity);
        drop_entity_table!(manager, video_gen_config::Entity);
        drop_entity_table!(manager, image_gen_config::Entity);
        drop_entity_table!(manager, embedding_config::Entity);
        drop_entity_table!(manager, llm_config::Entity);
        drop_entity_table!(manager, task_dependency::Entity);
        drop_entity_table!(manager, task_queue::Entity);
        drop_entity_table!(manager, cleanup_outbox::Entity);
        drop_entity_table!(manager, scene_entity_state::Entity);
        drop_entity_table!(manager, scene_transition::Entity);
        drop_entity_table!(manager, scene_occurrence::Entity);
        drop_entity_table!(manager, scene_profile::Entity);
        drop_entity_table!(manager, profile_primary_material::Entity);
        drop_entity_table!(manager, object_profile::Entity);
        drop_entity_table!(manager, foreshadowing_index::Entity);
        drop_entity_table!(manager, snapshot_reference::Entity);
        drop_entity_table!(manager, protagonist_deletion_mark::Entity);
        drop_entity_table!(manager, protagonist_behavior_record::Entity);
        drop_entity_table!(manager, character_portrait::Entity);
        drop_entity_table!(manager, character_state_index::Entity);
        drop_entity_table!(manager, attribute_event::Entity);
        drop_entity_table!(manager, protagonist_profile::Entity);
        drop_entity_table!(manager, chapter_video_prompt::Entity);
        drop_entity_table!(manager, chapter_manga_prompt::Entity);
        drop_entity_table!(manager, chapter_shooting_script::Entity);
        drop_entity_table!(manager, chapter_scene::Entity);
        drop_entity_table!(manager, chapter::Entity);
        drop_entity_table!(manager, chapter_outline::Entity);
        drop_entity_table!(manager, part_outline::Entity);
        drop_entity_table!(manager, project_derivation::Entity);
        drop_entity_table!(manager, text_anchor::Entity);
        drop_entity_table!(manager, source_snapshot_profile::Entity);
        drop_entity_table!(manager, source_snapshot_outline::Entity);
        drop_entity_table!(manager, source_snapshot_chapter::Entity);
        drop_entity_table!(manager, project_source_snapshot::Entity);
        drop_entity_table!(manager, profile_evidence::Entity);
        drop_entity_table!(manager, project_status_history::Entity);
        drop_entity_table!(manager, import_review_queue::Entity);
        drop_entity_table!(manager, blueprint_relationship::Entity);
        drop_entity_table!(manager, blueprint_character::Entity);
        drop_entity_table!(manager, blueprint_version::Entity);
        drop_entity_table!(manager, novel_blueprint::Entity);
        drop_entity_table!(manager, novel_project::Entity);
        drop_entity_table!(manager, usage_stats::Entity);
        drop_entity_table!(manager, llm_usage_log::Entity);
        drop_entity_table!(manager, theme_config::Entity);
        drop_entity_table!(manager, message::Entity);
        drop_entity_table!(manager, conversation::Entity);
        drop_entity_table!(manager, user_session::Entity);
        drop_entity_table!(manager, invite_code::Entity);
        drop_entity_table!(manager, user::Entity);
        drop_text_source_sync_functions(manager).await?;
        drop_project_family_integrity_functions(manager).await?;

        Ok(())
    }
}

const PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES: &[&str] = &[
    "a4n_validate_novel_project_family",
    "a4n_validate_project_source_snapshot_family",
    "a4n_validate_source_snapshot_chapter_family",
    "a4n_validate_text_anchor_family",
    "a4n_validate_project_derivation_family",
    "a4n_validate_generation_spec_family",
    "a4n_validate_generation_spec_text_anchor_family",
    "a4n_validate_generation_spec_entity_ref_family",
    "a4n_validate_media_asset_family",
    "a4n_validate_generation_spec_media_ref_family",
    "a4n_validate_media_provenance_family",
    "a4n_validate_galgame_script_family",
    "a4n_validate_galgame_scene_family",
    "a4n_validate_galgame_scene_link_family",
    "a4n_validate_galgame_line_family",
    "a4n_validate_galgame_line_order_family",
    "a4n_validate_galgame_choice_family",
    "a4n_validate_galgame_media_binding",
    "a4n_validate_galgame_target_reverse_integrity",
    "a4n_validate_video_segment_family",
    "a4n_validate_assembly_timeline_family",
    "a4n_validate_assembly_clip_family",
    "a4n_validate_source_sync_run_family",
    "a4n_validate_source_sync_conflict_family",
    "a4n_validate_source_sync_decision_family",
];

const TEXT_SOURCE_SYNC_FUNCTION_NAMES: &[&str] = &["a4n_mark_text_source_project_changed"];

#[cfg(test)]
const TEXT_SOURCE_SYNC_TRIGGER_BASE_NAMES: &[&str] = &[
    "trg_chapter_text_source_changed",
    "trg_chapter_outline_text_source_changed",
    "trg_part_outline_text_source_changed",
    "trg_blueprint_version_text_source_changed",
    "trg_protagonist_profile_text_source_changed",
    "trg_object_profile_text_source_changed",
    "trg_scene_profile_text_source_changed",
];

const PROJECT_FAMILY_INTEGRITY_SQL: &[&str] = &[
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_novel_project_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.project_kind NOT IN ('text','comic','video','galgame') THEN
        RAISE EXCEPTION 'novel_project.project_kind 无效';
    END IF;
    IF NEW.source_mode NOT IN ('self','derived','freeform') THEN
        RAISE EXCEPTION 'novel_project.source_mode 无效';
    END IF;
    IF NEW.source_mode = 'self' AND NEW.project_kind <> 'text' THEN
        RAISE EXCEPTION 'self 项目必须是文本项目';
    END IF;
    IF NEW.source_mode = 'derived' AND NEW.project_kind = 'text' THEN
        RAISE EXCEPTION 'derived 项目不能是文本项目';
    END IF;
    IF NEW.source_mode = 'freeform' AND NEW.project_kind <> 'video' THEN
        RAISE EXCEPTION 'freeform 项目当前只支持视频项目';
    END IF;
    IF NEW.source_mode IN ('self','freeform') AND (
        NEW.root_project_id IS NOT NULL
        OR NEW.parent_project_id IS NOT NULL
        OR NEW.source_snapshot_id IS NOT NULL
        OR NEW.derived_project_status IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'self/freeform 项目不能绑定派生项目字段';
    END IF;
    IF NEW.source_mode = 'derived' AND (
        NEW.root_project_id IS NULL
        OR NEW.parent_project_id IS NULL
        OR NEW.source_snapshot_id IS NULL
        OR NEW.derived_project_status IS NULL
    ) THEN
        RAISE EXCEPTION '派生项目必须绑定根项目、父项目、源快照和派生状态';
    END IF;
    IF NEW.source_mode = 'derived' AND NEW.parent_project_id IS DISTINCT FROM NEW.root_project_id THEN
        RAISE EXCEPTION '第一阶段派生项目 parent_project_id 必须等于 root_project_id';
    END IF;
    IF NEW.source_mode = 'derived' AND NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id
          AND root.project_kind = 'text'
          AND root.source_mode = 'self'
    ) THEN
        RAISE EXCEPTION '派生项目 root_project_id 必须指向文本项目';
    END IF;
    IF NEW.parent_project_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM novel_project parent
        WHERE parent.id = NEW.parent_project_id
          AND parent.project_kind = 'text'
          AND parent.source_mode = 'self'
    ) THEN
        RAISE EXCEPTION '派生项目 parent_project_id 必须指向文本项目';
    END IF;
    IF NEW.source_snapshot_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '派生项目 source_snapshot_id 必须属于 root_project_id';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_novel_project_family_insert
BEFORE INSERT ON novel_project
FOR EACH ROW EXECUTE FUNCTION a4n_validate_novel_project_family()
"#,
    r#"
CREATE TRIGGER trg_novel_project_family_update
BEFORE UPDATE OF project_kind, source_mode, root_project_id, parent_project_id, source_snapshot_id, derived_project_status ON novel_project
FOR EACH ROW EXECUTE FUNCTION a4n_validate_novel_project_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_project_source_snapshot_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id
          AND root.project_kind = 'text'
    ) THEN
        RAISE EXCEPTION '源文本快照 root_project_id 必须指向文本项目';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_project_source_snapshot_family_insert
BEFORE INSERT ON project_source_snapshot
FOR EACH ROW EXECUTE FUNCTION a4n_validate_project_source_snapshot_family()
"#,
    r#"
CREATE TRIGGER trg_project_source_snapshot_family_update
BEFORE UPDATE OF root_project_id ON project_source_snapshot
FOR EACH ROW EXECUTE FUNCTION a4n_validate_project_source_snapshot_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_source_snapshot_chapter_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '源快照章节必须属于同一根文本快照';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM chapter source_chapter
        WHERE source_chapter.id = NEW.source_chapter_id
          AND source_chapter.project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '源快照章节 source_chapter_id 必须属于根文本项目';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_source_snapshot_chapter_family_insert
BEFORE INSERT ON source_snapshot_chapter
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_snapshot_chapter_family()
"#,
    r#"
CREATE TRIGGER trg_source_snapshot_chapter_family_update
BEFORE UPDATE OF snapshot_id, root_project_id, source_chapter_id ON source_snapshot_chapter
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_snapshot_chapter_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_text_anchor_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.project_id
          AND root.project_kind = 'text'
    ) THEN
        RAISE EXCEPTION '文本锚点 project_id 必须指向文本根项目';
    END IF;
    IF NEW.source_snapshot_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.project_id
    ) THEN
        RAISE EXCEPTION '文本锚点 source_snapshot_id 必须属于文本根项目';
    END IF;
    IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM source_snapshot_chapter chapter
        WHERE chapter.id = NEW.source_snapshot_chapter_id
          AND chapter.snapshot_id = NEW.source_snapshot_id
          AND chapter.root_project_id = NEW.project_id
    ) THEN
        RAISE EXCEPTION '文本锚点 source_snapshot_chapter_id 必须属于目标快照';
    END IF;
    IF NEW.source_chapter_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM chapter source_chapter
        WHERE source_chapter.id = NEW.source_chapter_id
          AND source_chapter.project_id = NEW.project_id
    ) THEN
        RAISE EXCEPTION '文本锚点 source_chapter_id 必须属于文本根项目';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_text_anchor_family_insert
BEFORE INSERT ON text_anchor
FOR EACH ROW EXECUTE FUNCTION a4n_validate_text_anchor_family()
"#,
    r#"
CREATE TRIGGER trg_text_anchor_family_update
BEFORE UPDATE OF project_id, source_chapter_id, source_snapshot_id, source_snapshot_chapter_id ON text_anchor
FOR EACH ROW EXECUTE FUNCTION a4n_validate_text_anchor_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_project_derivation_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id
          AND root.project_kind = 'text'
    ) THEN
        RAISE EXCEPTION '派生关系 root_project_id 必须指向文本项目';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM novel_project derived
        WHERE derived.id = NEW.derived_project_id
          AND derived.project_kind = NEW.derived_kind
          AND derived.project_kind <> 'text'
          AND derived.root_project_id = NEW.root_project_id
          AND derived.source_snapshot_id = NEW.source_snapshot_id
    ) THEN
        RAISE EXCEPTION '派生关系必须与派生项目主体一致';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '派生关系 source_snapshot_id 必须属于根文本项目';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_project_derivation_family_insert
BEFORE INSERT ON project_derivation
FOR EACH ROW EXECUTE FUNCTION a4n_validate_project_derivation_family()
"#,
    r#"
CREATE TRIGGER trg_project_derivation_family_update
BEFORE UPDATE OF root_project_id, derived_project_id, derived_kind, source_snapshot_id ON project_derivation
FOR EACH ROW EXECUTE FUNCTION a4n_validate_project_derivation_family()
"#,
    crate::migrations::galgame_generation_spec_contract::GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
    r#"
CREATE TRIGGER trg_generation_spec_family_insert
BEFORE INSERT ON generation_spec
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_family()
"#,
    crate::migrations::galgame_generation_spec_contract::GALGAME_GENERATION_SPEC_FAMILY_UPDATE_TRIGGER_SQL,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_text_anchor_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM generation_spec spec
        WHERE spec.id = NEW.generation_spec_id
          AND COALESCE(NULLIF(spec.source_context_kind, ''), 'snapshot_backed') = 'freeform'
    ) THEN
        RAISE EXCEPTION 'freeform 生成意图不能绑定 text_anchor';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM generation_spec spec
        JOIN text_anchor anchor ON anchor.id = NEW.text_anchor_id
        WHERE spec.id = NEW.generation_spec_id
          AND anchor.project_id = spec.root_project_id
          AND anchor.source_snapshot_id = spec.source_snapshot_id
    ) THEN
        RAISE EXCEPTION '生成意图文本锚点必须属于同一根项目和源快照';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM generation_spec spec
        JOIN text_anchor anchor ON anchor.id = NEW.text_anchor_id
        WHERE spec.id = NEW.generation_spec_id
          AND spec.source_snapshot_chapter_id IS NOT NULL
          AND anchor.source_snapshot_chapter_id IS DISTINCT FROM spec.source_snapshot_chapter_id
    ) THEN
        RAISE EXCEPTION '生成意图文本锚点必须属于同一源快照章节';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM generation_spec spec
        JOIN text_anchor anchor ON anchor.id = NEW.text_anchor_id
        WHERE spec.id = NEW.generation_spec_id
          AND spec.source_snapshot_chapter_id IS NOT NULL
          AND spec.source_chapter_id IS NOT NULL
          AND anchor.source_chapter_id IS DISTINCT FROM spec.source_chapter_id
    ) THEN
        RAISE EXCEPTION '生成意图文本锚点 source_chapter_id 必须与生成意图一致';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_generation_spec_text_anchor_family_insert
BEFORE INSERT ON generation_spec_text_anchor
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_text_anchor_family()
"#,
    r#"
CREATE TRIGGER trg_generation_spec_text_anchor_family_update
BEFORE UPDATE OF generation_spec_id, text_anchor_id ON generation_spec_text_anchor
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_text_anchor_family()
"#,
    crate::migrations::galgame_generation_spec_contract::GENERATION_SPEC_ENTITY_REF_FAMILY_FUNCTION_SQL,
    crate::migrations::galgame_generation_spec_contract::GENERATION_SPEC_ENTITY_REF_FAMILY_INSERT_TRIGGER_SQL,
    crate::migrations::galgame_generation_spec_contract::GENERATION_SPEC_ENTITY_REF_FAMILY_UPDATE_TRIGGER_SQL,
    crate::migrations::media_asset_generation_ref_contract::MEDIA_ASSET_FAMILY_STRICT_FUNCTION_SQL,
    r#"
CREATE TRIGGER trg_media_asset_family_insert
BEFORE INSERT ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
    r#"
CREATE TRIGGER trg_media_asset_family_update
BEFORE UPDATE OF project_id, root_project_id, source_snapshot_id, source_chapter_id, source_snapshot_chapter_id, status, file_path ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
    crate::migrations::media_asset_generation_ref_contract::GENERATION_SPEC_MEDIA_REF_STRICT_FUNCTION_SQL,
    r#"
CREATE TRIGGER trg_generation_spec_media_ref_family_insert
BEFORE INSERT ON generation_spec_media_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_media_ref_family()
"#,
    r#"
CREATE TRIGGER trg_generation_spec_media_ref_family_update
BEFORE UPDATE OF generation_spec_id, media_asset_id, role ON generation_spec_media_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_media_ref_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_media_provenance_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    request_source TEXT;
BEGIN
    IF NEW.generation_spec_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN generation_spec spec ON spec.id = NEW.generation_spec_id
        WHERE asset.id = NEW.media_asset_id
          AND spec.project_id = asset.project_id
          AND spec.root_project_id = asset.root_project_id
          AND (
              (asset.source_snapshot_id IS NULL
               AND spec.source_snapshot_id IS NULL
               AND spec.source_snapshot_chapter_id IS NULL)
              OR spec.source_snapshot_id = asset.source_snapshot_id
          )
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR spec.source_snapshot_chapter_id = asset.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR spec.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION '媒体来源 generation_spec_id 必须匹配媒体资产项目族和源快照上下文';
    END IF;
    IF NEW.prompt_artifact_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM prompt_artifact prompt
        JOIN generation_spec spec ON spec.id = prompt.generation_spec_id
        JOIN media_asset asset ON asset.id = NEW.media_asset_id
        WHERE prompt.id = NEW.prompt_artifact_id
          AND spec.project_id = asset.project_id
          AND spec.root_project_id = asset.root_project_id
          AND (NEW.generation_spec_id IS NULL OR spec.id = NEW.generation_spec_id)
          AND (
              (asset.source_snapshot_id IS NULL
               AND spec.source_snapshot_id IS NULL
               AND spec.source_snapshot_chapter_id IS NULL)
              OR spec.source_snapshot_id = asset.source_snapshot_id
          )
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR spec.source_snapshot_chapter_id = asset.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR spec.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION '媒体来源 prompt_artifact_id 必须匹配媒体资产项目族和源快照上下文';
    END IF;
    IF NEW.source_media_asset_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN media_asset source ON source.id = NEW.source_media_asset_id
        WHERE asset.id = NEW.media_asset_id
          AND source.project_id = asset.project_id
          AND source.root_project_id = asset.root_project_id
          AND source.status = 'completed'
          AND source.file_path IS NOT NULL
          AND (
              (asset.source_snapshot_id IS NULL
               AND source.source_snapshot_id IS NULL
               AND source.source_snapshot_chapter_id IS NULL)
              OR source.source_snapshot_id = asset.source_snapshot_id
          )
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR source.source_snapshot_chapter_id = asset.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR source.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION '媒体来源 source_media_asset_id 必须匹配媒体资产项目族和源快照上下文';
    END IF;
    IF NEW.request_snapshot_json IS NOT NULL THEN
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb) IS DISTINCT FROM 'object' THEN
            RAISE EXCEPTION '媒体来源 request_snapshot_json 必须是 JSON 对象';
        END IF;
        request_source := BTRIM(NEW.request_snapshot_json::jsonb ->> 'source');
    END IF;

    IF NEW.generation_spec_id IS NOT NULL AND EXISTS (
        SELECT 1
        FROM generation_spec spec
        WHERE spec.id = NEW.generation_spec_id
          AND (
              (spec.target_kind = 'galgame_background' AND spec.target_entity_kind = 'galgame_scene')
              OR (spec.target_kind = 'galgame_cg' AND spec.target_entity_kind = 'galgame_line')
          )
    ) THEN
        IF request_source IS DISTINCT FROM 'galgame_asset_generation' THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.source 必须是 galgame_asset_generation';
        END IF;
    END IF;

    IF request_source = 'galgame_asset_generation' THEN
        IF NEW.generation_spec_id IS NULL THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源缺少 generation_spec_id';
        END IF;
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_kind') IS DISTINCT FROM 'string'
           OR BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_kind') = '' THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.target_kind 缺失';
        END IF;
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_entity_kind') IS DISTINCT FROM 'string'
           OR BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_entity_kind') = '' THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.target_entity_kind 缺失';
        END IF;
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_entity_id') IS DISTINCT FROM 'number'
           OR NOT ((NEW.request_snapshot_json::jsonb ->> 'target_entity_id') ~ '^[0-9]+$') THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json.target_entity_id 缺失';
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM generation_spec spec
            WHERE spec.id = NEW.generation_spec_id
              AND spec.target_kind = BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_kind')
              AND spec.target_entity_kind = BTRIM(NEW.request_snapshot_json::jsonb ->> 'target_entity_kind')
              AND spec.target_entity_id = (NEW.request_snapshot_json::jsonb ->> 'target_entity_id')::BIGINT
        ) THEN
            RAISE EXCEPTION 'Galgame 资产媒体来源 request_snapshot_json 与 generation_spec 目标不一致';
        END IF;
    END IF;

    IF request_source = 'galgame_audio_import'
       AND NOT (NEW.request_snapshot_json::jsonb ? 'target_line_id') THEN
        RAISE EXCEPTION 'Galgame 音频导入媒体来源 request_snapshot_json.target_line_id 缺失';
    END IF;
    IF NEW.request_snapshot_json IS NOT NULL
       AND NEW.request_snapshot_json::jsonb ? 'target_line_id' THEN
        IF jsonb_typeof(NEW.request_snapshot_json::jsonb -> 'target_line_id') IS DISTINCT FROM 'number'
           OR NOT ((NEW.request_snapshot_json::jsonb ->> 'target_line_id') ~ '^[0-9]+$') THEN
            RAISE EXCEPTION '媒体来源 request_snapshot_json.target_line_id 必须是整数';
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM media_asset asset
            JOIN galgame_line line ON line.id = (NEW.request_snapshot_json::jsonb ->> 'target_line_id')::BIGINT
            JOIN galgame_scene scene ON scene.id = line.scene_id
            JOIN galgame_script script ON script.id = scene.script_id
            WHERE asset.id = NEW.media_asset_id
              AND asset.project_id = script.project_id
              AND asset.root_project_id = script.root_project_id
              AND asset.source_snapshot_id = script.source_snapshot_id
        ) THEN
            RAISE EXCEPTION '媒体来源 request_snapshot_json.target_line_id 必须指向同项目 Galgame 行';
        END IF;
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_media_provenance_family_insert
BEFORE INSERT ON media_provenance
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_provenance_family()
"#,
    r#"
CREATE TRIGGER trg_media_provenance_family_update
BEFORE UPDATE OF media_asset_id, generation_spec_id, prompt_artifact_id, source_media_asset_id, request_snapshot_json ON media_provenance
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_provenance_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_script_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind_value TEXT;
    source_mode_value TEXT;
    project_root_project_id BIGINT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.source_mode,
           project_row.root_project_id,
           project_row.source_snapshot_id
      INTO project_kind_value,
           source_mode_value,
           project_root_project_id,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind_value IS NULL THEN
        RAISE EXCEPTION 'Galgame 剧本 project_id 不存在';
    END IF;
    IF project_kind_value <> 'galgame' OR source_mode_value <> 'derived' THEN
        RAISE EXCEPTION 'Galgame 剧本 project_id 必须指向 galgame 派生项目';
    END IF;
    IF project_root_project_id IS DISTINCT FROM NEW.root_project_id
       OR project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id THEN
        RAISE EXCEPTION 'Galgame 剧本必须匹配派生项目的根项目和源快照';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION 'Galgame 剧本 source_snapshot_id 必须属于 root_project_id';
    END IF;
    IF NEW.status NOT IN ('draft', 'generating', 'ready') THEN
        RAISE EXCEPTION 'Galgame 剧本 status 无效';
    END IF;

    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_galgame_script_family_insert
BEFORE INSERT ON galgame_script
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_script_family()
"#,
    r#"
CREATE TRIGGER trg_galgame_script_family_update
BEFORE UPDATE OF project_id, root_project_id, source_snapshot_id, status ON galgame_script
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_script_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    scene_row galgame_scene%ROWTYPE;
    script_root_project_id BIGINT;
BEGIN
    SELECT * INTO scene_row
    FROM galgame_scene
    WHERE id = NEW.id;

    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    IF scene_row.scene_index <= 0 THEN
        RAISE EXCEPTION 'Galgame 场 scene_index 必须是正整数';
    END IF;
    IF BTRIM(scene_row.title) = '' THEN
        RAISE EXCEPTION 'Galgame 场 title 不能为空';
    END IF;
    IF scene_row.entry_kind NOT IN ('main', 'branch') THEN
        RAISE EXCEPTION 'Galgame 场 entry_kind 无效';
    END IF;
    IF jsonb_typeof(scene_row.participants_json::jsonb) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION 'Galgame 场 participants_json 必须是数组';
    END IF;
    SELECT script.root_project_id
      INTO script_root_project_id
    FROM galgame_script script
    JOIN source_snapshot_chapter chapter
      ON chapter.id = scene_row.source_snapshot_chapter_id
    WHERE script.id = scene_row.script_id
      AND chapter.snapshot_id = script.source_snapshot_id
      AND chapter.root_project_id = script.root_project_id;
    IF script_root_project_id IS NULL THEN
        RAISE EXCEPTION 'Galgame 场 source_snapshot_chapter_id 必须属于剧本源快照';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM jsonb_array_elements(scene_row.participants_json::jsonb) participant(item)
        WHERE jsonb_typeof(participant.item) IS DISTINCT FROM 'object'
    ) THEN
        RAISE EXCEPTION 'Galgame 场 participants_json 项必须是对象';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM jsonb_array_elements(scene_row.participants_json::jsonb) participant(item)
        WHERE participant.item ? 'character_profile_id'
          AND CASE
              WHEN jsonb_typeof(participant.item -> 'character_profile_id') IS DISTINCT FROM 'number' THEN TRUE
              WHEN NOT ((participant.item ->> 'character_profile_id') ~ '^[0-9]+$') THEN TRUE
              WHEN (participant.item ->> 'character_profile_id')::BIGINT <= 0 THEN TRUE
              ELSE FALSE
          END
    ) THEN
        RAISE EXCEPTION 'Galgame 场 participants_json.character_profile_id 必须是正整数';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM jsonb_array_elements(scene_row.participants_json::jsonb) participant(item)
        WHERE participant.item ? 'character_profile_id'
          AND NOT EXISTS (
              SELECT 1
              FROM protagonist_profile profile
              WHERE profile.id = (participant.item ->> 'character_profile_id')::BIGINT
                AND profile.project_id = script_root_project_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 场 participants_json.character_profile_id 必须属于剧本根项目';
    END IF;

    RETURN NEW;
END;
$$
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_family_insert
AFTER INSERT ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_family_update
AFTER UPDATE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_link_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    affected_script_ids BIGINT[];
    check_script_id BIGINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        affected_script_ids := ARRAY[NEW.script_id];
    ELSIF TG_OP = 'DELETE' THEN
        affected_script_ids := ARRAY[OLD.script_id];
    ELSIF OLD.script_id IS DISTINCT FROM NEW.script_id THEN
        affected_script_ids := ARRAY[OLD.script_id, NEW.script_id];
    ELSE
        affected_script_ids := ARRAY[NEW.script_id];
    END IF;

    FOREACH check_script_id IN ARRAY affected_script_ids LOOP
        IF NOT EXISTS (
            SELECT 1
            FROM galgame_scene scene
            WHERE scene.script_id = check_script_id
        ) THEN
            CONTINUE;
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            WHERE scene.script_id = check_script_id
              AND scene.next_scene_id = scene.id
        ) THEN
            RAISE EXCEPTION 'Galgame 场 next_scene_id 不能指向自身';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            LEFT JOIN galgame_scene target
              ON target.id = scene.next_scene_id
             AND target.script_id = scene.script_id
            WHERE scene.script_id = check_script_id
              AND scene.next_scene_id IS NOT NULL
              AND target.id IS NULL
        ) THEN
            RAISE EXCEPTION 'Galgame 场 next_scene_id 必须属于同一剧本';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            WHERE scene.script_id = check_script_id
              AND scene.entry_kind = 'branch'
              AND scene.next_scene_id IS NULL
        ) THEN
            RAISE EXCEPTION 'Galgame branch 场缺少 next_scene_id';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            JOIN galgame_scene target
              ON target.id = scene.next_scene_id
             AND target.script_id = scene.script_id
            WHERE scene.script_id = check_script_id
              AND scene.entry_kind = 'branch'
              AND target.entry_kind <> 'main'
        ) THEN
            RAISE EXCEPTION 'Galgame branch 场 next_scene_id 必须指向主线场';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_insert
AFTER INSERT ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_link_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_update
AFTER UPDATE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_link_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_delete
AFTER DELETE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_link_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    line_row galgame_line%ROWTYPE;
    script_root_project_id BIGINT;
    script_source_snapshot_id BIGINT;
    scene_source_snapshot_chapter_id BIGINT;
    scene_source_chapter_id BIGINT;
    choice_count BIGINT;
    choice_distinct_index_count BIGINT;
    choice_min_option_index INTEGER;
    choice_max_option_index INTEGER;
BEGIN
    SELECT * INTO line_row
    FROM galgame_line
    WHERE id = NEW.id;

    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    SELECT script.root_project_id,
           script.source_snapshot_id,
           scene.source_snapshot_chapter_id,
           chapter.source_chapter_id
      INTO script_root_project_id,
           script_source_snapshot_id,
           scene_source_snapshot_chapter_id,
           scene_source_chapter_id
    FROM galgame_scene scene
    JOIN galgame_script script ON script.id = scene.script_id
    JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
    WHERE scene.id = line_row.scene_id;

    IF script_root_project_id IS NULL THEN
        RAISE EXCEPTION 'Galgame 行 scene_id 不存在';
    END IF;
    IF line_row.line_index <= 0 THEN
        RAISE EXCEPTION 'Galgame 行 line_index 必须是正整数';
    END IF;
    IF line_row.line_type NOT IN ('dialogue', 'narration', 'monologue', 'stage_direction', 'choice') THEN
        RAISE EXCEPTION 'Galgame 行 line_type 无效';
    END IF;
    IF BTRIM(line_row.text) = '' THEN
        RAISE EXCEPTION 'Galgame 行 text 不能为空';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json 必须是对象';
    END IF;
    IF line_row.presentation_json::jsonb ? 'cg'
       AND jsonb_typeof(line_row.presentation_json::jsonb -> 'cg') NOT IN ('boolean', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg 必须是布尔值';
    END IF;
    IF line_row.presentation_json::jsonb ? 'cg_importance'
       AND jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_importance') NOT IN ('string', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg_importance 必须是字符串';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_importance') = 'string'
       AND BTRIM(line_row.presentation_json::jsonb ->> 'cg_importance') <> ''
       AND BTRIM(line_row.presentation_json::jsonb ->> 'cg_importance') NOT IN ('normal', 'high', 'critical') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg_importance 无效';
    END IF;
    IF (line_row.presentation_json::jsonb ->> 'cg') = 'true'
       AND COALESCE(
           CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_note') = 'string'
                THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_note'), '') END,
           CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_prompt') = 'string'
                THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_prompt'), '') END,
           CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'event_cg_note') = 'string'
                THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'event_cg_note'), '') END
       ) IS NULL THEN
        RAISE EXCEPTION 'Galgame 行启用事件 CG 时必须提供 cg_note';
    END IF;
    IF line_row.expression IS NOT NULL
       AND line_row.expression NOT IN ('neutral', 'smile', 'sad', 'angry', 'surprised', 'shy', 'serious') THEN
        RAISE EXCEPTION 'Galgame 行 expression 无效';
    END IF;
    IF line_row.position IS NOT NULL
       AND line_row.position NOT IN ('left', 'center', 'right') THEN
        RAISE EXCEPTION 'Galgame 行 position 无效';
    END IF;

    IF line_row.line_type IN ('dialogue', 'monologue') THEN
        IF line_row.speaker_profile_id IS NULL THEN
            RAISE EXCEPTION 'dialogue/monologue 行必须绑定 speaker_profile_id';
        END IF;
        IF line_row.expression IS NULL OR BTRIM(line_row.expression) = '' THEN
            RAISE EXCEPTION 'dialogue/monologue 行必须显式提供 expression';
        END IF;
    ELSIF line_row.speaker_profile_id IS NOT NULL THEN
        RAISE EXCEPTION '非 dialogue/monologue 行不能绑定 speaker_profile_id';
    END IF;

    IF line_row.speaker_profile_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM protagonist_profile profile
        WHERE profile.id = line_row.speaker_profile_id
          AND profile.project_id = script_root_project_id
    ) THEN
        RAISE EXCEPTION 'Galgame 行 speaker_profile_id 必须属于剧本根项目';
    END IF;

    IF line_row.text_anchor_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM text_anchor anchor
        WHERE anchor.id = line_row.text_anchor_id
          AND anchor.project_id = script_root_project_id
          AND anchor.source_snapshot_id = script_source_snapshot_id
          AND anchor.source_snapshot_chapter_id = scene_source_snapshot_chapter_id
          AND (
              anchor.source_chapter_id IS NULL
              OR anchor.source_chapter_id = scene_source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 行 text_anchor_id 必须属于所在场源快照章节';
    END IF;

    IF line_row.line_type <> 'choice' AND EXISTS (
        SELECT 1
        FROM galgame_choice choice_row
        WHERE choice_row.line_id = line_row.id
    ) THEN
        RAISE EXCEPTION '非 choice Galgame 行不能保留 galgame_choice 选项';
    END IF;
    IF line_row.line_type = 'choice' THEN
        SELECT COUNT(*),
               COUNT(DISTINCT choice_row.option_index),
               MIN(choice_row.option_index),
               MAX(choice_row.option_index)
          INTO choice_count,
               choice_distinct_index_count,
               choice_min_option_index,
               choice_max_option_index
        FROM galgame_choice choice_row
        WHERE choice_row.line_id = line_row.id;
        IF choice_count NOT BETWEEN 2 AND 3 THEN
            RAISE EXCEPTION 'Galgame choice 行选项数量必须是 2 到 3 个';
        END IF;
        IF choice_distinct_index_count <> choice_count
           OR choice_min_option_index <> 1
           OR choice_max_option_index <> choice_count THEN
            RAISE EXCEPTION 'Galgame 选项 option_index 必须从 1 连续编号';
        END IF;
    END IF;

    RETURN NEW;
END;
$$
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_family_insert
AFTER INSERT ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_family_update
AFTER UPDATE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_order_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    affected_scene_ids BIGINT[];
    check_scene_id BIGINT;
    line_count BIGINT;
    line_distinct_index_count BIGINT;
    line_min_index INTEGER;
    line_max_index INTEGER;
BEGIN
    IF TG_OP = 'INSERT' THEN
        affected_scene_ids := ARRAY[NEW.scene_id];
    ELSIF TG_OP = 'DELETE' THEN
        affected_scene_ids := ARRAY[OLD.scene_id];
    ELSIF OLD.scene_id IS DISTINCT FROM NEW.scene_id THEN
        affected_scene_ids := ARRAY[OLD.scene_id, NEW.scene_id];
    ELSE
        affected_scene_ids := ARRAY[NEW.scene_id];
    END IF;

    FOREACH check_scene_id IN ARRAY affected_scene_ids LOOP
        SELECT COUNT(*),
               COUNT(DISTINCT line.line_index),
               MIN(line.line_index),
               MAX(line.line_index)
          INTO line_count,
               line_distinct_index_count,
               line_min_index,
               line_max_index
        FROM galgame_line line
        WHERE line.scene_id = check_scene_id;

        IF line_count = 0 THEN
            CONTINUE;
        END IF;
        IF line_distinct_index_count <> line_count
           OR line_min_index <> 1
           OR line_max_index <> line_count THEN
            RAISE EXCEPTION 'Galgame 行 line_index 必须在同一场内从 1 连续编号且不重复';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_insert
AFTER INSERT ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_order_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_update
AFTER UPDATE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_order_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_delete
AFTER DELETE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_order_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_choice_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    line_type_value TEXT;
    line_script_id BIGINT;
    target_entry_kind TEXT;
    target_next_scene_id BIGINT;
    return_entry_kind TEXT;
    affected_line_ids BIGINT[];
    check_line_id BIGINT;
    choice_count BIGINT;
    choice_distinct_index_count BIGINT;
    choice_min_option_index INTEGER;
    choice_max_option_index INTEGER;
BEGIN
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        SELECT line.line_type,
               scene.script_id
          INTO line_type_value,
               line_script_id
        FROM galgame_line line
        JOIN galgame_scene scene ON scene.id = line.scene_id
        WHERE line.id = NEW.line_id;

        IF line_type_value IS NULL THEN
            RAISE EXCEPTION 'Galgame 选项 line_id 不存在';
        END IF;
        IF line_type_value <> 'choice' THEN
            RAISE EXCEPTION 'Galgame 选项只能挂在 choice 行下';
        END IF;
        IF NEW.option_index <= 0 THEN
            RAISE EXCEPTION 'Galgame 选项 option_index 必须是正整数';
        END IF;
        IF BTRIM(NEW.option_text) = '' THEN
            RAISE EXCEPTION 'Galgame 选项 option_text 不能为空';
        END IF;
        IF NEW.target_scene_id IS NULL THEN
            RAISE EXCEPTION 'Galgame 选项 target_scene_id 不能为空';
        END IF;

        SELECT target.entry_kind,
               target.next_scene_id
          INTO target_entry_kind,
               target_next_scene_id
        FROM galgame_scene target
        WHERE target.id = NEW.target_scene_id
          AND target.script_id = line_script_id;

        IF target_entry_kind IS NULL THEN
            RAISE EXCEPTION 'Galgame 选项 target_scene_id 必须属于同一剧本';
        END IF;

        IF target_entry_kind = 'branch' THEN
            IF target_next_scene_id IS NULL THEN
                RAISE EXCEPTION 'Galgame 选项 target_scene_id 指向未回收敛支线';
            END IF;
            SELECT return_scene.entry_kind INTO return_entry_kind
            FROM galgame_scene return_scene
            WHERE return_scene.id = target_next_scene_id
              AND return_scene.script_id = line_script_id;
            IF return_entry_kind IS DISTINCT FROM 'main' THEN
                RAISE EXCEPTION 'Galgame 选项支线回收敛目标必须是同剧本主线场';
            END IF;
        END IF;
    END IF;

    IF TG_OP = 'INSERT' THEN
        affected_line_ids := ARRAY[NEW.line_id];
    ELSIF TG_OP = 'DELETE' THEN
        affected_line_ids := ARRAY[OLD.line_id];
    ELSIF OLD.line_id IS DISTINCT FROM NEW.line_id THEN
        affected_line_ids := ARRAY[OLD.line_id, NEW.line_id];
    ELSE
        affected_line_ids := ARRAY[NEW.line_id];
    END IF;

    FOREACH check_line_id IN ARRAY affected_line_ids LOOP
        SELECT line.line_type INTO line_type_value
        FROM galgame_line line
        WHERE line.id = check_line_id;

        IF line_type_value IS DISTINCT FROM 'choice' THEN
            CONTINUE;
        END IF;

        SELECT COUNT(*),
               COUNT(DISTINCT choice_row.option_index),
               MIN(choice_row.option_index),
               MAX(choice_row.option_index)
          INTO choice_count,
               choice_distinct_index_count,
               choice_min_option_index,
               choice_max_option_index
        FROM galgame_choice choice_row
        WHERE choice_row.line_id = check_line_id;

        IF choice_count NOT BETWEEN 2 AND 3 THEN
            RAISE EXCEPTION 'Galgame choice 行选项数量必须是 2 到 3 个';
        END IF;
        IF choice_distinct_index_count <> choice_count
           OR choice_min_option_index <> 1
           OR choice_max_option_index <> choice_count THEN
            RAISE EXCEPTION 'Galgame 选项 option_index 必须从 1 连续编号';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_insert
AFTER INSERT ON galgame_choice
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_choice_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_update
AFTER UPDATE ON galgame_choice
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_choice_family()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_delete
AFTER DELETE ON galgame_choice
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_choice_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_media_binding()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.target_kind NOT IN ('galgame_scene', 'galgame_line') THEN
        RETURN NEW;
    END IF;

    IF NEW.target_kind = 'galgame_scene' AND NEW.role <> 'background' THEN
        RAISE EXCEPTION 'Galgame 场媒体绑定只允许 background 角色';
    END IF;
    IF NEW.target_kind = 'galgame_line' AND NEW.role NOT IN ('cg', 'bgm', 'sfx') THEN
        RAISE EXCEPTION 'Galgame 行媒体绑定只允许 cg/bgm/sfx 角色';
    END IF;

    IF NEW.target_kind = 'galgame_scene' AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN galgame_scene scene ON scene.id = NEW.target_id
        JOIN galgame_script script ON script.id = scene.script_id
        LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
        WHERE asset.id = NEW.media_asset_id
          AND asset.project_id = script.project_id
          AND asset.root_project_id = script.root_project_id
          AND asset.source_snapshot_id = script.source_snapshot_id
          AND asset.media_kind = 'image'
          AND asset.status = 'completed'
          AND asset.file_path IS NOT NULL
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR asset.source_snapshot_chapter_id = scene.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR chapter.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 场媒体绑定必须匹配目标项目、媒体类型和源快照上下文';
    END IF;

    IF NEW.target_kind = 'galgame_line' AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN galgame_line line ON line.id = NEW.target_id
        JOIN galgame_scene scene ON scene.id = line.scene_id
        JOIN galgame_script script ON script.id = scene.script_id
        LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
        WHERE asset.id = NEW.media_asset_id
          AND asset.project_id = script.project_id
          AND asset.root_project_id = script.root_project_id
          AND asset.source_snapshot_id = script.source_snapshot_id
          AND (
              (NEW.role = 'cg' AND asset.media_kind = 'image')
              OR (NEW.role IN ('bgm', 'sfx') AND asset.media_kind = 'audio')
          )
          AND asset.status = 'completed'
          AND asset.file_path IS NOT NULL
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR asset.source_snapshot_chapter_id = scene.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR chapter.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 行媒体绑定必须匹配目标项目、媒体类型和源快照上下文';
    END IF;

    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_galgame_media_binding_insert
BEFORE INSERT ON media_binding
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_media_binding()
"#,
    r#"
CREATE TRIGGER trg_galgame_media_binding_update
BEFORE UPDATE OF media_asset_id, target_kind, target_id, role ON media_binding
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_media_binding()
"#,
    crate::migrations::galgame_target_reverse_integrity_contract::GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_script_delete
AFTER DELETE ON galgame_script
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_scene_update
AFTER UPDATE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_scene_delete
AFTER DELETE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_line_update
AFTER UPDATE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_line_delete
AFTER DELETE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_video_segment_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_source_mode TEXT;
BEGIN
    SELECT project_row.source_mode INTO project_source_mode
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id
      AND project_row.project_kind = 'video';

    IF project_source_mode = 'freeform' THEN
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 视频片段 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_chapter_id IS NOT NULL OR NEW.source_snapshot_chapter_id IS NOT NULL THEN
            RAISE EXCEPTION 'freeform 视频片段不能绑定源章节或源快照章节';
        END IF;
    ELSE
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND project_row.project_kind = 'video'
              AND project_row.source_mode = 'derived'
              AND project_row.root_project_id = NEW.root_project_id
        ) THEN
            RAISE EXCEPTION '视频片段 project_id 必须指向派生视频项目';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.root_project_id = NEW.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '视频片段 source_snapshot_chapter_id 必须属于根文本快照章节';
        END IF;
    END IF;
    IF NEW.source_generation_spec_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM generation_spec spec
        WHERE spec.id = NEW.source_generation_spec_id
          AND spec.project_id = NEW.project_id
          AND spec.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '视频片段 source_generation_spec_id 必须属于同一视频项目';
    END IF;
    IF NEW.result_media_asset_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM media_asset asset
        WHERE asset.id = NEW.result_media_asset_id
          AND asset.project_id = NEW.project_id
          AND asset.root_project_id = NEW.root_project_id
          AND asset.media_kind = 'video'
    ) THEN
        RAISE EXCEPTION '视频片段 result_media_asset_id 必须是同项目视频资产';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_video_segment_family_insert
BEFORE INSERT ON video_segment
FOR EACH ROW EXECUTE FUNCTION a4n_validate_video_segment_family()
"#,
    r#"
CREATE TRIGGER trg_video_segment_family_update
BEFORE UPDATE OF project_id, root_project_id, source_chapter_id, source_snapshot_chapter_id, source_generation_spec_id, result_media_asset_id ON video_segment
FOR EACH ROW EXECUTE FUNCTION a4n_validate_video_segment_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_assembly_timeline_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_source_mode TEXT;
BEGIN
    SELECT project_row.source_mode INTO project_source_mode
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id
      AND project_row.project_kind = 'video';

    IF project_source_mode = 'freeform' THEN
        IF NEW.source_chapter_id IS NOT NULL OR NEW.source_snapshot_chapter_id IS NOT NULL THEN
            RAISE EXCEPTION 'freeform 成片时间线不能绑定源章节或源快照章节';
        END IF;
    ELSIF project_source_mode = 'derived' THEN
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND project_row.project_kind = 'video'
              AND project_row.source_mode = 'derived'
              AND project_row.root_project_id IS NOT NULL
        ) THEN
            RAISE EXCEPTION '成片时间线 project_id 必须指向视频项目';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1
            FROM novel_project project_row
            JOIN source_snapshot_chapter chapter ON chapter.id = NEW.source_snapshot_chapter_id
            WHERE project_row.id = NEW.project_id
              AND chapter.root_project_id = project_row.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '成片时间线 source_snapshot_chapter_id 必须属于视频项目根文本';
        END IF;
    ELSE
        RAISE EXCEPTION '成片时间线 project_id 必须指向视频项目';
    END IF;
    IF NEW.preview_media_asset_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM media_asset asset
        WHERE asset.id = NEW.preview_media_asset_id
          AND asset.project_id = NEW.project_id
          AND asset.media_kind = 'video'
          AND asset.origin = 'assembled_preview'
          AND asset.status = 'completed'
    ) THEN
        RAISE EXCEPTION '成片时间线 preview_media_asset_id 必须是同项目成片预览资产';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_assembly_timeline_family_insert
BEFORE INSERT ON assembly_timeline
FOR EACH ROW EXECUTE FUNCTION a4n_validate_assembly_timeline_family()
"#,
    r#"
CREATE TRIGGER trg_assembly_timeline_family_update
BEFORE UPDATE OF project_id, source_chapter_id, source_snapshot_chapter_id, preview_media_asset_id ON assembly_timeline
FOR EACH ROW EXECUTE FUNCTION a4n_validate_assembly_timeline_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_assembly_clip_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM assembly_timeline timeline
        JOIN media_asset asset ON asset.id = NEW.media_asset_id
        WHERE timeline.id = NEW.timeline_id
          AND asset.project_id = timeline.project_id
          AND asset.media_kind = 'video'
          AND asset.status = 'completed'
    ) THEN
        RAISE EXCEPTION '时间线片段必须引用同视频项目已完成视频资产';
    END IF;
    IF NEW.source_segment_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM assembly_timeline timeline
        JOIN video_segment segment ON segment.id = NEW.source_segment_id
        WHERE timeline.id = NEW.timeline_id
          AND segment.project_id = timeline.project_id
    ) THEN
        RAISE EXCEPTION '时间线片段 source_segment_id 必须属于同一视频项目';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_assembly_clip_family_insert
BEFORE INSERT ON assembly_clip
FOR EACH ROW EXECUTE FUNCTION a4n_validate_assembly_clip_family()
"#,
    r#"
CREATE TRIGGER trg_assembly_clip_family_update
BEFORE UPDATE OF timeline_id, media_asset_id, source_segment_id ON assembly_clip
FOR EACH ROW EXECUTE FUNCTION a4n_validate_assembly_clip_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_source_sync_run_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM novel_project derived
        WHERE derived.id = NEW.derived_project_id
          AND derived.project_kind <> 'text'
          AND derived.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '源同步运行 derived_project_id 必须属于 root_project_id';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.from_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '源同步运行 from_snapshot_id 必须属于根文本项目';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.to_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '源同步运行 to_snapshot_id 必须属于根文本项目';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_source_sync_run_family_insert
BEFORE INSERT ON source_sync_run
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_sync_run_family()
"#,
    r#"
CREATE TRIGGER trg_source_sync_run_family_update
BEFORE UPDATE OF derived_project_id, root_project_id, from_snapshot_id, to_snapshot_id ON source_sync_run
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_sync_run_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_source_sync_conflict_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM source_sync_run sync_run
        WHERE sync_run.id = NEW.sync_run_id
          AND sync_run.derived_project_id = NEW.derived_project_id
          AND sync_run.root_project_id = NEW.root_project_id
          AND sync_run.from_snapshot_id = NEW.from_snapshot_id
          AND sync_run.to_snapshot_id = NEW.to_snapshot_id
    ) THEN
        RAISE EXCEPTION '源同步冲突必须与同步运行项目族字段一致';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_source_sync_conflict_family_insert
BEFORE INSERT ON source_sync_conflict
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_sync_conflict_family()
"#,
    r#"
CREATE TRIGGER trg_source_sync_conflict_family_update
BEFORE UPDATE OF sync_run_id, derived_project_id, root_project_id, from_snapshot_id, to_snapshot_id ON source_sync_conflict
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_sync_conflict_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_source_sync_decision_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.decided_by NOT IN ('user','system') THEN
        RAISE EXCEPTION '源同步决策 decided_by 无效';
    END IF;
    IF NEW.strategy NOT IN ('keep_local','adopt_source','rebuild_prompt','rebind_anchor','create_replacement','manual','detach','delete_local') THEN
        RAISE EXCEPTION '源同步决策 strategy 无效';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM source_sync_conflict sync_conflict
        WHERE sync_conflict.id = NEW.conflict_id
          AND sync_conflict.status IN ('resolved','ignored')
          AND sync_conflict.resolution_strategy = NEW.strategy
    ) THEN
        RAISE EXCEPTION '源同步决策必须指向已按同一策略处理的冲突';
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_source_sync_decision_family_insert
BEFORE INSERT ON source_sync_decision
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_sync_decision_family()
"#,
    r#"
CREATE TRIGGER trg_source_sync_decision_family_update
BEFORE UPDATE OF conflict_id, decided_by, strategy ON source_sync_decision
FOR EACH ROW EXECUTE FUNCTION a4n_validate_source_sync_decision_family()
"#,
];

const TEXT_SOURCE_SYNC_SQL: &[&str] = &[
    r#"
CREATE OR REPLACE FUNCTION a4n_mark_text_source_project_changed()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    changed_project_id bigint;
BEGIN
    IF TG_OP = 'DELETE' THEN
        changed_project_id := OLD.project_id;
    ELSE
        changed_project_id := NEW.project_id;
    END IF;

    IF EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = changed_project_id
          AND root.project_kind = 'text'
    ) THEN
        UPDATE novel_project
        SET source_sync_status = 'root_updated',
            updated_at = timezone('UTC', now())
        WHERE root_project_id = changed_project_id
          AND source_sync_status = 'current';
    END IF;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#,
    r#"
CREATE TRIGGER trg_chapter_text_source_changed_insert
AFTER INSERT ON chapter
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_chapter_text_source_changed_update
AFTER UPDATE OF title, content, summary, analysis_data ON chapter
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_chapter_text_source_changed_delete
AFTER DELETE ON chapter
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_chapter_outline_text_source_changed_insert
AFTER INSERT ON chapter_outline
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_chapter_outline_text_source_changed_update
AFTER UPDATE ON chapter_outline
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_chapter_outline_text_source_changed_delete
AFTER DELETE ON chapter_outline
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_part_outline_text_source_changed_insert
AFTER INSERT ON part_outline
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_part_outline_text_source_changed_update
AFTER UPDATE ON part_outline
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_part_outline_text_source_changed_delete
AFTER DELETE ON part_outline
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_blueprint_version_text_source_changed_insert
AFTER INSERT ON blueprint_version
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_blueprint_version_text_source_changed_update
AFTER UPDATE ON blueprint_version
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_blueprint_version_text_source_changed_delete
AFTER DELETE ON blueprint_version
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_protagonist_profile_text_source_changed_insert
AFTER INSERT ON protagonist_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_protagonist_profile_text_source_changed_update
AFTER UPDATE ON protagonist_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_protagonist_profile_text_source_changed_delete
AFTER DELETE ON protagonist_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_object_profile_text_source_changed_insert
AFTER INSERT ON object_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_object_profile_text_source_changed_update
AFTER UPDATE ON object_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_object_profile_text_source_changed_delete
AFTER DELETE ON object_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_scene_profile_text_source_changed_insert
AFTER INSERT ON scene_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_scene_profile_text_source_changed_update
AFTER UPDATE ON scene_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
    r#"
CREATE TRIGGER trg_scene_profile_text_source_changed_delete
AFTER DELETE ON scene_profile
FOR EACH ROW EXECUTE FUNCTION a4n_mark_text_source_project_changed()
"#,
];

#[cfg(test)]
const PROJECT_FAMILY_TRIGGER_BASE_NAMES: &[&str] = &[
    "trg_novel_project_family",
    "trg_project_source_snapshot_family",
    "trg_source_snapshot_chapter_family",
    "trg_text_anchor_family",
    "trg_project_derivation_family",
    "trg_generation_spec_family",
    "trg_generation_spec_text_anchor_family",
    "trg_generation_spec_entity_ref_family",
    "trg_media_asset_family",
    "trg_generation_spec_media_ref_family",
    "trg_media_provenance_family",
    "trg_galgame_script_family",
    "trg_galgame_scene_family",
    "trg_galgame_scene_link_family",
    "trg_galgame_line_family",
    "trg_galgame_line_order_family",
    "trg_galgame_choice_family",
    "trg_galgame_media_binding",
    "trg_video_segment_family",
    "trg_assembly_timeline_family",
    "trg_assembly_clip_family",
    "trg_source_sync_run_family",
    "trg_source_sync_conflict_family",
    "trg_source_sync_decision_family",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_complete_project_family_integrity_trigger_set() {
        let function_count = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .filter(|sql| sql.contains("CREATE OR REPLACE FUNCTION "))
            .count();
        let trigger_count = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .filter(|sql| {
                sql.contains("CREATE TRIGGER ") || sql.contains("CREATE CONSTRAINT TRIGGER ")
            })
            .count();

        assert_eq!(
            function_count,
            PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES.len()
        );
        assert_eq!(
            trigger_count,
            PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES.len() * 2 + 6
        );

        for function_name in PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES {
            let declaration = format!("CREATE OR REPLACE FUNCTION {function_name}()");
            assert!(
                PROJECT_FAMILY_INTEGRITY_SQL
                    .iter()
                    .any(|sql| sql.contains(&declaration)),
                "missing trigger function declaration: {function_name}"
            );
        }

        for trigger_base in PROJECT_FAMILY_TRIGGER_BASE_NAMES {
            for suffix in ["insert", "update"] {
                let trigger_name = format!("CREATE TRIGGER {trigger_base}_{suffix}");
                let constraint_trigger_name =
                    format!("CREATE CONSTRAINT TRIGGER {trigger_base}_{suffix}");
                assert!(
                    PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
                        sql.contains(&trigger_name) || sql.contains(&constraint_trigger_name)
                    }),
                    "missing project family trigger: {trigger_base}_{suffix}"
                );
            }
        }
    }

    #[test]
    fn baseline_choice_family_trigger_enforces_group_contract() {
        let line_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_family()")
            })
            .expect("Galgame line family trigger function");
        let choice_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_choice_family()")
            })
            .expect("Galgame choice family trigger function");

        for expected in [
            "Galgame choice 行选项数量必须是 2 到 3 个",
            "Galgame 选项 option_index 必须从 1 连续编号",
        ] {
            assert!(
                line_family_sql.contains(expected) && choice_family_sql.contains(expected),
                "missing baseline Galgame choice group contract: {expected}"
            );
        }
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
                sql.contains("CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_delete")
                    && sql.contains("AFTER DELETE ON galgame_choice")
            }),
            "missing baseline Galgame choice delete trigger"
        );
    }

    #[test]
    fn baseline_scene_family_trigger_enforces_participants_contract() {
        let scene_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_family()")
            })
            .expect("Galgame scene family trigger function");

        for expected in [
            "participants_json 项必须是对象",
            "participants_json.character_profile_id 必须是正整数",
            "participants_json.character_profile_id 必须属于剧本根项目",
            "jsonb_array_elements(scene_row.participants_json::jsonb)",
            "protagonist_profile profile",
        ] {
            assert!(
                scene_family_sql.contains(expected),
                "missing baseline Galgame scene participants contract: {expected}"
            );
        }
    }

    #[test]
    fn baseline_scene_link_family_trigger_enforces_graph_contract() {
        let scene_link_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_link_family()")
            })
            .expect("Galgame scene link family trigger function");

        for expected in [
            "Galgame 场 next_scene_id 不能指向自身",
            "Galgame 场 next_scene_id 必须属于同一剧本",
            "Galgame branch 场缺少 next_scene_id",
            "Galgame branch 场 next_scene_id 必须指向主线场",
        ] {
            assert!(
                scene_link_sql.contains(expected),
                "missing baseline Galgame scene link contract: {expected}"
            );
        }
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
                sql.contains("CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_delete")
                    && sql.contains("AFTER DELETE ON galgame_scene")
            }),
            "missing baseline Galgame scene link delete trigger"
        );
    }

    #[test]
    fn baseline_line_order_family_trigger_enforces_group_contract() {
        let line_order_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_order_family()")
            })
            .expect("Galgame line order family trigger function");

        assert!(
            line_order_sql.contains("Galgame 行 line_index 必须在同一场内从 1 连续编号且不重复"),
            "missing baseline Galgame line order group contract"
        );
        for expected in [
            "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_insert",
            "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_update",
            "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_delete",
        ] {
            assert!(
                PROJECT_FAMILY_INTEGRITY_SQL
                    .iter()
                    .any(|sql| sql.contains(expected)),
                "missing baseline Galgame line order trigger: {expected}"
            );
        }
    }

    #[test]
    fn baseline_line_family_trigger_enforces_event_cg_presentation_contract() {
        let line_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_family()")
            })
            .expect("Galgame line family trigger function");

        for expected in [
            "presentation_json.cg 必须是布尔值",
            "presentation_json.cg_importance 必须是字符串",
            "presentation_json.cg_importance 无效",
            "启用事件 CG 时必须提供 cg_note",
        ] {
            assert!(
                line_family_sql.contains(expected),
                "missing baseline Galgame line trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn baseline_generation_spec_family_trigger_enforces_galgame_asset_contract() {
        let generation_spec_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_family()")
            })
            .expect("generation_spec family trigger function");

        for expected in [
            "生成意图 params_json 必须是 JSON 对象",
            "Galgame 生成意图 target_kind/target_entity_kind 配对无效",
            "Galgame 生成意图 target_entity_id 必填",
            "Galgame 背景生成意图 source_snapshot_chapter_id 与生成意图不一致",
            "Galgame CG 生成意图 scene.id 与目标行所属场不一致",
            "Galgame 立绘差分目标角色必须属于源快照",
            "Galgame 立绘差分 source_line_ids 行上下文不一致",
            "Galgame 立绘差分 position_samples 与 source_line_ids 行站位不一致",
            "生成意图已有媒体引用必须属于更新后的同一项目族且指向已完成媒体资产",
            "生成意图已有媒体来源必须匹配更新后的项目族和源快照上下文",
            "生成意图已有提示词媒体来源必须匹配更新后的项目族和源快照上下文",
            "Galgame 生成意图已有媒体来源 request_snapshot_json 必须匹配更新后的目标",
        ] {
            assert!(
                generation_spec_sql.contains(expected),
                "missing baseline Galgame generation_spec trigger contract: {expected}"
            );
        }
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
                sql.contains("CREATE TRIGGER trg_generation_spec_family_update")
                    && sql.contains("target_kind")
                    && sql.contains("target_entity_kind")
                    && sql.contains("target_entity_id")
                    && sql.contains("params_json ON generation_spec")
            }),
            "baseline generation_spec update trigger must cover Galgame target and params columns"
        );
    }

    #[test]
    fn baseline_generation_spec_text_anchor_trigger_enforces_chapter_scope() {
        let text_anchor_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains(
                    "CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_text_anchor_family()",
                )
            })
            .expect("generation_spec text_anchor family trigger function");

        for expected in [
            "生成意图文本锚点必须属于同一源快照章节",
            "生成意图文本锚点 source_chapter_id 必须与生成意图一致",
        ] {
            assert!(
                text_anchor_sql.contains(expected),
                "missing baseline generation_spec text_anchor chapter contract: {expected}"
            );
        }
    }

    #[test]
    fn baseline_generation_spec_entity_ref_trigger_enforces_snapshot_profile_scope() {
        let entity_ref_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains(
                    "CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_entity_ref_family()",
                )
            })
            .expect("generation_spec entity_ref family trigger function");

        for expected in [
            "生成意图实体引用 entity_kind 无效",
            "生成意图实体引用 role 无效",
            "事件档案尚未建立持久模型，不能作为生成意图实体引用",
            "生成意图实体引用必须属于生成意图源快照",
        ] {
            assert!(
                entity_ref_sql.contains(expected),
                "missing baseline generation_spec entity_ref contract: {expected}"
            );
        }
        for expected in [
            "CREATE TRIGGER trg_generation_spec_entity_ref_family_insert",
            "CREATE TRIGGER trg_generation_spec_entity_ref_family_update",
        ] {
            assert!(
                PROJECT_FAMILY_INTEGRITY_SQL
                    .iter()
                    .any(|sql| sql.contains(expected)),
                "missing baseline generation_spec entity_ref trigger: {expected}"
            );
        }
    }

    #[test]
    fn baseline_media_asset_trigger_enforces_paired_chapter_context() {
        let media_asset_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_media_asset_family()")
            })
            .expect("media asset family trigger function");

        for expected in [
            "source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空",
            "chapter.source_chapter_id = NEW.source_chapter_id",
            "completed 媒体资产必须有 file_path",
            "媒体资产更新会破坏已有生成意图媒体引用",
            "媒体资产更新会破坏已有 Galgame 场媒体绑定",
            "媒体资产更新会破坏已有 Galgame 行媒体绑定",
            "媒体资产更新会破坏已有 generation_spec 媒体来源",
            "媒体资产更新会破坏已有 prompt_artifact 媒体来源",
            "媒体资产更新会破坏已有 source_media_asset 媒体来源",
            "媒体资产更新会破坏引用它的媒体来源",
        ] {
            assert!(
                media_asset_sql.contains(expected),
                "missing baseline media asset trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn baseline_generation_spec_media_ref_trigger_enforces_role_and_asset_readiness() {
        let media_ref_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains(
                    "CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_media_ref_family()",
                )
            })
            .expect("generation spec media ref family trigger function");

        for expected in [
            "生成意图媒体引用 role 无效",
            "'character_reference'",
            "'style_reference'",
            "asset.status = 'completed'",
            "asset.file_path IS NOT NULL",
            "生成意图媒体引用必须使用同项目族已完成媒体资产",
        ] {
            assert!(
                media_ref_sql.contains(expected),
                "missing baseline generation_spec media_ref trigger contract: {expected}"
            );
        }
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
                sql.contains("CREATE TRIGGER trg_generation_spec_media_ref_family_update")
                    && sql.contains(
                        "generation_spec_id, media_asset_id, role ON generation_spec_media_ref",
                    )
            }),
            "baseline generation_spec_media_ref update trigger must cover role"
        );
    }

    #[test]
    fn baseline_media_provenance_trigger_enforces_galgame_request_snapshot_contract() {
        let media_provenance_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_media_provenance_family()")
            })
            .expect("media provenance family trigger function");

        for expected in [
            "request_snapshot_json 必须是 JSON 对象",
            "request_snapshot_json.source 必须是 galgame_asset_generation",
            "request_snapshot_json.target_entity_kind 缺失",
            "request_snapshot_json.target_entity_id 缺失",
            "request_snapshot_json 与 generation_spec 目标不一致",
            "request_snapshot_json.target_line_id 缺失",
            "request_snapshot_json.target_line_id 必须是整数",
            "request_snapshot_json.target_line_id 必须指向同项目 Galgame 行",
            "source.status = 'completed'",
            "source.file_path IS NOT NULL",
        ] {
            assert!(
                media_provenance_sql.contains(expected),
                "missing baseline media provenance trigger contract: {expected}"
            );
        }
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
                sql.contains("CREATE TRIGGER trg_media_provenance_family_update")
                    && sql.contains("request_snapshot_json ON media_provenance")
            }),
            "baseline media provenance update trigger must cover request_snapshot_json"
        );
    }

    #[test]
    fn baseline_galgame_media_binding_trigger_requires_ready_media_asset() {
        let media_binding_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_media_binding()")
            })
            .expect("Galgame media binding trigger function");

        for expected in ["asset.status = 'completed'", "asset.file_path IS NOT NULL"] {
            assert!(
                media_binding_sql.contains(expected),
                "missing baseline Galgame media binding asset readiness contract: {expected}"
            );
        }
    }

    #[test]
    fn baseline_galgame_target_reverse_integrity_contract_is_installed() {
        let reverse_sql = PROJECT_FAMILY_INTEGRITY_SQL
            .iter()
            .find(|sql| {
                sql.contains(
                    "CREATE OR REPLACE FUNCTION a4n_validate_galgame_target_reverse_integrity()",
                )
            })
            .expect("Galgame target reverse integrity trigger function");

        for expected in [
            "Galgame 背景生成意图必须随目标场更新或清理",
            "Galgame CG 生成意图必须随目标行更新或清理",
            "Galgame 立绘差分生成意图必须随来源行更新或清理",
            "Galgame 场媒体绑定必须随目标场更新或清理",
            "Galgame 行媒体绑定必须随目标行更新或清理",
            "Galgame 音频媒体来源 target_line_id 必须随目标行更新或清理",
        ] {
            assert!(
                reverse_sql.contains(expected),
                "missing baseline Galgame reverse integrity contract: {expected}"
            );
        }

        for expected_trigger in [
            "trg_galgame_target_reverse_script_delete",
            "trg_galgame_target_reverse_scene_update",
            "trg_galgame_target_reverse_scene_delete",
            "trg_galgame_target_reverse_line_update",
            "trg_galgame_target_reverse_line_delete",
        ] {
            assert!(
                PROJECT_FAMILY_INTEGRITY_SQL
                    .iter()
                    .any(|sql| sql.contains(expected_trigger)
                        && sql.contains("DEFERRABLE INITIALLY DEFERRED")),
                "missing baseline Galgame reverse integrity trigger: {expected_trigger}"
            );
        }
    }

    #[test]
    fn declares_text_source_sync_triggers_for_snapshot_inputs() {
        let function_count = TEXT_SOURCE_SYNC_SQL
            .iter()
            .filter(|sql| sql.contains("CREATE OR REPLACE FUNCTION "))
            .count();
        let trigger_count = TEXT_SOURCE_SYNC_SQL
            .iter()
            .filter(|sql| sql.contains("CREATE TRIGGER "))
            .count();

        assert_eq!(function_count, TEXT_SOURCE_SYNC_FUNCTION_NAMES.len());
        assert_eq!(trigger_count, TEXT_SOURCE_SYNC_TRIGGER_BASE_NAMES.len() * 3);

        for function_name in TEXT_SOURCE_SYNC_FUNCTION_NAMES {
            let declaration = format!("CREATE OR REPLACE FUNCTION {function_name}()");
            assert!(
                TEXT_SOURCE_SYNC_SQL
                    .iter()
                    .any(|sql| sql.contains(&declaration)),
                "missing text source sync function declaration: {function_name}"
            );
        }

        for trigger_base in TEXT_SOURCE_SYNC_TRIGGER_BASE_NAMES {
            for suffix in ["insert", "update", "delete"] {
                let trigger_name = format!("CREATE TRIGGER {trigger_base}_{suffix}");
                assert!(
                    TEXT_SOURCE_SYNC_SQL
                        .iter()
                        .any(|sql| sql.contains(&trigger_name)),
                    "missing text source sync trigger: {trigger_base}_{suffix}"
                );
            }
        }
    }
}
