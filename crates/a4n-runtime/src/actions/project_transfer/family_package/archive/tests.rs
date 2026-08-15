use std::io::{Cursor, Write};

use zip::{ZipWriter, write::SimpleFileOptions};

use super::super::{
    MANIFEST_PATH, PROJECT_FAMILY_PACKAGE_VERSION, ProjectFamilyPackage, TextCorePackage,
};
use super::reader::read_project_family_package_reader;

fn empty_package() -> ProjectFamilyPackage {
    ProjectFamilyPackage {
        version: PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
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
