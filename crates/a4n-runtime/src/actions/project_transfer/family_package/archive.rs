use super::import_rows::{cleanup_imported_media_files, import_package_rows};
use super::validation::{
    validate_media_bytes, validate_media_manifest, validate_package_entry_path,
    validate_package_manifest,
};
use super::*;

pub(super) async fn import_project_family_package_bytes(
    state: &AppState,
    user_id: i64,
    bytes: Vec<u8>,
) -> Result<Vec<novel_project::Model>, String> {
    if bytes.is_empty() {
        return Err("项目族导入包不能为空".to_string());
    }
    let reader = std::io::Cursor::new(bytes);
    let (package, media_payload) = read_project_family_package_reader(reader)?;
    import_project_family_package_parts(state, user_id, package, media_payload).await
}

async fn import_project_family_package_parts(
    state: &AppState,
    user_id: i64,
    package: ProjectFamilyPackage,
    media_payload: ImportMediaPayload,
) -> Result<Vec<novel_project::Model>, String> {
    validate_package_manifest(&package)?;
    validate_media_manifest(&package, &media_payload)?;

    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let mut written_files = Vec::new();
    let import_result = import_package_rows(
        state,
        user_id,
        &txn,
        &package,
        &media_payload,
        &mut written_files,
    )
    .await;
    let imported_projects = match import_result {
        Ok(projects) => projects,
        Err(err) => {
            cleanup_imported_media_files(&written_files);
            return Err(err);
        }
    };
    if let Err(err) = txn.commit().await {
        cleanup_imported_media_files(&written_files);
        return Err(format!("提交项目族导入事务失败: {err}"));
    }
    Ok(imported_projects)
}

fn read_project_family_package_reader<R: Read + Seek>(
    reader: R,
) -> Result<(ProjectFamilyPackage, ImportMediaPayload), String> {
    let mut zip = ZipArchive::new(reader).map_err(|err| format!("读取项目族 zip 失败: {err}"))?;
    let mut manifest_bytes = Vec::new();
    zip.by_name(MANIFEST_PATH)
        .map_err(|err| format!("项目族导入包缺少清单 {MANIFEST_PATH}: {err}"))?
        .read_to_end(&mut manifest_bytes)
        .map_err(|err| format!("读取项目族清单失败: {err}"))?;
    let package: ProjectFamilyPackage = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| format!("解析项目族清单失败: {err}"))?;
    let media_payload = read_media_payloads(&mut zip, &package)?;
    Ok((package, media_payload))
}

fn read_media_payloads<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    package: &ProjectFamilyPackage,
) -> Result<ImportMediaPayload, String> {
    let media_assets = package
        .media_assets
        .iter()
        .map(|asset| (asset.id, asset))
        .collect::<HashMap<_, _>>();
    let mut entries = HashMap::new();
    let mut bytes = HashMap::new();
    for entry in &package.media_files {
        validate_package_entry_path(&entry.package_path)?;
        let asset = media_assets.get(&entry.media_asset_id).ok_or_else(|| {
            format!(
                "媒体文件清单引用了不存在的 media_asset_id: {}",
                entry.media_asset_id
            )
        })?;
        let expected_path = media_asset_path(asset, &entry.field)?;
        if expected_path.is_none() {
            return Err(format!(
                "媒体文件清单字段 {} 与媒体资产 {} 不匹配",
                entry.field, asset.id
            ));
        }
        let key = media_file_key(entry.media_asset_id, &entry.field);
        if entries.insert(key, entry.clone()).is_some() {
            return Err(format!(
                "媒体文件清单重复: media_asset_id={}, field={}",
                entry.media_asset_id, entry.field
            ));
        }
        let mut file_bytes = Vec::new();
        zip.by_name(&entry.package_path)
            .map_err(|err| format!("项目族导入包缺少媒体文件 {}: {err}", entry.package_path))?
            .read_to_end(&mut file_bytes)
            .map_err(|err| format!("读取媒体文件失败 {}: {err}", entry.package_path))?;
        if entry.field == "file_path" {
            validate_media_bytes(asset, &file_bytes)?;
        }
        bytes.insert(entry.package_path.clone(), file_bytes);
    }
    Ok(ImportMediaPayload { entries, bytes })
}
