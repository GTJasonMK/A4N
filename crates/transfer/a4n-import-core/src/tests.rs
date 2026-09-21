use std::io::{Cursor, Write};

use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::{
    ImportReadMediaPackageZipInput,
    archive::{
        read_json_manifest_entry, read_zip_entries_bytes, read_zip_entry_bytes,
        validate_zip_declared_files, validate_zip_manifest_declared_files,
    },
    json::parse_json_manifest,
    read_media_package_zip,
};

#[test]
fn read_media_package_zip_reads_manifest_and_declared_entries() {
    let manifest = serde_json::json!({
        "version": "test",
        "media_files": [
            { "package_path": "media/file.txt" }
        ]
    });
    let manifest_bytes = serde_json::to_vec(&manifest).expect("serialize manifest");
    let zip_bytes = zip_with_entries(&[
        ("manifest.json", manifest_bytes.as_slice()),
        ("media/file.txt", b"payload"),
    ]);

    let output = read_media_package_zip(ImportReadMediaPackageZipInput::from_zip_bytes(
        zip_bytes,
        "manifest.json",
        "测试导入包",
        "测试清单",
    ))
    .expect("read media package zip");
    let (manifest_value, entries) = output
        .into_manifest_and_entries_bytes()
        .expect("decode returned entries");

    assert_eq!(manifest_value["version"], "test");
    assert_eq!(entries.get("media/file.txt").unwrap(), b"payload");
}

#[test]
fn read_media_package_zip_rejects_undeclared_zip_entry() {
    let manifest = serde_json::json!({
        "media_files": []
    });
    let manifest_bytes = serde_json::to_vec(&manifest).expect("serialize manifest");
    let zip_bytes = zip_with_entries(&[
        ("manifest.json", manifest_bytes.as_slice()),
        ("media/extra.txt", b"bad"),
    ]);

    let err = read_media_package_zip(ImportReadMediaPackageZipInput::from_zip_bytes(
        zip_bytes,
        "manifest.json",
        "测试导入包",
        "测试清单",
    ))
    .expect_err("undeclared zip entry must fail");

    assert!(err.to_string().contains("未在清单声明"));
    assert!(err.to_string().contains("media/extra.txt"));
}

#[test]
fn read_media_package_zip_rejects_missing_package_path() {
    let manifest = serde_json::json!({
        "media_files": [
            { "field": "file_path" }
        ]
    });
    let manifest_bytes = serde_json::to_vec(&manifest).expect("serialize manifest");
    let zip_bytes = zip_with_entries(&[("manifest.json", manifest_bytes.as_slice())]);

    let err = read_media_package_zip(ImportReadMediaPackageZipInput::from_zip_bytes(
        zip_bytes,
        "manifest.json",
        "测试导入包",
        "测试清单",
    ))
    .expect_err("missing package_path must fail");

    assert!(err.to_string().contains("package_path 必须是非空字符串"));
}

#[test]
fn read_media_package_zip_rejects_invalid_zip_base64() {
    let err = read_media_package_zip(ImportReadMediaPackageZipInput {
        zip_bytes_base64: "not base64".to_string(),
        manifest_path: "manifest.json".to_string(),
        package_label: "测试导入包".to_string(),
        manifest_label: "测试清单".to_string(),
    })
    .expect_err("invalid base64 must fail");

    assert!(err.to_string().contains("测试导入包 base64 解码失败"));
}

#[test]
fn parse_json_manifest_reports_labelled_json_errors() {
    let err = parse_json_manifest::<serde_json::Value>(b"{", "项目族清单").unwrap_err();

    assert!(err.to_string().contains("项目族清单解析失败"));
}

fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        if name.ends_with('/') {
            zip.add_directory(*name, options)
                .expect("start zip directory");
        } else {
            zip.start_file(*name, options).expect("start zip entry");
            zip.write_all(bytes).expect("write zip entry");
        }
    }
    zip.finish().expect("finish zip").into_inner()
}

#[test]
fn read_zip_entry_bytes_reads_named_entry() {
    let bytes = zip_with_entries(&[("manifest.json", br#"{"ok":true}"#)]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    let entry = read_zip_entry_bytes(&mut zip, "manifest.json", "测试包").unwrap();

    assert_eq!(entry, br#"{"ok":true}"#);
}

#[test]
fn read_json_manifest_entry_reads_and_parses_manifest() {
    let bytes = zip_with_entries(&[("manifest.json", br#"{"ok":true}"#)]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    let manifest: serde_json::Value =
        read_json_manifest_entry(&mut zip, "manifest.json", "测试包", "测试清单").unwrap();

    assert_eq!(manifest["ok"], true);
}

#[test]
fn read_zip_entries_bytes_reads_named_entries() {
    let bytes = zip_with_entries(&[
        ("manifest.json", br#"{"ok":true}"#),
        ("media/file.txt", b"payload"),
    ]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    let entries =
        read_zip_entries_bytes(&mut zip, ["manifest.json", "media/file.txt"], "测试包").unwrap();

    assert_eq!(entries["manifest.json"].as_slice(), br#"{"ok":true}"#);
    assert_eq!(entries["media/file.txt"].as_slice(), b"payload");
}

#[test]
fn read_zip_entries_bytes_rejects_duplicate_declared_path() {
    let bytes = zip_with_entries(&[("media/file.txt", b"payload")]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    let err = read_zip_entries_bytes(&mut zip, ["media/file.txt", "media/file.txt"], "测试包")
        .unwrap_err();

    assert!(err.to_string().contains("重复声明"));
    assert!(err.to_string().contains("media/file.txt"));
}

#[test]
fn validate_zip_manifest_declared_files_includes_manifest_entry() {
    let bytes = zip_with_entries(&[("manifest.json", b"{}"), ("media/file.txt", b"payload")]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    validate_zip_manifest_declared_files(&mut zip, "manifest.json", ["media/file.txt"], "测试包")
        .unwrap();
}

#[test]
fn validate_zip_manifest_declared_files_rejects_extra_file() {
    let bytes = zip_with_entries(&[
        ("manifest.json", b"{}"),
        ("media/file.txt", b"payload"),
        ("media/extra.txt", b"extra"),
    ]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    let err = validate_zip_manifest_declared_files(
        &mut zip,
        "manifest.json",
        ["media/file.txt"],
        "测试包",
    )
    .unwrap_err();

    assert!(err.to_string().contains("未在清单声明"));
    assert!(err.to_string().contains("media/extra.txt"));
}

#[test]
fn validate_zip_declared_files_rejects_undeclared_file() {
    let bytes = zip_with_entries(&[("manifest.json", b"{}"), ("payload.bin", b"payload")]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    let err = validate_zip_declared_files(&mut zip, ["manifest.json"], "测试包").unwrap_err();

    assert!(err.to_string().contains("未在清单声明"));
    assert!(err.to_string().contains("payload.bin"));
}

#[test]
fn validate_zip_declared_files_rejects_duplicate_declared_file() {
    let bytes = zip_with_entries(&[("payload.bin", b"payload")]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    let err = validate_zip_declared_files(&mut zip, ["payload.bin", "payload.bin"], "测试包")
        .unwrap_err();

    assert!(err.to_string().contains("重复声明"));
    assert!(err.to_string().contains("payload.bin"));
}

#[test]
fn validate_zip_declared_files_ignores_directory_entries() {
    let bytes = zip_with_entries(&[("manifest.json", b"{}"), ("media/", b"")]);
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");

    validate_zip_declared_files(&mut zip, ["manifest.json"], "测试包").unwrap();
}
