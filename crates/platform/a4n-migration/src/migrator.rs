//! 数据库 schema 初始化器。
//!
//! 新库通过 PostgreSQL baseline 建表；已部署数据库通过后续幂等迁移补齐新增表。

use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSchemaMigrations {
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMigrationPreflight {
    pub pending_schema_migrations: Vec<String>,
    pub pending_data_migrations: Vec<String>,
    pub migrations: Vec<crate::migrations::canvas_project_split::DataMigrationPreflight>,
}

pub struct Migrator;

#[sea_orm_migration::async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(crate::migrations::postgres_baseline::Migration),
            Box::new(crate::migrations::user_default_ai_config::Migration),
            Box::new(crate::migrations::invite_codes::Migration),
            Box::new(crate::migrations::generation_runs::Migration),
            Box::new(crate::migrations::llm_context_tracking::Migration),
            Box::new(crate::migrations::llm_call_trace_join_fields::Migration),
            Box::new(crate::migrations::llm_call_trace_status_fields::Migration),
            Box::new(crate::migrations::llm_request_options::Migration),
            Box::new(crate::migrations::llm_model_capabilities::Migration),
            Box::new(crate::migrations::llm_agent_protocol::Migration),
            Box::new(crate::migrations::object_profile_usage_fields::Migration),
            Box::new(crate::migrations::chapter_video_prompt_language_identity::Migration),
            Box::new(crate::migrations::shared_video_segment_plan::Migration),
            Box::new(crate::migrations::project_collaboration::Migration),
            Box::new(crate::migrations::workflow_runs::Migration),
            Box::new(crate::migrations::generation_run_checkpoint_fields::Migration),
            Box::new(crate::migrations::workflow_checkpoint_fingerprint_repair::Migration),
            Box::new(crate::migrations::media_provider_options::Migration),
            Box::new(crate::migrations::video_canvas_workflow::Migration),
            Box::new(crate::migrations::canvas_assemble_edge_role::Migration),
            Box::new(crate::migrations::canvas_assemble_clip_order::Migration),
            Box::new(crate::migrations::freeform_generation_context::Migration),
            Box::new(crate::migrations::freeform_canonical_media::Migration),
            Box::new(crate::migrations::media_asset_snapshot_context::Migration),
            Box::new(crate::migrations::media_provenance_snapshot_context::Migration),
            Box::new(crate::migrations::galgame_script_tables::Migration),
            Box::new(crate::migrations::galgame_script_integrity_contract::Migration),
            Box::new(crate::migrations::galgame_generation_spec_contract::Migration),
            Box::new(crate::migrations::galgame_media_binding_contract::Migration),
            Box::new(crate::migrations::media_asset_generation_ref_contract::Migration),
            Box::new(crate::migrations::galgame_target_reverse_integrity_contract::Migration),
            Box::new(crate::migrations::cleanup_outbox::Migration),
            Box::new(crate::migrations::directing_workflow_v2::Migration),
            Box::new(crate::migrations::chapter_shooting_script::Migration),
            Box::new(crate::migrations::shooting_script_narrative_units::Migration),
            Box::new(crate::migrations::shooting_script_staged_generation::Migration),
            Box::new(crate::migrations::video_execution_contract_v5_reset::Migration),
            Box::new(crate::migrations::video_workflow_exclusive_claim::Migration),
            Box::new(crate::migrations::video_workflow_task_binding::Migration),
            Box::new(crate::migrations::neutral_video_segment_plan::Migration),
            Box::new(
                crate::migrations::shooting_script_transition_type_canonicalization::Migration,
            ),
            Box::new(crate::migrations::generation_task_persistence::Migration),
            Box::new(crate::migrations::cleanup_outbox_generation_scope::Migration),
            Box::new(crate::migrations::generation_snapshot_retention::Migration),
            Box::new(crate::migrations::generation_snapshot_hash_contract::Migration),
            Box::new(crate::migrations::segment_card_video_workflow::Migration),
            Box::new(crate::migrations::segment_execution_recipe_identity::Migration),
            Box::new(crate::migrations::segment_recipe_adoption_repair::Migration),
            Box::new(crate::migrations::chapter_video_prompt_execution_identity::Migration),
            Box::new(crate::migrations::drop_chapter_video_prompt_identity_unique::Migration),
            Box::new(crate::migrations::unified_model_provider_requests::Migration),
            Box::new(crate::migrations::system_settings_key_unique::Migration),
            Box::new(crate::migrations::chapter_outline_provenance::Migration),
            Box::new(crate::migrations::character_timeline_provenance::Migration),
            Box::new(crate::migrations::chapter_video_timeline::Migration),
            Box::new(crate::migrations::chapter_video_timeline_cleanup::Migration),
            Box::new(crate::migrations::character_deletion_mark_provenance::Migration),
            Box::new(crate::migrations::profile_record_ownership::Migration),
            Box::new(crate::migrations::blueprint_version_snapshot_cleanup::Migration),
            Box::new(crate::migrations::canvas_node_ref_contract_cleanup::Migration),
            Box::new(crate::migrations::manga_page_prompt_contract_cleanup::Migration),
            Box::new(crate::migrations::manga_reference_binding_contract_cleanup::Migration),
            Box::new(crate::migrations::manga_reference_binding_payload_cleanup::Migration),
            Box::new(crate::migrations::theme_config_integrity_contract::Migration),
            Box::new(crate::migrations::provider_default_config_integrity_contract::Migration),
            Box::new(crate::migrations::galgame_presentation_contract_cleanup::Migration),
            Box::new(crate::migrations::chapter_video_timeline_retirement::Migration),
            Box::new(crate::migrations::import_review_contract_cleanup::Migration),
            Box::new(crate::migrations::foreshadowing_index_integrity_contract::Migration),
            Box::new(crate::migrations::chapter_video_timeline_contract::Migration),
            Box::new(crate::migrations::chapter_video_timeline_state_cleanup::Migration),
            Box::new(crate::migrations::blueprint_version_snapshot_contract::Migration),
            Box::new(crate::migrations::source_sync_regenerate_scene_strategy::Migration),
            Box::new(crate::migrations::shooting_script_transition_type_contract::Migration),
            Box::new(crate::migrations::project_derivation_identity_cleanup::Migration),
            Box::new(crate::migrations::project_source_mode_identity_cleanup::Migration),
            Box::new(crate::migrations::canvas_extract_frame_contract::Migration),
            Box::new(crate::migrations::canvas_trim_contract::Migration),
            Box::new(crate::migrations::external_import_model_identity_cleanup::Migration),
            Box::new(crate::migrations::chapter_video_prompt_output_language_contract::Migration),
            Box::new(crate::migrations::profile_record_ownership_default_cleanup::Migration),
            Box::new(crate::migrations::chapter_visual_plan::Migration),
            Box::new(crate::migrations::chapter_visual_plan_source_revision_identity::Migration),
            Box::new(crate::migrations::legacy_asset_lineage_shadow_cleanup::Migration),
            Box::new(crate::migrations::manga_page_layout_identity_cleanup::Migration),
            Box::new(crate::migrations::chapter_visual_plan_workflow_identity_cleanup::Migration),
            Box::new(crate::migrations::page_cli_enabled::Migration),
            Box::new(crate::migrations::theme_system_v2::Migration),
            Box::new(crate::migrations::theme_system_v3::Migration),
            Box::new(crate::migrations::theme_resource_size_limit_v2::Migration),
            Box::new(crate::migrations::theme_token_layer_cleanup::Migration),
            Box::new(crate::migrations::image_supported_sizes::Migration),
            Box::new(crate::migrations::agent_runtime::Migration),
            Box::new(crate::migrations::agent_replay_policy_canonicalization::Migration),
            Box::new(crate::migrations::agent_runtime_snapshot_canonicalization::Migration),
            Box::new(crate::migrations::agent_runtime_checkpoint_cleanup::Migration),
            Box::new(crate::migrations::agent_runtime_status_canonicalization::Migration),
            Box::new(crate::migrations::agent_runtime_dead_fields_cleanup::Migration),
            Box::new(crate::migrations::agent_runtime_run_activity_canonicalization::Migration),
            Box::new(crate::migrations::chapter_visual_plan_lineage_identity::Migration),
            Box::new(crate::migrations::chapter_visual_plan_authoring_mode_cleanup::Migration),
            Box::new(crate::migrations::image_publication::Migration),
            Box::new(crate::migrations::package_lifecycle::Migration),
            Box::new(crate::migrations::llm_max_tokens_setting_cleanup::Migration),
            Box::new(crate::migrations::llm_cache_usage::Migration),
            Box::new(crate::migrations::claude_cache_usage_repair::Migration),
            Box::new(crate::migrations::web_search_global_config_retirement::Migration),
            Box::new(crate::migrations::chapter_visual_plan_v2_retirement::Migration),
            Box::new(crate::migrations::model_cost_pricing::Migration),
            Box::new(crate::migrations::model_usage_ledger::Migration),
            Box::new(crate::migrations::model_usage_request_snapshot::Migration),
            Box::new(crate::migrations::generation_task_workflow_binding_repair::Migration),
            Box::new(crate::migrations::optimization_session_agent_unification::Migration),
            Box::new(crate::migrations::media_workflow_reset_retirement::Migration),
            Box::new(crate::migrations::optimization_turn_agent_idempotency::Migration),
            Box::new(crate::migrations::agent_history_contract_canonicalization::Migration),
            Box::new(crate::migrations::galgame_generation_spec_source_hash::Migration),
            Box::new(crate::migrations::video_shot_source_identity::Migration),
            Box::new(crate::migrations::body_fact_feedback::Migration),
            Box::new(crate::migrations::fact_feedback_summary_history_index::Migration),
            Box::new(crate::migrations::galgame_scene_source_identity::Migration),
            Box::new(crate::migrations::galgame_scene_snapshot_scope::Migration),
            Box::new(crate::migrations::retire_stale_live_text_anchors::Migration),
            Box::new(crate::migrations::source_sync_local_target_retirement::Migration),
            Box::new(crate::migrations::frozen_chapter_identity::Migration),
            Box::new(crate::migrations::shooting_script_timing_v2::Migration),
            Box::new(crate::migrations::workflow_chapter_identity::Migration),
            Box::new(crate::migrations::media_authoring::Migration),
            Box::new(crate::migrations::agent_project_binding::Migration),
            Box::new(crate::migrations::standalone_comic_project::Migration),
            Box::new(crate::migrations::canvas_text_asset_versions::Migration),
            Box::new(crate::migrations::standalone_comic_text_sources::Migration),
            Box::new(crate::migrations::canvas_project_split::Migration),
            Box::new(crate::migrations::galgame_story_graph::Migration),
            Box::new(crate::migrations::galgame_story_runtime_state::Migration),
            Box::new(crate::migrations::manga_pipeline_artifacts::Migration),
            Box::new(crate::migrations::manga_preprocessing::Migration),
            Box::new(crate::migrations::manga_preprocessing_contract_v2::Migration),
            Box::new(crate::migrations::manga_preprocessing_contract_v3::Migration),
            Box::new(crate::migrations::manga_page_image_artifacts::Migration),
            Box::new(crate::migrations::user_read_only_role_removal::Migration),
            Box::new(crate::migrations::creator_notifications::Migration),
            Box::new(crate::migrations::generated_image_project_index::Migration),
            Box::new(crate::migrations::generated_image_project_chapter_index::Migration),
            Box::new(crate::migrations::workflow_run_project_updated_index::Migration),
            Box::new(crate::migrations::task_queue_project_created_index::Migration),
            Box::new(crate::migrations::project_status_history_project_changed_index::Migration),
            Box::new(crate::migrations::workflow_run_business_updated_index::Migration),
            Box::new(crate::migrations::llm_call_trace_created_index::Migration),
            Box::new(crate::migrations::generation_run_created_index::Migration),
            Box::new(crate::migrations::model_usage_root_started_index::Migration),
            Box::new(crate::migrations::character_portrait_project_index::Migration),
            Box::new(crate::migrations::character_portrait_primary_profile_index::Migration),
            Box::new(crate::migrations::generated_video_project_chapter_index::Migration),
            Box::new(crate::migrations::task_queue_created_index::Migration),
        ]
    }
}

