use super::*;
use std::{
    collections::HashSet,
    fs,
    io::{Cursor, Read, Seek, Write},
    path::{Component, Path},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub(super) fn package_zip_bytes(
    state: &AppState,
    package: &VideoStudioProjectPackage,
) -> Result<Vec<u8>, String> {
    let cursor = Cursor::new(Vec::new());
    let cursor = write_package_zip_to_writer(state, cursor, package)?;
    Ok(cursor.into_inner())
}

fn write_package_zip_to_writer<W: Write + Seek>(
    state: &AppState,
    writer: W,
    package: &VideoStudioProjectPackage,
) -> Result<W, String> {
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let manifest = serde_json::to_vec_pretty(package)
        .map_err(|err| format!("视频画布项目导出包序列化失败: {err}"))?;
    start_zip_file(&mut zip, MANIFEST_PATH, options)?;
    zip.write_all(&manifest)
        .map_err(|err| format!("写入视频画布项目清单失败: {err}"))?;
    for media in &package.media_files {
        let source = canonicalize_data_file_path(
            &state.config.data_dir,
            &media.original_path,
            "视频画布项目媒体文件",
        )?;
        let bytes = fs::read(&source)
            .map_err(|err| format!("读取媒体文件失败: {}: {err}", source.display()))?;
        start_zip_file(&mut zip, &media.package_path, options)?;
        zip.write_all(&bytes)
            .map_err(|err| format!("写入媒体文件失败: {}: {err}", media.package_path))?;
    }
    zip.finish()
        .map_err(|err| format!("完成视频画布项目导出包失败: {err}"))
}

fn start_zip_file<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    path: &str,
    options: SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(path, options)
        .map_err(|err| format!("写入 zip 条目失败: {path}: {err}"))
}

pub(super) fn read_video_studio_package_reader(
    bytes: Vec<u8>,
) -> Result<(VideoStudioProjectPackage, ImportMediaPayload), String> {
    let reader = Cursor::new(bytes);
    let mut zip =
        ZipArchive::new(reader).map_err(|err| format!("读取视频画布项目 zip 失败: {err}"))?;
    let mut manifest_bytes = Vec::new();
    zip.by_name(MANIFEST_PATH)
        .map_err(|err| format!("视频画布项目包缺少清单 {MANIFEST_PATH}: {err}"))?
        .read_to_end(&mut manifest_bytes)
        .map_err(|err| format!("读取视频画布项目清单失败: {err}"))?;
    let package: VideoStudioProjectPackage = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| format!("解析视频画布项目清单失败: {err}"))?;
    validate_declared_zip_entries(&mut zip, &package)?;
    let media_payload = read_media_payloads(&mut zip, &package)?;
    Ok((package, media_payload))
}

fn validate_declared_zip_entries<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    package: &VideoStudioProjectPackage,
) -> Result<(), String> {
    let declared_files = package
        .media_files
        .iter()
        .map(|entry| entry.package_path.as_str())
        .chain(std::iter::once(MANIFEST_PATH))
        .collect::<HashSet<_>>();
    let mut seen_files = HashSet::new();

    for index in 0..zip.len() {
        let file = zip
            .by_index(index)
            .map_err(|err| format!("读取视频画布项目 zip 条目失败: index={index}: {err}"))?;
        let name = file.name().to_string();
        validate_package_entry_path(&name)?;
        if file.is_dir() {
            continue;
        }
        if !seen_files.insert(name.clone()) {
            return Err(format!("视频画布项目包 zip 条目重复: {name}"));
        }
        if !declared_files.contains(name.as_str()) {
            return Err(format!("视频画布项目包包含未在清单声明的文件: {name}"));
        }
    }

    Ok(())
}

