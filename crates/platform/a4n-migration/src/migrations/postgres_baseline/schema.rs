use super::*;

pub(super) async fn up(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(
            "A4N baseline schema only supports PostgreSQL".to_string(),
        ));
    }

    let schema = Schema::new(DbBackend::Postgres);

    create_entity_table!(manager, schema, user::Entity);
    execute_pg_sql(
        manager,
        "ALTER TABLE \"user\" ALTER COLUMN page_cli_enabled SET DEFAULT false",
    )
    .await?;
    create_entity_table!(manager, schema, invite_code::Entity);
    create_entity_table!(manager, schema, user_session::Entity);
    create_entity_table!(manager, schema, theme_config::Entity);
    // The historical theme integrity migrations still need to read the old
    // column before theme_system_v3 removes it from deployed databases.
    execute_pg_sql(
            manager,
            "ALTER TABLE theme_config ADD COLUMN IF NOT EXISTS css_variables JSON NOT NULL DEFAULT '{}'",
        )
        .await?;
    create_entity_table!(manager, schema, theme_preference::Entity);
    create_entity_table!(manager, schema, theme_resource::Entity);
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
    create_entity_table!(manager, schema, canvas_text_asset_version::Entity);
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
    create_entity_index!(
        manager,
        "uidx_canvas_text_asset_version",
        canvas_text_asset_version::Entity,
        [
            canvas_text_asset_version::Column::CanvasTextAssetId,
            canvas_text_asset_version::Column::Version
        ],
        unique
    );
    create_entity_table!(manager, schema, text_anchor::Entity);

    create_entity_table!(manager, schema, part_outline::Entity);
    create_entity_table!(manager, schema, chapter_outline::Entity);
    create_entity_table!(manager, schema, chapter_outline_provenance::Entity);
    create_entity_table!(manager, schema, chapter::Entity);
    create_entity_table!(manager, schema, chapter_scene::Entity);
    create_entity_table!(manager, schema, chapter_visual_plan::Entity);
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
    create_entity_table!(manager, schema, standalone_comic_text_source::Entity);
    create_entity_table!(manager, schema, chapter_video_prompt::Entity);

    create_entity_table!(manager, schema, protagonist_profile::Entity);
    create_entity_table!(manager, schema, attribute_event::Entity);
    create_entity_table!(manager, schema, character_state_index::Entity);
    create_entity_table!(manager, schema, character_portrait::Entity);
    create_entity_table!(manager, schema, protagonist_behavior_record::Entity);
    create_entity_table!(manager, schema, protagonist_deletion_mark::Entity);
    create_entity_table!(manager, schema, snapshot_reference::Entity);
    create_entity_table!(manager, schema, foreshadowing_index::Entity);

    execute_pg_sql(
        manager,
        "ALTER TABLE attribute_event ALTER COLUMN source_kind SET DEFAULT 'legacy_unknown'",
    )
    .await?;
    execute_pg_sql(
            manager,
            "ALTER TABLE protagonist_behavior_record ALTER COLUMN source_kind SET DEFAULT 'legacy_unknown'",
        )
        .await?;
    execute_pg_sql(
            manager,
            "ALTER TABLE protagonist_deletion_mark ALTER COLUMN source_kind SET DEFAULT 'legacy_unknown'",
        )
        .await?;

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
    create_entity_index!(
        manager,
        "uidx_system_settings_key",
        system_settings::Entity,
        [system_settings::Column::Key],
        unique
    );
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

pub(super) async fn down(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
    drop_entity_table!(manager, standalone_comic_text_source::Entity);
    drop_entity_table!(manager, chapter_manga_prompt::Entity);
    drop_entity_table!(manager, chapter_shooting_script::Entity);
    drop_entity_table!(manager, chapter_visual_plan::Entity);
    drop_entity_table!(manager, chapter_scene::Entity);
    drop_entity_table!(manager, chapter_outline_provenance::Entity);
    drop_entity_table!(manager, chapter::Entity);
    drop_entity_table!(manager, chapter_outline::Entity);
    drop_entity_table!(manager, part_outline::Entity);
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
    drop_entity_table!(manager, theme_resource::Entity);
    drop_entity_table!(manager, theme_preference::Entity);
    drop_entity_table!(manager, message::Entity);
    drop_entity_table!(manager, conversation::Entity);
    drop_entity_table!(manager, user_session::Entity);
    drop_entity_table!(manager, invite_code::Entity);
    drop_entity_table!(manager, user::Entity);
    drop_text_source_sync_functions(manager).await?;
    drop_project_family_integrity_functions(manager).await?;

    Ok(())
}