pub async fn run_migrations(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    Migrator::up(db, None).await
}

pub async fn pending_schema_migrations(
    db: &sea_orm::DatabaseConnection,
) -> Result<PendingSchemaMigrations, sea_orm::DbErr> {
    let applied = applied_schema_migration_names(db).await?;
    let mut names = Vec::new();
    for migration in Migrator::migrations() {
        let name = migration.name().to_string();
        if applied.contains(&name) {
            continue;
        }
        names.push(name);
    }
    Ok(PendingSchemaMigrations { names })
}

pub async fn applied_schema_migration_names(
    db: &sea_orm::DatabaseConnection,
) -> Result<std::collections::HashSet<String>, sea_orm::DbErr> {
    use sea_orm_migration::prelude::MigratorTrait;

    match Migrator::get_migration_models(db).await {
        Ok(migrations) => Ok(migrations
            .into_iter()
            .map(|migration| migration.version)
            .collect()),
        Err(error) => Err(error),
    }
}

pub fn data_migration_names() -> Vec<String> {
    use sea_orm_migration::prelude::MigratorTrait;

    Migrator::migrations()
        .into_iter()
        .map(|migration| migration.name().to_string())
        .filter(|name| migration_requires_data_backup(name))
        .collect()
}

pub async fn pending_data_migrations(
    db: &sea_orm::DatabaseConnection,
    names: &[String],
) -> Result<Vec<String>, sea_orm::DbErr> {
    let mut pending = Vec::new();
    for name in names {
        if !migration_requires_data_backup(name) {
            return Err(sea_orm::DbErr::Custom(format!(
                "未注册的数据迁移名称：{name}"
            )));
        }
        let is_pending = match name.as_str() {
            "canvas_project_split" => {
                crate::migrations::canvas_project_split::data_migration_is_pending(db).await?
            }
            _ => true,
        };
        if is_pending {
            pending.push(name.clone());
        }
    }
    Ok(pending)
}