fn read_media_payloads<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    package: &VideoStudioProjectPackage,
) -> Result<ImportMediaPayload, String> {
    let mut entries = HashMap::new();
    let mut bytes = HashMap::new();
    for entry in &package.media_files {
        validate_package_entry_path(&entry.package_path)?;
        validation::validate_media_entry_owner(package, entry)?;
        let key = media_file_key(&entry.owner_kind, entry.owner_id, &entry.field);
        if entries.insert(key, entry.clone()).is_some() {
            return Err(format!(
                "视频画布项目媒体清单重复: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            ));
        }
        let mut file_bytes = Vec::new();
        zip.by_name(&entry.package_path)
            .map_err(|err| format!("视频画布项目包缺少媒体文件 {}: {err}", entry.package_path))?
            .read_to_end(&mut file_bytes)
            .map_err(|err| format!("读取媒体文件失败 {}: {err}", entry.package_path))?;
        if file_bytes.is_empty() {
            return Err(format!("媒体文件不能为空: {}", entry.package_path));
        }
        validation::validate_media_entry_bytes(package, entry, &file_bytes)?;
        bytes.insert(entry.package_path.clone(), file_bytes);
    }
    Ok(ImportMediaPayload { entries, bytes })
}

fn validate_package_entry_path(path: &str) -> Result<(), String> {
    let raw = Path::new(path);
    if raw.is_absolute() {
        return Err(format!("zip 条目不能是绝对路径: {path}"));
    }
    let mut has_component = false;
    for component in raw.components() {
        match component {
            Component::Normal(_) => {
                has_component = true;
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("zip 条目路径不能越界: {path}"));
            }
        }
    }
    if !has_component {
        return Err("zip 条目路径不能为空".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_services::media::MEDIA_STATUS_COMPLETED;
    use serde_json::json;
    use sha2::Digest;

    fn now() -> chrono::NaiveDateTime {
        chrono::Utc::now().naive_utc()
    }

    fn freeform_project() -> novel_project::Model {
        novel_project::Model {
            id: 1,
            user_id: 1,
            title: "自由视频".to_string(),
            description: String::new(),
            project_type: "script".to_string(),
            project_kind: PROJECT_KIND_VIDEO.to_string(),
            source_mode: SOURCE_MODE_FREEFORM.to_string(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            source_sync_status: SOURCE_SYNC_CURRENT.to_string(),
            derived_project_status: None,
            status: "draft".to_string(),
            target_chapters: 1,
            current_chapter: 1,
            total_chapters: None,
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn package() -> VideoStudioProjectPackage {
        VideoStudioProjectPackage {
            version: VIDEO_STUDIO_PACKAGE_VERSION.to_string(),
            exported_at: now().to_string(),
            mode: PACKAGE_MODE.to_string(),
            project: freeform_project(),
            canvas_graphs: Vec::new(),
            canvas_nodes: Vec::new(),
            canvas_edges: Vec::new(),
            canvas_text_assets: Vec::new(),
            generation_specs: Vec::new(),
            prompt_artifacts: Vec::new(),
            generated_images: Vec::new(),
            generated_videos: Vec::new(),
            media_assets: Vec::new(),
            media_provenance: Vec::new(),
            media_bindings: Vec::new(),
            video_segments: Vec::new(),
            assembly_timelines: Vec::new(),
            assembly_clips: Vec::new(),
            media_files: Vec::new(),
        }
    }

    fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in entries {
            if name.ends_with('/') {
                zip.add_directory(*name, options)
                    .expect("start zip directory entry");
            } else {
                zip.start_file(*name, options).expect("start zip entry");
                zip.write_all(bytes).expect("write zip entry");
            }
        }
        zip.finish().expect("finish zip").into_inner()
    }

    #[test]
    fn read_video_studio_package_rejects_undeclared_zip_file() {
        let manifest = serde_json::to_vec(&package()).expect("serialize package");
        let bytes = zip_with_entries(&[
            (MANIFEST_PATH, &manifest),
            ("media/generated_video/10/file.mp4", b"undeclared"),
        ]);

        let err =
            read_video_studio_package_reader(bytes).expect_err("undeclared package file must fail");

        assert!(err.contains("未在清单声明"));
        assert!(err.contains("media/generated_video/10/file.mp4"));
    }

    #[test]
    fn read_video_studio_package_ignores_directory_entries() {
        let manifest = serde_json::to_vec(&package()).expect("serialize package");
        let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest), ("media/", b"")]);

        read_video_studio_package_reader(bytes).expect("directory entries should be ignored");
    }

    #[test]
    fn read_video_studio_package_rejects_parent_dir_entry() {
        let manifest = serde_json::to_vec(&package()).expect("serialize package");
        let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest), ("../payload.bin", b"bad")]);

        let err = read_video_studio_package_reader(bytes)
            .expect_err("zip entries must not escape package root");

        assert!(err.contains("路径不能越界"));
    }

    #[test]
    fn read_video_studio_package_accepts_declared_media_file() {
        let mut package = package();
        let bytes = b"media bytes";
        package.media_assets.push(media_asset::Model {
            id: 10,
            project_id: 1,
            root_project_id: 1,
            source_snapshot_id: None,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            media_kind: "video".to_string(),
            origin: "imported".to_string(),
            file_path: Some("projects/1/videos/asset.mp4".to_string()),
            thumbnail_path: None,
            width: Some(1280),
            height: Some(720),
            duration_secs: Some(3.0),
            fps: Some(24.0),
            file_size: Some(bytes.len() as i64),
            checksum: Some({
                let mut hasher = sha2::Sha256::new();
                hasher.update(bytes);
                format!("{:x}", hasher.finalize())
            }),
            status: MEDIA_STATUS_COMPLETED.to_string(),
            error_message: None,
            created_at: now(),
            updated_at: now(),
        });
        package.media_files.push(VideoStudioMediaFileEntry {
            owner_kind: MEDIA_OWNER_MEDIA_ASSET.to_string(),
            owner_id: 10,
            field: MEDIA_FIELD_FILE.to_string(),
            original_path: "projects/1/videos/asset.mp4".to_string(),
            package_path: "media/media_asset/10/file.mp4".to_string(),
        });
        let manifest = serde_json::to_vec(&package).expect("serialize package");
        let archive = zip_with_entries(&[
            (MANIFEST_PATH, &manifest),
            ("media/media_asset/10/file.mp4", bytes),
        ]);

        let (_, payload) =
            read_video_studio_package_reader(archive).expect("declared media file should read");

        assert_eq!(
            payload.bytes["media/media_asset/10/file.mp4"],
            bytes.to_vec()
        );
        assert_eq!(payload.entries.len(), 1);
        assert_eq!(json!(payload.bytes.len()), json!(1));
    }
}
