use super::super::*;
use a4n_db::text_anchor::SOURCE_KIND_CHAPTER_CONTENT;

pub(in crate::project::family_package::tests) fn part_outline(
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

pub(in crate::project::family_package::tests) fn chapter_outline(
    id: i64,
    chapter_number: i32,
) -> chapter_outline::Model {
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

pub(in crate::project::family_package::tests) fn chapter_outline_with_part(
    id: i64,
    chapter_number: i32,
    part_id: i64,
) -> chapter_outline::Model {
    let mut outline = chapter_outline(id, chapter_number);
    outline.part_id = Some(part_id);
    outline
}

pub(in crate::project::family_package::tests) fn chapter_record(
    id: i64,
    chapter_number: i32,
    outline_id: Option<i64>,
) -> chapter::Model {
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

pub(in crate::project::family_package::tests) fn character_profile_record(
    id: i64,
) -> protagonist_profile::Model {
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

pub(in crate::project::family_package::tests) fn chapter_text_anchor_record(
    chapter: &chapter::Model,
) -> text_anchor::Model {
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

pub(in crate::project::family_package::tests) fn package_with_part_outlines(
    part_outlines: Vec<part_outline::Model>,
) -> ProjectFamilyPackage {
    ProjectFamilyPackage {
        version: PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
        exported_at: dt().to_string(),
        mode: "project_family".to_string(),
        root_project_id: 1,
        projects: vec![root_project()],
        text_core: TextCorePackage {
            fact_feedback: None,
            part_outlines,
            ..Default::default()
        },
        source_snapshots: Vec::new(),
        source_snapshot_chapters: Vec::new(),
        source_snapshot_outlines: Vec::new(),
        source_snapshot_profiles: Vec::new(),
        text_anchors: Vec::new(),
        profile_evidence: Vec::new(),
        chapter_visual_plans: Vec::new(),
        chapter_shooting_scripts: Vec::new(),
        authoring_documents: Vec::new(),
        authoring_receipts: Vec::new(),
        character_portraits: Vec::new(),
        generated_images: Vec::new(),
        chapter_manga_prompts: Vec::new(),
        manga_pipeline_artifacts: Vec::new(),
        chapter_video_prompts: Vec::new(),
        generated_videos: Vec::new(),
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

pub(in crate::project::family_package::tests) fn object_profile_record(
    id: i64,
) -> object_profile::Model {
    object_profile::Model {
        id,
        project_id: 1,
        ownership_kind: "legacy_unknown".to_string(),
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

pub(in crate::project::family_package::tests) fn scene_profile_record(
    id: i64,
) -> scene_profile::Model {
    scene_profile::Model {
        id,
        project_id: 1,
        ownership_kind: "legacy_unknown".to_string(),
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
