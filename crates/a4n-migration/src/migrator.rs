//! 数据库 schema 初始化器。
//!
//! 新库通过 PostgreSQL baseline 建表；已部署数据库通过后续幂等迁移补齐新增表。

use sea_orm_migration::prelude::*;

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
        ]
    }
}

pub async fn run_migrations(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    Migrator::up(db, None).await
}

#[cfg(test)]
mod tests {
    use super::Migrator;
    use a4n_core::schema::REQUIRED_SCHEMA_MIGRATION;
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
        assert_eq!(
            names
                .iter()
                .rev()
                .take(12)
                .rev()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "neutral_video_segment_plan",
                "shooting_script_transition_type_canonicalization",
                "generation_task_persistence",
                "cleanup_outbox_generation_scope",
                "generation_snapshot_retention",
                "generation_snapshot_hash_contract",
                "segment_card_video_workflow",
                "segment_execution_recipe_identity",
                "segment_recipe_adoption_repair",
                "chapter_video_prompt_execution_identity",
                "drop_chapter_video_prompt_identity_unique",
                "unified_model_provider_requests",
            ],
            "deployed migration history must remain registered in execution order"
        );
        assert_eq!(
            names.last().map(String::as_str),
            Some(REQUIRED_SCHEMA_MIGRATION),
            "runtime schema requirement must match the last registered migration"
        );
    }
}
