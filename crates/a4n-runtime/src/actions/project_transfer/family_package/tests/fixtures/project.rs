use super::super::*;

pub(in crate::actions::project_transfer::family_package::tests) fn root_project()
-> novel_project::Model {
    novel_project::Model {
        id: 1,
        user_id: 7,
        title: "root".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: PROJECT_KIND_TEXT.to_string(),
        source_mode: "self".to_string(),
        root_project_id: None,
        parent_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        derived_project_status: None,
        status: "draft".to_string(),
        target_chapters: 20,
        current_chapter: 0,
        total_chapters: Some(20),
        chapters_per_part: Some(10),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::actions::project_transfer::family_package::tests) fn derived_video_project()
-> novel_project::Model {
    novel_project::Model {
        id: 2,
        user_id: 7,
        title: "derived".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: PROJECT_KIND_VIDEO.to_string(),
        source_mode: "derived".to_string(),
        root_project_id: Some(1),
        parent_project_id: Some(1),
        source_snapshot_id: Some(901),
        source_sync_status: SOURCE_SYNC_CURRENT.to_string(),
        derived_project_status: Some(DERIVED_PROJECT_STATUS_DRAFT.to_string()),
        status: "draft".to_string(),
        target_chapters: 20,
        current_chapter: 0,
        total_chapters: Some(20),
        chapters_per_part: Some(10),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::actions::project_transfer::family_package::tests) fn package_with_derived_project()
-> ProjectFamilyPackage {
    let mut package = package_with_part_outlines(Vec::new());
    let snapshot_chapters = (1..=10)
        .map(source_snapshot_chapter_record)
        .collect::<Vec<_>>();
    let mut snapshot = source_snapshot_record();
    snapshot.chapter_hashes_json = Value::Array(
        snapshot_chapters
            .iter()
            .map(source_snapshot_chapter_hash_row)
            .collect(),
    );
    package.projects.push(derived_video_project());
    package.source_snapshots = vec![snapshot];
    package.source_snapshot_chapters = snapshot_chapters;
    package.project_derivations = vec![project_derivation_record()];
    package
}
