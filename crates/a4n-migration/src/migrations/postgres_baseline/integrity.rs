use super::*;

pub(super) async fn execute_pg_sql(
    manager: &SchemaManager<'_>,
    sql: impl Into<String>,
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql))
        .await?;
    Ok(())
}

pub(super) async fn create_project_family_integrity_triggers(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    for sql in PROJECT_FAMILY_INTEGRITY_SQL {
        execute_pg_sql(manager, *sql).await?;
    }
    Ok(())
}

pub(super) async fn create_text_source_sync_triggers(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    for sql in TEXT_SOURCE_SYNC_SQL {
        execute_pg_sql(manager, *sql).await?;
    }
    Ok(())
}

pub(super) async fn drop_project_family_integrity_functions(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    for name in PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES.iter().rev() {
        execute_pg_sql(manager, format!("DROP FUNCTION IF EXISTS {name}()")).await?;
    }
    Ok(())
}

pub(super) async fn drop_text_source_sync_functions(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    for name in TEXT_SOURCE_SYNC_FUNCTION_NAMES.iter().rev() {
        execute_pg_sql(manager, format!("DROP FUNCTION IF EXISTS {name}()")).await?;
    }
    Ok(())
}
pub(super) const PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES: &[&str] = &[
    "a4n_validate_novel_project_family",
    "a4n_validate_project_source_snapshot_family",
    "a4n_validate_source_snapshot_chapter_family",
    "a4n_validate_text_anchor_family",
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

pub(super) const TEXT_SOURCE_SYNC_FUNCTION_NAMES: &[&str] =
    &["a4n_mark_text_source_project_changed"];

#[cfg(test)]
pub(super) const TEXT_SOURCE_SYNC_TRIGGER_BASE_NAMES: &[&str] = &[
    "trg_chapter_text_source_changed",
    "trg_chapter_outline_text_source_changed",
    "trg_part_outline_text_source_changed",
    "trg_blueprint_version_text_source_changed",
    "trg_protagonist_profile_text_source_changed",
    "trg_object_profile_text_source_changed",
    "trg_scene_profile_text_source_changed",
];

pub(super) const PROJECT_FAMILY_INTEGRITY_SQL: &[&str] = &[
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_FUNCTION_SQL,
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_INSERT_TRIGGER_SQL,
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_UPDATE_TRIGGER_SQL,
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
    crate::migrations::media_provenance_snapshot_context::MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL,
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
    crate::migrations::galgame_script_integrity_contract::GALGAME_SCRIPT_FAMILY_FUNCTION_SQL,
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
    crate::migrations::galgame_script_integrity_contract::GALGAME_SCENE_FAMILY_FUNCTION_SQL,
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
    crate::migrations::galgame_script_integrity_contract::GALGAME_SCENE_LINK_FAMILY_FUNCTION_SQL,
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
    crate::migrations::galgame_script_integrity_contract::GALGAME_LINE_FAMILY_FUNCTION_SQL,
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
    crate::migrations::galgame_script_integrity_contract::GALGAME_LINE_ORDER_FAMILY_FUNCTION_SQL,
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
    crate::migrations::galgame_script_integrity_contract::GALGAME_CHOICE_FAMILY_FUNCTION_SQL,
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
    crate::migrations::galgame_media_binding_contract::GALGAME_MEDIA_BINDING_FUNCTION_SQL,
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
    crate::migrations::freeform_canonical_media::VIDEO_SEGMENT_FAMILY_FUNCTION_SQL,
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
    crate::migrations::freeform_canonical_media::ASSEMBLY_TIMELINE_FAMILY_FUNCTION_SQL,
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
    crate::migrations::source_sync_regenerate_scene_strategy::SOURCE_SYNC_DECISION_FAMILY_FUNCTION_SQL,
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

pub(super) const TEXT_SOURCE_SYNC_SQL: &[&str] = &[
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
pub(super) const PROJECT_FAMILY_TRIGGER_BASE_NAMES: &[&str] = &[
    "trg_novel_project_family",
    "trg_project_source_snapshot_family",
    "trg_source_snapshot_chapter_family",
    "trg_text_anchor_family",
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
