use super::import_rows::media_asset_status_for_import_insert;
use super::remap::*;
use super::validation::{
    snapshot_profile_key, validate_media_bytes, validate_media_manifest,
    validate_package_entry_path, validate_package_manifest,
};
use super::*;
use a4n_services::generation::{
    GENERATION_ENTITY_ROLES, GENERATION_MEDIA_REF_ROLES, GENERATION_PROMPT_KINDS,
    GENERATION_SPEC_STATUSES, GENERATION_SPEC_TARGET_ENTITY_KINDS, GENERATION_SPEC_TARGET_KINDS,
    GENERATION_TEXT_ANCHOR_ROLES,
};
use a4n_services::media::{
    MEDIA_BINDING_ROLES, MEDIA_BINDING_TARGET_KINDS, MEDIA_KIND_AUDIO, MEDIA_KIND_IMAGE,
    MEDIA_KIND_VIDEO, MEDIA_ORIGIN_ASSEMBLED_PREVIEW, MEDIA_ORIGIN_EXTRACTED_FRAME,
    MEDIA_ORIGIN_GENERATED, MEDIA_ORIGIN_IMPORTED, MEDIA_STATUS_CANCELLED, MEDIA_STATUS_FAILED,
    MEDIA_STATUS_RUNNING,
};
use a4n_services::project::{
    DERIVED_PROJECT_STATUS_DRAFT, PROJECT_KIND_VIDEO, SOURCE_SYNC_CONFLICT, SOURCE_SYNC_CURRENT,
    SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED,
};
use a4n_services::source_sync::{
    CONFLICT_SEVERITIES, CONFLICT_STATUSES, DECISION_ACTORS, LOCAL_STATE_KINDS,
    RESOLUTION_STRATEGIES, SOURCE_CHANGE_KINDS, SYNC_RUN_STATUSES,
};
use chrono::NaiveDate;

fn dt() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap()
}

const PROJECT_KIND_GALGAME_TEST: &str = "galgame";

