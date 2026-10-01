use a4n_db_models::models::project::novel_project;
use a4n_domain::project::ProjectRecord;

pub(crate) fn project_record_from_model(row: novel_project::Model) -> ProjectRecord {
    ProjectRecord {
        id: row.id,
        user_id: row.user_id,
        title: row.title,
        description: row.description,
        project_type: row.project_type,
        project_kind: row.project_kind,
        root_project_id: row.root_project_id,
        source_snapshot_id: row.source_snapshot_id,
        source_sync_status: row.source_sync_status,
        status: row.status,
        lifecycle_gate: row.lifecycle_gate,
        target_chapters: row.target_chapters,
        current_chapter: row.current_chapter,
        total_chapters: row.total_chapters,
        chapters_per_part: row.chapters_per_part,
        avatar_svg: row.avatar_svg,
        avatar_animal: row.avatar_animal,
        raw_blueprint: row.raw_blueprint,
        import_analysis_progress: row.import_analysis_progress,
        export_config: row.export_config,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub(crate) fn project_record_into_model(record: ProjectRecord) -> novel_project::Model {
    novel_project::Model {
        id: record.id,
        user_id: record.user_id,
        title: record.title,
        description: record.description,
        project_type: record.project_type,
        project_kind: record.project_kind,
        root_project_id: record.root_project_id,
        source_snapshot_id: record.source_snapshot_id,
        source_sync_status: record.source_sync_status,
        status: record.status,
        lifecycle_gate: record.lifecycle_gate,
        target_chapters: record.target_chapters,
        current_chapter: record.current_chapter,
        total_chapters: record.total_chapters,
        chapters_per_part: record.chapters_per_part,
        avatar_svg: record.avatar_svg,
        avatar_animal: record.avatar_animal,
        raw_blueprint: record.raw_blueprint,
        import_analysis_progress: record.import_analysis_progress,
        export_config: record.export_config,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}
