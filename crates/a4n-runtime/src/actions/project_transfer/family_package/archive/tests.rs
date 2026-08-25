use std::io::{Cursor, Write};

use zip::{ZipWriter, write::SimpleFileOptions};

use super::super::MANIFEST_PATH;
use super::reader::read_project_family_package_reader;
use a4n_core::models::media::media_asset::FILE_PATH_FIELD;
use a4n_domain::canvas::CanvasNodeRefKind;
use a4n_orchestrator::project::{MediaFileEntry, ProjectFamilyPackage, TextCorePackage};

fn empty_package() -> ProjectFamilyPackage {
    ProjectFamilyPackage {
        version: "project_family_v5".to_string(),
        exported_at: "2026-01-02T03:04:05".to_string(),
        mode: "project_family".to_string(),
        root_project_id: 1,
        projects: Vec::new(),
        text_core: TextCorePackage::default(),
        source_snapshots: Vec::new(),
        source_snapshot_chapters: Vec::new(),
        source_snapshot_outlines: Vec::new(),
        source_snapshot_profiles: Vec::new(),
        text_anchors: Vec::new(),
        profile_evidence: Vec::new(),
        chapter_visual_plans: Vec::new(),
        chapter_shooting_scripts: Vec::new(),
        character_portraits: Vec::new(),
        generated_images: Vec::new(),
        chapter_manga_prompts: Vec::new(),
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

fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        zip.start_file(*name, options).expect("start zip entry");
        zip.write_all(bytes).expect("write zip entry");
    }
    zip.finish().expect("finish zip").into_inner()
}

#[test]
fn read_project_family_package_accepts_v5_manifest() {
    let manifest = serde_json::to_vec(&empty_package()).expect("serialize package");
    let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest)]);

    let (package, payload) =
        read_project_family_package_reader(Cursor::new(bytes)).expect("read v5 package");

    assert_eq!(package.version, "project_family_v5");
    assert!(payload.entries.is_empty());
    assert!(payload.bytes.is_empty());
}

#[test]
fn read_project_family_package_rejects_v4_shape_without_visual_plan_rows() {
    let mut manifest = serde_json::to_value(empty_package()).expect("serialize package");
    let object = manifest.as_object_mut().expect("package object");
    object.insert(
        "version".to_string(),
        serde_json::Value::String("project_family_v4".to_string()),
    );
    object.remove("chapter_visual_plans");
    let manifest = serde_json::to_vec(&manifest).expect("serialize v4 package");
    let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest)]);

    let err = match read_project_family_package_reader(Cursor::new(bytes)) {
        Ok(_) => panic!("v4 package shape must fail"),
        Err(err) => err,
    };

    assert!(err.contains("chapter_visual_plans"), "{err}");
}

#[test]
fn read_project_family_package_rejects_v3_shape_without_manga_rows() {
    let mut manifest = serde_json::to_value(empty_package()).expect("serialize package");
    let object = manifest.as_object_mut().expect("package object");
    object.insert(
        "version".to_string(),
        serde_json::Value::String("project_family_v3".to_string()),
    );
    for field in [
        "chapter_shooting_scripts",
        "character_portraits",
        "generated_images",
        "chapter_manga_prompts",
    ] {
        object.remove(field);
    }
    let manifest = serde_json::to_vec(&manifest).expect("serialize v3 package");
    let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest)]);

    let err = match read_project_family_package_reader(Cursor::new(bytes)) {
        Ok(_) => panic!("v3 package shape must fail"),
        Err(err) => err,
    };

    assert!(err.contains("chapter_shooting_scripts"), "{err}");
}

#[test]
fn read_project_family_package_indexes_media_by_owner_and_field() {
    let generated_image_path = "media/generated_image/7/file_path.png";
    let portrait_path = "media/character_portrait/7/image_path.png";
    let mut package = empty_package();
    package.media_files = vec![
        MediaFileEntry {
            owner_kind: CanvasNodeRefKind::GeneratedImage.as_str().to_string(),
            owner_id: 7,
            field: FILE_PATH_FIELD.to_string(),
            original_path: "/data/generated.png".to_string(),
            package_path: generated_image_path.to_string(),
        },
        MediaFileEntry {
            owner_kind: "character_portrait".to_string(),
            owner_id: 7,
            field: "image_path".to_string(),
            original_path: "/data/portrait.png".to_string(),
            package_path: portrait_path.to_string(),
        },
    ];
    let manifest = serde_json::to_vec(&package).expect("serialize package");
    let bytes = zip_with_entries(&[
        (MANIFEST_PATH, &manifest),
        (generated_image_path, b"generated"),
        (portrait_path, b"portrait"),
    ]);

    let (_, payload) =
        read_project_family_package_reader(Cursor::new(bytes)).expect("read media package");

    assert!(payload.entries.contains_key(&(
        CanvasNodeRefKind::GeneratedImage.as_str().to_string(),
        7,
        FILE_PATH_FIELD.to_string(),
    )));
    assert!(payload.entries.contains_key(&(
        "character_portrait".to_string(),
        7,
        "image_path".to_string(),
    )));
    assert_eq!(payload.bytes[generated_image_path], b"generated");
    assert_eq!(payload.bytes[portrait_path], b"portrait");
}

#[test]
fn read_project_family_package_rejects_undeclared_zip_file() {
    let manifest = serde_json::to_vec(&empty_package()).expect("serialize package");
    let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest), ("media/extra.bin", b"bad")]);

    let err = match read_project_family_package_reader(Cursor::new(bytes)) {
        Ok(_) => panic!("undeclared package file must fail"),
        Err(err) => err,
    };

    assert!(err.contains("未在清单声明"));
    assert!(err.contains("media/extra.bin"));
}