fn media_asset_with_checksum(bytes: &[u8]) -> media_asset::Model {
    media_asset::Model {
        id: 10,
        project_id: 2,
        root_project_id: 1,
        source_snapshot_id: Some(901),
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: "imported".to_string(),
        file_path: Some("projects/2/media/video/input.mp4".to_string()),
        thumbnail_path: None,
        width: None,
        height: None,
        duration_secs: Some(8.0),
        fps: Some(24.0),
        file_size: Some(bytes.len() as i64),
        checksum: Some(sha256_bytes(bytes)),
        status: "completed".to_string(),
        error_message: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn root_project() -> novel_project::Model {
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

fn derived_video_project() -> novel_project::Model {
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

fn derived_galgame_project() -> novel_project::Model {
    let mut project = derived_video_project();
    project.title = "galgame".to_string();
    project.project_kind = PROJECT_KIND_GALGAME_TEST.to_string();
    project
}

fn source_snapshot_record() -> project_source_snapshot::Model {
    project_source_snapshot::Model {
        id: 901,
        root_project_id: 1,
        source_version_hash: "hash".to_string(),
        blueprint_version_id: None,
        chapter_scope_json: json!({"start": 1, "end": 10}),
        chapter_hashes_json: json!([]),
        profile_hashes_json: json!([]),
        created_at: dt(),
    }
}

fn project_derivation_record() -> project_derivation::Model {
    project_derivation::Model {
        id: 911,
        root_project_id: 1,
        derived_project_id: 2,
        derived_kind: PROJECT_KIND_VIDEO.to_string(),
        source_snapshot_id: 901,
        chapter_scope_json: json!({"start": 1, "end": 10}),
        creation_options_json: json!({}),
        created_at: dt(),
    }
}

fn galgame_project_derivation_record() -> project_derivation::Model {
    let mut derivation = project_derivation_record();
    derivation.derived_kind = PROJECT_KIND_GALGAME_TEST.to_string();
    derivation
}

fn source_snapshot_chapter_record(chapter_number: i32) -> source_snapshot_chapter::Model {
    let content = format!("第{chapter_number}章冻结正文");
    source_snapshot_chapter::Model {
        id: 1000 + chapter_number as i64,
        snapshot_id: 901,
        root_project_id: 1,
        source_chapter_id: 2000 + chapter_number as i64,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        content_hash: hash_text(&content),
        content,
        summary: None,
        analysis_data_json: None,
        analysis_hash: None,
    }
}

fn source_snapshot_chapter_hash_row(chapter: &source_snapshot_chapter::Model) -> Value {
    json!({
        "source_chapter_id": chapter.source_chapter_id,
        "chapter_number": chapter.chapter_number,
        "content_hash": chapter.content_hash.clone(),
        "analysis_hash": chapter.analysis_hash.clone(),
    })
}

fn source_snapshot_outline_record(outline_kind: &str) -> source_snapshot_outline::Model {
    let payload = json!({
        "id": 301,
        "project_id": 1,
        "outline_kind": outline_kind,
        "title": "冻结大纲"
    });
    source_snapshot_outline::Model {
        id: 1201,
        snapshot_id: 901,
        root_project_id: 1,
        source_outline_id: 301,
        outline_kind: outline_kind.to_string(),
        payload_hash: hash_json(&payload),
        payload_json: payload,
    }
}

fn source_snapshot_profile_record(profile_kind: &str) -> source_snapshot_profile::Model {
    let payload = json!({
        "id": 401,
        "project_id": 1,
        "profile_kind": profile_kind,
        "name": "冻结档案"
    });
    source_snapshot_profile::Model {
        id: 1301,
        snapshot_id: 901,
        root_project_id: 1,
        profile_kind: profile_kind.to_string(),
        source_profile_id: 401,
        payload_hash: hash_json(&payload),
        payload_json: payload,
    }
}

fn source_snapshot_profile_hash_row(profile: &source_snapshot_profile::Model) -> Value {
    json!({
        "kind": profile.profile_kind.clone(),
        "id": profile.source_profile_id,
        "hash": profile.payload_hash.clone(),
    })
}

fn package_with_derived_project() -> ProjectFamilyPackage {
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

fn package_with_galgame_project() -> ProjectFamilyPackage {
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
    package.projects.push(derived_galgame_project());
    package.source_snapshots = vec![snapshot];
    package.source_snapshot_chapters = snapshot_chapters;
    package.project_derivations = vec![galgame_project_derivation_record()];
    package.galgame_scripts = vec![galgame_script_record()];
    package.galgame_scenes = vec![galgame_scene_record(1001)];
    package.galgame_lines = vec![galgame_line_record(2201)];
    package
}

fn part_outline(
    id: i64,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
) -> part_outline::Model {
    part_outline::Model {
        id,
        project_id: 1,
        part_number,
        title: format!("第{part_number}部"),
        start_chapter,
        end_chapter,
        summary: None,
        theme: None,
        key_events: None,
        character_focus: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn chapter_outline(id: i64, chapter_number: i32) -> chapter_outline::Model {
    chapter_outline::Model {
        id,
        project_id: 1,
        part_id: None,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        summary: None,
        beats: None,
        key_scenes: None,
        pacing_score: None,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: None,
        status: "draft".to_string(),
        created_at: dt(),
        updated_at: dt(),
    }
}

fn chapter_outline_with_part(id: i64, chapter_number: i32, part_id: i64) -> chapter_outline::Model {
    let mut outline = chapter_outline(id, chapter_number);
    outline.part_id = Some(part_id);
    outline
}

fn chapter_record(id: i64, chapter_number: i32, outline_id: Option<i64>) -> chapter::Model {
    let content = format!("第{chapter_number}章正文");
    chapter::Model {
        id,
        project_id: 1,
        outline_id,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        word_count: content.chars().count() as i32,
        content,
        status: chapter::STATUS_IMPORTED.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn character_profile_record(id: i64) -> protagonist_profile::Model {
    protagonist_profile::Model {
        id,
        project_id: 1,
        name: "林晓雨".to_string(),
        gender: None,
        age: None,
        base_attributes: json!({}),
        current_state: json!({}),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter: 1,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn galgame_script_record() -> galgame_script::Model {
    galgame_script::Model {
        id: 2101,
        project_id: 2,
        root_project_id: 1,
        source_snapshot_id: 901,
        title: "Galgame 剧本".to_string(),
        style_config_json: json!({}),
        status: "draft".to_string(),
        created_at: dt(),
        updated_at: dt(),
    }
}

fn galgame_scene_record(source_snapshot_chapter_id: i64) -> galgame_scene::Model {
    galgame_scene_record_with_id(2201, source_snapshot_chapter_id, "main", None)
}

fn galgame_scene_record_with_id(
    id: i64,
    source_snapshot_chapter_id: i64,
    entry_kind: &str,
    next_scene_id: Option<i64>,
) -> galgame_scene::Model {
    galgame_scene::Model {
        id,
        script_id: 2101,
        source_snapshot_chapter_id,
        scene_index: 1,
        title: "开场".to_string(),
        location: None,
        time_of_day: None,
        participants_json: json!([]),
        background_note: None,
        entry_kind: entry_kind.to_string(),
        next_scene_id,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn galgame_line_record(scene_id: i64) -> galgame_line::Model {
    galgame_line_record_with_id(2301, scene_id)
}

fn galgame_line_record_with_id(id: i64, scene_id: i64) -> galgame_line::Model {
    galgame_line::Model {
        id,
        scene_id,
        line_index: 1,
        line_type: "narration".to_string(),
        speaker_profile_id: None,
        text: "这里是开场旁白。".to_string(),
        expression: None,
        pose: None,
        position: None,
        presentation_json: json!({}),
        text_anchor_id: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn galgame_choice_record(id: i64, line_id: i64, target_scene_id: i64) -> galgame_choice::Model {
    galgame_choice_record_with_index(id, line_id, 1, target_scene_id)
}

fn galgame_choice_record_with_index(
    id: i64,
    line_id: i64,
    option_index: i32,
    target_scene_id: i64,
) -> galgame_choice::Model {
    galgame_choice::Model {
        id,
        line_id,
        option_index,
        option_text: "前往".to_string(),
        target_scene_id: Some(target_scene_id),
        created_at: dt(),
    }
}

fn chapter_text_anchor_record(chapter: &chapter::Model) -> text_anchor::Model {
    text_anchor::Model {
        id: 701,
        project_id: 1,
        source_chapter_id: Some(chapter.id),
        source_snapshot_id: None,
        source_snapshot_chapter_id: None,
        source_kind: SOURCE_KIND_CHAPTER_CONTENT.to_string(),
        source_id: chapter.id,
        paragraph_index: 0,
        start_offset: 0,
        end_offset: chapter.content.chars().count() as i32,
        quote: chapter.content.clone(),
        quote_hash: hash_text(&chapter.content),
        source_version_hash: hash_text(&chapter.content),
        created_at: dt(),
    }
}

fn snapshot_text_anchor_record(
    snapshot_chapter: &source_snapshot_chapter::Model,
) -> text_anchor::Model {
    text_anchor::Model {
        id: 702,
        project_id: 1,
        source_chapter_id: Some(snapshot_chapter.source_chapter_id),
        source_snapshot_id: Some(snapshot_chapter.snapshot_id),
        source_snapshot_chapter_id: Some(snapshot_chapter.id),
        source_kind: SOURCE_KIND_CHAPTER_CONTENT.to_string(),
        source_id: snapshot_chapter.id,
        paragraph_index: 0,
        start_offset: 0,
        end_offset: snapshot_chapter.content.chars().count() as i32,
        quote: snapshot_chapter.content.clone(),
        quote_hash: hash_text(&snapshot_chapter.content),
        source_version_hash: snapshot_chapter.content_hash.clone(),
        created_at: dt(),
    }
}

fn profile_evidence_record() -> profile_evidence::Model {
    profile_evidence::Model {
        id: 801,
        profile_kind: "character".to_string(),
        profile_id: 601,
        text_anchor_id: 701,
        evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
        created_at: dt(),
    }
}

fn package_with_part_outlines(part_outlines: Vec<part_outline::Model>) -> ProjectFamilyPackage {
    ProjectFamilyPackage {
        version: PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
        exported_at: dt().to_string(),
        mode: "project_family".to_string(),
        root_project_id: 1,
        projects: vec![root_project()],
        text_core: TextCorePackage {
            part_outlines,
            ..Default::default()
        },
        project_derivations: Vec::new(),
        source_snapshots: Vec::new(),
        source_snapshot_chapters: Vec::new(),
        source_snapshot_outlines: Vec::new(),
        source_snapshot_profiles: Vec::new(),
        text_anchors: Vec::new(),
        profile_evidence: Vec::new(),
        generation_specs: Vec::new(),
        generation_spec_text_anchors: Vec::new(),
        generation_spec_entity_refs: Vec::new(),
        generation_spec_media_refs: Vec::new(),
        prompt_artifacts: Vec::new(),
        media_assets: Vec::new(),
        media_provenance: Vec::new(),
        media_bindings: Vec::new(),
        galgame_scripts: Vec::new(),
        galgame_scenes: Vec::new(),
        galgame_lines: Vec::new(),
        galgame_choices: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: Vec::new(),
        assembly_clips: Vec::new(),
        source_sync_runs: Vec::new(),
        source_sync_conflicts: Vec::new(),
        source_sync_decisions: Vec::new(),
        media_files: Vec::new(),
    }
}

fn empty_media_payload() -> ImportMediaPayload {
    ImportMediaPayload {
        entries: HashMap::new(),
        bytes: HashMap::new(),
    }
}

fn media_payload_for_assets(assets: &[media_asset::Model]) -> ImportMediaPayload {
    let mut payload = empty_media_payload();
    for asset in assets {
        if let Some(file_path) = asset.file_path.as_ref() {
            let package_path = format!("media/{}/file", asset.id);
            payload.entries.insert(
                media_file_key(asset.id, MEDIA_FILE_FIELD_FILE),
                MediaFileEntry {
                    media_asset_id: asset.id,
                    field: MEDIA_FILE_FIELD_FILE.to_string(),
                    original_path: file_path.clone(),
                    package_path: package_path.clone(),
                },
            );
            payload
                .bytes
                .insert(package_path, b"expected bytes".to_vec());
        }
    }
    payload
}

fn completed_media_asset_with_kind(media_kind: &str) -> media_asset::Model {
    let mut asset = media_asset_with_checksum(b"expected bytes");
    asset.media_kind = media_kind.to_string();
    asset
}

fn media_asset_with_status(status: &str) -> media_asset::Model {
    let mut asset = media_asset_with_checksum(b"expected bytes");
    asset.status = status.to_string();
    asset.file_path = None;
    asset.thumbnail_path = None;
    asset.file_size = None;
    asset.checksum = None;
    asset
}

fn media_asset_with_kind(media_kind: &str) -> media_asset::Model {
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.media_kind = media_kind.to_string();
    asset
}

fn media_asset_with_origin(origin: &str) -> media_asset::Model {
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.origin = origin.to_string();
    asset
}

fn media_binding_record(target_kind: &str, role: &str) -> media_binding::Model {
    media_binding::Model {
        id: 901,
        media_asset_id: 10,
        target_kind: target_kind.to_string(),
        target_id: 201,
        role: role.to_string(),
        created_at: dt(),
    }
}

fn media_provenance_record(request_snapshot_json: Option<Value>) -> media_provenance::Model {
    media_provenance::Model {
        media_asset_id: 10,
        generation_spec_id: None,
        prompt_artifact_id: None,
        provider_job_id: None,
        source_media_asset_id: None,
        source_external_path: None,
        imported_from_user: false,
        request_snapshot_json,
        created_at: dt(),
    }
}

fn generation_spec_record() -> generation_spec::Model {
    generation_spec::Model {
        id: 301,
        project_id: 1,
        root_project_id: 1,
        source_context_kind: Some("snapshot_backed".to_string()),
        source_snapshot_id: Some(401),
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: "video_segment".to_string(),
        target_entity_kind: "segment".to_string(),
        target_entity_id: None,
        status: "draft".to_string(),
        source_version_hash: "hash".to_string(),
        params_json: json!({}),
        created_at: dt(),
        updated_at: dt(),
    }
}

fn generation_spec_for_source_snapshot(snapshot_id: i64) -> generation_spec::Model {
    let mut spec = generation_spec_record();
    spec.source_snapshot_id = Some(snapshot_id);
    spec
}

fn generation_text_anchor_ref(role: &str) -> generation_spec_text_anchor::Model {
    generation_spec_text_anchor::Model {
        generation_spec_id: 301,
        text_anchor_id: 501,
        role: role.to_string(),
    }
}

fn generation_entity_ref(entity_kind: &str, role: &str) -> generation_spec_entity_ref::Model {
    generation_spec_entity_ref::Model {
        generation_spec_id: 301,
        entity_kind: entity_kind.to_string(),
        entity_id: 601,
        role: role.to_string(),
    }
}

fn generation_media_ref(role: &str) -> generation_spec_media_ref::Model {
    generation_spec_media_ref::Model {
        generation_spec_id: 301,
        media_asset_id: 10,
        role: role.to_string(),
    }
}

fn package_with_valid_media_binding_target(target_kind: &str) -> ProjectFamilyPackage {
    let mut package = match target_kind {
        "galgame_scene" | "galgame_line" => package_with_galgame_project(),
        _ => package_with_derived_project(),
    };
    match target_kind {
        "character_profile" => {
            package.text_core.character_profiles = vec![character_profile_record(201)];
        }
        "object_profile" => {
            package.text_core.object_profiles = vec![object_profile_record(201)];
        }
        "scene_profile" => {
            package.text_core.scene_profiles = vec![scene_profile_record(201)];
        }
        "video_segment" => {
            package.video_segments = vec![video_segment_record(201)];
        }
        "chapter" => {
            package.text_core.chapters = vec![chapter_record(201, 1, None)];
        }
        "assembly_timeline" => {
            package.assembly_timelines = vec![assembly_timeline_record(201)];
        }
        "galgame_scene" => {
            package.galgame_scripts = vec![galgame_script_record()];
            package.galgame_scenes = vec![galgame_scene_record_with_id(201, 1001, "main", None)];
        }
        "galgame_line" => {
            package.galgame_scripts = vec![galgame_script_record()];
            package.galgame_scenes = vec![galgame_scene_record_with_id(2201, 1001, "main", None)];
            package.galgame_lines = vec![galgame_line_record_with_id(201, 2201)];
        }
        _ => {}
    }
    package
}

fn object_profile_record(id: i64) -> object_profile::Model {
    object_profile::Model {
        id,
        project_id: 1,
        name: "旧怀表".to_string(),
        normalized_name: "旧怀表".to_string(),
        aliases_json: json!([]),
        object_type: "prop".to_string(),
        importance: "supporting".to_string(),
        visual_description: "一块旧怀表".to_string(),
        material_and_state: "黄铜，磨损".to_string(),
        story_usage: "提示过去".to_string(),
        visual_usage: "特写道具".to_string(),
        generation_usage: "保持复古质感".to_string(),
        location_context: None,
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence_json: json!([]),
        image_prompt_seed: "vintage brass pocket watch".to_string(),
        created_at: dt(),
        updated_at: dt(),
    }
}

fn scene_profile_record(id: i64) -> scene_profile::Model {
    scene_profile::Model {
        id,
        project_id: 1,
        name: "雨夜车站".to_string(),
        normalized_name: "雨夜车站".to_string(),
        aliases_json: json!([]),
        scene_type: "location".to_string(),
        importance: "major".to_string(),
        parent_scene_id: None,
        base_description: "雨夜中的旧车站".to_string(),
        visual_description: "湿润站台和冷色灯光".to_string(),
        spatial_layout: "站台、候车室、出口".to_string(),
        atmosphere: "冷清".to_string(),
        entry_points_json: json!([]),
        rules_json: json!({}),
        current_state_json: json!({}),
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence_json: json!([]),
        image_prompt_seed: "rainy night train station".to_string(),
        created_at: dt(),
        updated_at: dt(),
    }
}

fn video_segment_record(id: i64) -> video_segment::Model {
    video_segment::Model {
        id,
        project_id: 2,
        root_project_id: 1,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        segment_type: "shot".to_string(),
        order_index: 1,
        title: "镜头".to_string(),
        duration_secs: 6.0,
        source_generation_spec_id: None,
        result_media_asset_id: None,
        previous_segment_id: None,
        next_segment_id: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn assembly_timeline_record(id: i64) -> assembly_timeline::Model {
    assembly_timeline::Model {
        id,
        project_id: 2,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        version: 1,
        status: "draft".to_string(),
        preview_media_asset_id: None,
        preview_cache_key: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn prompt_artifact_record(prompt_kind: &str) -> prompt_artifact::Model {
    prompt_artifact::Model {
        id: 701,
        generation_spec_id: 301,
        prompt_kind: prompt_kind.to_string(),
        prompt_text: "prompt text".to_string(),
        renderer_version: "test".to_string(),
        edited_by_user: false,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn source_sync_run_record() -> source_sync_run::Model {
    source_sync_run::Model {
        id: 801,
        derived_project_id: 2,
        root_project_id: 1,
        from_snapshot_id: 901,
        to_snapshot_id: 902,
        mode: SYNC_RUN_MODE_DRY_RUN.to_string(),
        status: SYNC_RUN_STATUS_PLANNED.to_string(),
        summary_json: json!({}),
        error_message: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

fn source_sync_conflict_record() -> source_sync_conflict::Model {
    source_sync_conflict::Model {
        id: 811,
        sync_run_id: 801,
        derived_project_id: 2,
        root_project_id: 1,
        from_snapshot_id: 901,
        to_snapshot_id: 902,
        conflict_scope: "generation_spec".to_string(),
        target_kind: "segment".to_string(),
        target_id: 301,
        source_change_kind: "updated".to_string(),
        local_state_kind: "draft".to_string(),
        severity: "warning".to_string(),
        status: CONFLICT_STATUS_PENDING.to_string(),
        resolution_strategy: None,
        source_summary_json: json!({}),
        local_summary_json: json!({}),
        affected_refs_json: json!({}),
        decision_json: None,
        created_at: dt(),
        resolved_at: None,
    }
}

fn handled_source_sync_conflict(status: &str) -> source_sync_conflict::Model {
    let mut conflict = source_sync_conflict_record();
    conflict.status = status.to_string();
    conflict.resolution_strategy = Some("keep_local".to_string());
    conflict.decision_json = Some(json!({"reason": "kept"}));
    conflict.resolved_at = Some(dt());
    conflict
}

fn handled_source_sync_conflict_for_decision(
    status: &str,
    strategy: &str,
    decision_payload_json: Value,
) -> source_sync_conflict::Model {
    let mut conflict = source_sync_conflict_record();
    conflict.status = status.to_string();
    conflict.resolution_strategy = Some(strategy.to_string());
    conflict.decision_json = Some(decision_payload_json);
    conflict.resolved_at = Some(dt());
    conflict
}

fn source_sync_decision_record(strategy: &str) -> source_sync_decision::Model {
    source_sync_decision::Model {
        id: 821,
        conflict_id: 811,
        decided_by: "user".to_string(),
        strategy: strategy.to_string(),
        decision_payload_json: json!({}),
        created_at: dt(),
    }
}

fn package_with_source_sync_conflict(
    conflict: source_sync_conflict::Model,
) -> ProjectFamilyPackage {
    let mut package = package_with_source_sync_conflict_target(&conflict);
    package.source_sync_runs = vec![source_sync_run_record()];
    package.source_sync_conflicts = vec![conflict];
    package
}

fn package_with_source_sync_conflict_target(
    conflict: &source_sync_conflict::Model,
) -> ProjectFamilyPackage {
    match (
        conflict.conflict_scope.as_str(),
        conflict.target_kind.as_str(),
    ) {
        ("generation_spec", "segment") | ("text_anchor", "segment") => {
            let mut package = package_with_derived_project();
            package.generation_specs = vec![generation_spec_for_source_snapshot(
                source_sync_run_record().from_snapshot_id,
            )];
            package
        }
        ("text_anchor", "chapter") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.chapters = vec![chapter_record(conflict.target_id, 1, None)];
            package
        }
        ("text_anchor", "galgame_scene") => {
            let mut package = package_with_galgame_project();
            package.galgame_scenes = vec![galgame_scene_record_with_id(
                conflict.target_id,
                1001,
                "main",
                None,
            )];
            package.galgame_lines = vec![galgame_line_record(conflict.target_id)];
            package
        }
        ("text_anchor", "galgame_line") => {
            let mut package = package_with_galgame_project();
            package.galgame_scenes = vec![galgame_scene_record(1001)];
            package.galgame_lines = vec![galgame_line_record_with_id(conflict.target_id, 2201)];
            package
        }
        ("profile", "character") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.character_profiles =
                vec![character_profile_record(conflict.target_id)];
            package
        }
        ("profile", "object") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.object_profiles = vec![object_profile_record(conflict.target_id)];
            package
        }
        ("profile", "scene") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.scene_profiles = vec![scene_profile_record(conflict.target_id)];
            package
        }
        _ => package_with_part_outlines(Vec::new()),
    }
}

mod core_validation;
mod media_validation;
mod remap;
mod source_sync_validation;