pub fn migration_requires_data_backup(name: &str) -> bool {
    name == "canvas_project_split"
}

pub async fn preflight_data_migrations(
    db: &sea_orm::DatabaseConnection,
    data_dir: &std::path::Path,
    names: &[String],
) -> Result<Vec<crate::migrations::canvas_project_split::DataMigrationPreflight>, sea_orm::DbErr> {
    let mut reports = Vec::new();
    for name in names {
        if !migration_requires_data_backup(name) {
            return Err(sea_orm::DbErr::Custom(format!(
                "缺少数据迁移预检实现：{name}"
            )));
        }
        match name.as_str() {
            "canvas_project_split" => reports.push(
                crate::migrations::canvas_project_split::data_migration_preflight(db, data_dir)
                    .await?,
            ),
            _ => {
                return Err(sea_orm::DbErr::Custom(format!(
                    "缺少数据迁移预检实现：{name}"
                )));
            }
        }
    }
    Ok(reports)
}

pub async fn apply_data_migrations(
    db: &sea_orm::DatabaseConnection,
    data_dir: &std::path::Path,
    names: &[String],
) -> Result<(), sea_orm::DbErr> {
    for name in names {
        if !migration_requires_data_backup(name) {
            return Err(sea_orm::DbErr::Custom(format!(
                "未注册的数据迁移名称：{name}"
            )));
        }
        match name.as_str() {
            "canvas_project_split" => {
                crate::migrations::canvas_project_split::apply(db, data_dir).await?;
            }
            _ => {
                return Err(sea_orm::DbErr::Custom(format!(
                    "缺少数据迁移执行实现：{name}"
                )));
            }
        }
    }
    Ok(())
}

