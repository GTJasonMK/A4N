use std::io::{Cursor, Read};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Serialize, Serializer, ser::Error as _};
use zip::ZipArchive;

use crate::{
    ExportRenderZipEntryInput, ExportRenderZipInput, ExportZipEntryBytes, ExportZipPackageInput,
    ExportZipWriter, render_json_manifest_pretty, render_zip, render_zip_package,
};

struct FailingManifest;

impl Serialize for FailingManifest {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(S::Error::custom("intentional serialization failure"))
    }
}

#[test]
fn render_json_manifest_pretty_reports_labelled_json_errors() {
    let err = render_json_manifest_pretty(&FailingManifest, "项目族清单")
        .expect_err("custom serializer should fail JSON serialization");

    assert!(err.to_string().contains("项目族清单序列化失败"));
}

#[test]
fn export_zip_writer_writes_named_entries() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ExportZipWriter::new(cursor);
    writer
        .write_entry_bytes("manifest.json", br#"{"ok":true}"#, "manifest")
        .expect("write manifest entry");
    writer
        .write_entry_bytes("media/file.txt", b"payload", "media file")
        .expect("write media entry");
    let cursor = writer.finish("test package").expect("finish zip");
    let bytes = cursor.into_inner();

    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("read zip");
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .expect("manifest entry")
        .read_to_string(&mut manifest)
        .expect("read manifest");
    assert_eq!(manifest, r#"{"ok":true}"#);

    let mut media = Vec::new();
    archive
        .by_name("media/file.txt")
        .expect("media entry")
        .read_to_end(&mut media)
        .expect("read media");
    assert_eq!(media, b"payload");
}

#[test]
fn export_zip_writer_rejects_duplicate_entry_path() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ExportZipWriter::new(cursor);
    writer
        .write_entry_bytes("media/file.txt", b"first", "media file")
        .expect("write first entry");

    let err = writer
        .write_entry_bytes("media/file.txt", b"second", "media file")
        .expect_err("duplicate zip entries must fail");

    assert!(err.to_string().contains("导出 zip 条目重复"));
    assert!(err.to_string().contains("media/file.txt"));
}

#[test]
fn export_zip_writer_rejects_unsafe_entry_path() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ExportZipWriter::new(cursor);

    let err = writer
        .write_entry_bytes("../payload.bin", b"payload", "media file")
        .expect_err("unsafe zip entry paths must fail");

    assert!(err.to_string().contains("不能越界"));
    assert!(err.to_string().contains("../payload.bin"));
}

#[test]
fn render_zip_package_writes_manifest_and_entries() {
    let bytes = render_zip_package(ExportZipPackageInput {
        manifest_path: "manifest.json".to_string(),
        manifest_bytes: br#"{"ok":true}"#.to_vec(),
        manifest_label: "manifest".to_string(),
        entries: vec![ExportZipEntryBytes {
            path: "media/file.txt".to_string(),
            bytes: b"payload".to_vec(),
            label: "media file".to_string(),
        }],
        package_label: "test package".to_string(),
    })
    .expect("render zip package");

    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("read zip");
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .expect("manifest entry")
        .read_to_string(&mut manifest)
        .expect("read manifest");
    assert_eq!(manifest, r#"{"ok":true}"#);

    let mut media = Vec::new();
    archive
        .by_name("media/file.txt")
        .expect("media entry")
        .read_to_end(&mut media)
        .expect("read media");
    assert_eq!(media, b"payload");
}

#[test]
fn render_zip_package_rejects_duplicate_entry_path() {
    let err = render_zip_package(ExportZipPackageInput {
        manifest_path: "manifest.json".to_string(),
        manifest_bytes: b"{}".to_vec(),
        manifest_label: "manifest".to_string(),
        entries: vec![ExportZipEntryBytes {
            path: "manifest.json".to_string(),
            bytes: b"duplicate".to_vec(),
            label: "duplicate".to_string(),
        }],
        package_label: "test package".to_string(),
    })
    .expect_err("duplicate package entry must fail");

    assert!(err.to_string().contains("导出 zip 条目重复"));
    assert!(err.to_string().contains("manifest.json"));
}

#[test]
fn render_zip_encodes_zip_bytes_base64() {
    let output = render_zip(ExportRenderZipInput::from_package_input(
        ExportZipPackageInput {
            manifest_path: "manifest.json".to_string(),
            manifest_bytes: br#"{"ok":true}"#.to_vec(),
            manifest_label: "manifest".to_string(),
            entries: vec![ExportZipEntryBytes {
                path: "media/file.txt".to_string(),
                bytes: b"payload".to_vec(),
                label: "media file".to_string(),
            }],
            package_label: "test package".to_string(),
        },
    ))
    .expect("render zip via rpc dto");

    assert_eq!(output.content_type, "application/zip");
    let bytes = output.into_zip_bytes().expect("decode zip bytes");
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("read zip");
    let mut media = Vec::new();
    archive
        .by_name("media/file.txt")
        .expect("media entry")
        .read_to_end(&mut media)
        .expect("read media");
    assert_eq!(media, b"payload");
}

#[test]
fn render_zip_rejects_invalid_entry_base64() {
    let err = render_zip(ExportRenderZipInput {
        manifest_path: "manifest.json".to_string(),
        manifest_bytes_base64: BASE64.encode(b"{}"),
        manifest_label: "manifest".to_string(),
        entries: vec![ExportRenderZipEntryInput {
            path: "media/file.txt".to_string(),
            bytes_base64: "not base64".to_string(),
            label: "media file".to_string(),
        }],
        package_label: "test package".to_string(),
    })
    .expect_err("invalid entry base64 must fail");

    assert!(err.to_string().contains("media file base64 解码失败"));
}