pub async fn preflight_update_migrations(
    database_url: &str,
    data_dir: &std::path::Path,
    data_migrations: &[String],
) -> Result<UpdateMigrationPreflight, sea_orm::DbErr> {
    let db = Database::connect(ConnectOptions::new(database_url.to_string())).await?;
    let pending_schema = pending_schema_migrations(&db).await?;
    let pending_data = pending_data_migrations(&db, data_migrations).await?;
    let migrations = preflight_data_migrations(&db, data_dir, &pending_data).await?;
    Ok(UpdateMigrationPreflight {
        pending_schema_migrations: pending_schema.names,
        pending_data_migrations: pending_data,
        migrations,
    })
}

#[cfg(test)]
mod tests {
    use super::{Migrator, migration_requires_data_backup};
    use a4n_db_models::REQUIRED_SCHEMA_MIGRATION;
    use sea_orm_migration::prelude::MigratorTrait;

    #[test]
    fn migrations_keep_user_default_ai_config_history_marker() {
        let names: Vec<_> = Migrator::migrations()
            .into_iter()
            .map(|migration| migration.name().to_string())
            .collect();

        assert!(
            names.iter().any(|name| name == "user_default_ai_config"),
            "deployed databases may already record user_default_ai_config"
        );
        assert!(
            names.iter().any(|name| name == "cleanup_outbox"),
            "project deletion cleanup outbox migration must remain registered"
        );
        assert!(
            names.iter().any(|name| name == "directing_workflow_v2"),
            "shared directing workflow migration must remain registered"
        );
        let cleanup_outbox_index = names
            .iter()
            .position(|name| name == "cleanup_outbox")
            .expect("cleanup_outbox migration must be registered");
        let directing_workflow_index = names
            .iter()
            .position(|name| name == "directing_workflow_v2")
            .expect("directing_workflow_v2 migration must be registered");
        assert!(
            cleanup_outbox_index < directing_workflow_index,
            "directing workflow migration enqueues cleanup intents and must run after cleanup_outbox"
        );
        let deployed_history = [
            "chapter_video_timeline",
            "chapter_video_timeline_cleanup",
            "character_deletion_mark_provenance",
            "profile_record_ownership",
            "blueprint_version_snapshot_cleanup",
            "canvas_node_ref_contract_cleanup",
            "manga_page_prompt_contract_cleanup",
            "manga_reference_binding_contract_cleanup",
            "manga_reference_binding_payload_cleanup",
            "theme_config_integrity_contract",
            "provider_default_config_integrity_contract",
            "galgame_presentation_contract_cleanup",
            "chapter_video_timeline_retirement",
            "import_review_contract_cleanup",
            "foreshadowing_index_integrity_contract",
            "chapter_video_timeline_contract",
            "chapter_video_timeline_state_cleanup",
            "blueprint_version_snapshot_contract",
            "source_sync_regenerate_scene_strategy",
            "shooting_script_transition_type_contract",
            "project_derivation_identity_cleanup",
            "project_source_mode_identity_cleanup",
            "canvas_extract_frame_contract",
            "canvas_trim_contract",
            "external_import_model_identity_cleanup",
            "chapter_video_prompt_output_language_contract",
            "profile_record_ownership_default_cleanup",
            "chapter_visual_plan",
            "chapter_visual_plan_source_revision_identity",
            "legacy_asset_lineage_shadow_cleanup",
            "manga_page_layout_identity_cleanup",
            "chapter_visual_plan_workflow_identity_cleanup",
            "page_cli_enabled",
            "theme_system_v2",
            "theme_system_v3",
            "theme_resource_size_limit_v2",
            "theme_token_layer_cleanup",
            "image_supported_sizes",
            "agent_runtime",
            "agent_replay_policy_canonicalization",
            "agent_runtime_snapshot_canonicalization",
            "agent_runtime_checkpoint_cleanup",
            "agent_runtime_status_canonicalization",
            "agent_runtime_dead_fields_cleanup",
            "agent_runtime_run_activity_canonicalization",
            "chapter_visual_plan_lineage_identity",
            "chapter_visual_plan_authoring_mode_cleanup",
            "image_publication",
            "package_lifecycle",
            "llm_max_tokens_setting_cleanup",
            "llm_cache_usage",
            "claude_cache_usage_repair",
            "web_search_global_config_retirement",
            "chapter_visual_plan_v2_retirement",
            "model_cost_pricing",
            "model_usage_ledger",
            "model_usage_request_snapshot",
            "generation_task_workflow_binding_repair",
            "optimization_session_agent_unification",
            "media_workflow_reset_retirement",
            "optimization_turn_agent_idempotency",
            "agent_history_contract_canonicalization",
            "galgame_generation_spec_source_hash",
            "video_shot_source_identity",
            "body_fact_feedback",
            "fact_feedback_summary_history_index",
            "galgame_scene_source_identity",
            "galgame_scene_snapshot_scope",
            "retire_stale_live_text_anchors",
            "source_sync_local_target_retirement",
            "frozen_chapter_identity",
            "shooting_script_timing_v2",
            "workflow_chapter_identity",
            "media_authoring",
            "agent_project_binding",
            "standalone_comic_project",
            "canvas_text_asset_versions",
            "standalone_comic_text_sources",
            "canvas_project_split",
            "galgame_story_graph",
            "galgame_story_runtime_state",
            "manga_pipeline_artifacts",
            "manga_preprocessing",
            "manga_preprocessing_contract_v2",
            "manga_preprocessing_contract_v3",
            "manga_page_image_artifacts",
            "user_read_only_role_removal",
            "creator_notifications",
            "generated_image_project_index",
            "generated_image_project_chapter_index",
            "workflow_run_project_updated_index",
            "task_queue_project_created_index",
            "project_status_history_project_changed_index",
            "workflow_run_business_updated_index",
            "llm_call_trace_created_index",
            "generation_run_created_index",
            "model_usage_root_started_index",
            "character_portrait_project_index",
            "character_portrait_primary_profile_index",
            "generated_video_project_chapter_index",
            "task_queue_created_index",
        ];
        let history_start = names
            .iter()
            .position(|name| name == deployed_history[0])
            .expect("deployed migration history must remain registered");
        assert_eq!(
            names
                .iter()
                .skip(history_start)
                .take(deployed_history.len())
                .map(String::as_str)
                .collect::<Vec<_>>(),
            deployed_history,
            "deployed migration history must remain registered in execution order"
        );
        assert_eq!(
            names.last().map(String::as_str),
            Some(REQUIRED_SCHEMA_MIGRATION),
            "runtime schema requirement must match the last registered migration"
        );
    }

    #[test]
    fn only_data_migrations_require_a_pre_update_backup() {
        assert!(migration_requires_data_backup("canvas_project_split"));
        assert!(!migration_requires_data_backup("profile_record_ownership"));
        assert!(!migration_requires_data_backup(
            "future_schema_only_migration"
        ));
    }
}
