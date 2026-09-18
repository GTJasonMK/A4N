//! Unified project bundle contract.
//!
//! The existing family and freeform packages remain the compatibility
//! implementations for their domain rows. This manifest composes them with
//! project-scoped conversations and durable generation history.

use std::collections::{BTreeMap, HashSet};

use a4n_db::project_bundle::ProjectBundleHistoryRows;
use a4n_db_models::models::generation::generation_spec;
use a4n_domain::clear_canvas_transfer_runtime_identity;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{MediaFileEntry, ProjectFamilyPackage, VideoStudioProjectPackage};

pub(crate) const PROJECT_BUNDLE_PACKAGE_VERSION: &str = "a4n_project_bundle_v1";
pub(crate) const MANIFEST_PATH: &str = "a4n-project-bundle.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProjectBundlePackage {
    pub version: String,
    pub exported_at: String,
    pub bundle_id: String,
    pub mode: String,
    pub root_project_id: i64,
    pub family: Option<ProjectFamilyPackage>,
    pub canvas_projects: Vec<VideoStudioProjectPackage>,
    pub history: ProjectBundleHistoryRows,
    pub provider_slots: Vec<ProjectBundleProviderSlot>,
    pub media_files: Vec<MediaFileEntry>,
    #[serde(default)]
    pub media_sha256: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectBundleProviderSlot {
    pub slot_key: String,
    pub capability: String,
    pub provider: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub parameters: Value,
    pub configuration_fingerprint_sha256: String,
    pub requires_credentials: bool,
}

impl ProjectBundlePackage {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version != PROJECT_BUNDLE_PACKAGE_VERSION {
            return Err(format!(
                "项目包版本不受支持: expected={}, actual={}",
                PROJECT_BUNDLE_PACKAGE_VERSION, self.version
            ));
        }
        if self.bundle_id.trim().is_empty() || self.bundle_id.contains(':') {
            return Err("项目包 bundle_id 无效".to_string());
        }
        if self.root_project_id <= 0 {
            return Err("项目包 root_project_id 必须大于 0".to_string());
        }
        if self.family.is_none() && self.canvas_projects.is_empty() {
            return Err("项目包至少需要一个项目族或画布项目".to_string());
        }
        if let Some(family) = &self.family {
            if family.root_project_id != self.root_project_id {
                return Err("项目包根项目与项目族根项目不一致".to_string());
            }
        }
        let mut family_project_ids = HashSet::new();
        if let Some(family) = &self.family {
            for project in &family.projects {
                if !family_project_ids.insert(project.id) {
                    return Err(format!("项目包项目重复: {}", project.id));
                }
            }
        }
        let mut canvas_project_ids = HashSet::new();
        for canvas in &self.canvas_projects {
            if canvas.project.id <= 0 {
                return Err("画布项目 ID 必须大于 0".to_string());
            }
            if family_project_ids.contains(&canvas.project.id) {
                // A family project may also own a canvas. It is intentionally
                // allowed; import will attach the graph to the remapped row.
                continue;
            }
            if !canvas_project_ids.insert(canvas.project.id) {
                return Err(format!("画布项目重复: {}", canvas.project.id));
            }
        }
        validate_media_entries(&self.media_files)?;
        validate_provider_slots(&self.provider_slots)
    }
}

pub(crate) fn assemble_project_bundle(
    root_project_id: i64,
    mode: &str,
    family: Option<ProjectFamilyPackage>,
    canvas_projects: Vec<VideoStudioProjectPackage>,
    history: ProjectBundleHistoryRows,
    media_files: Vec<MediaFileEntry>,
) -> Result<ProjectBundlePackage, String> {
    let provider_slots = collect_provider_slots(&family, &canvas_projects, &history)?;
    let package = ProjectBundlePackage {
        version: PROJECT_BUNDLE_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        bundle_id: uuid::Uuid::new_v4().simple().to_string(),
        mode: mode.to_string(),
        root_project_id,
        family,
        canvas_projects,
        history,
        provider_slots,
        media_files,
        media_sha256: BTreeMap::new(),
    };
    package.validate()?;
    Ok(package)
}

fn collect_provider_slots(
    family: &Option<ProjectFamilyPackage>,
    canvas_projects: &[VideoStudioProjectPackage],
    history: &ProjectBundleHistoryRows,
) -> Result<Vec<ProjectBundleProviderSlot>, String> {
    let mut slots = BTreeMap::<String, ProjectBundleProviderSlot>::new();
    for run in &history.generation_runs {
        insert_provider_slot(
            &mut slots,
            "generation_run",
            "llm",
            run.provider.clone(),
            run.model.clone(),
            None,
            json!({}),
        )?;
    }
    let mut insert_package = |package: &VideoStudioProjectPackage| -> Result<(), String> {
        for spec in &package.generation_specs {
            insert_spec_provider_slot(&mut slots, spec)?;
        }
        for image in &package.generated_images {
            insert_provider_slot(
                &mut slots,
                "generated_image",
                "image",
                None,
                Some(image.model.clone()),
                None,
                json!({}),
            )?;
        }
        for video in &package.generated_videos {
            insert_provider_slot(
                &mut slots,
                "generated_video",
                "video",
                None,
                Some(video.model.clone()),
                None,
                json!({}),
            )?;
        }
        Ok(())
    };
    for package in canvas_projects {
        insert_package(package)?;
    }
    if let Some(family) = family {
        for spec in &family.generation_specs {
            insert_spec_provider_slot(&mut slots, spec)?;
        }
        for image in &family.generated_images {
            insert_provider_slot(
                &mut slots,
                "generated_image",
                "image",
                None,
                Some(image.model.clone()),
                None,
                json!({}),
            )?;
        }
        for video in &family.generated_videos {
            insert_provider_slot(
                &mut slots,
                "generated_video",
                "video",
                None,
                Some(video.model.clone()),
                None,
                json!({}),
            )?;
        }
    }
    Ok(slots.into_values().collect())
}

fn insert_spec_provider_slot(
    slots: &mut BTreeMap<String, ProjectBundleProviderSlot>,
    spec: &generation_spec::Model,
) -> Result<(), String> {
    let mut material = spec.params_json.clone();
    clear_canvas_transfer_runtime_identity(&mut material);
    let provider = material
        .get("provider")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let model = material
        .get("model")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let capability = match spec.target_kind.as_str() {
        value if value.contains("image") => "image",
        value if value.contains("video") => "video",
        _ => "llm",
    };
    insert_provider_slot(
        slots,
        "generation_spec",
        capability,
        provider,
        model,
        material
            .get("base_url")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        material,
    )
}

fn insert_provider_slot(
    slots: &mut BTreeMap<String, ProjectBundleProviderSlot>,
    source: &str,
    capability: &str,
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    parameters: Value,
) -> Result<(), String> {
    if provider.is_none() && model.is_none() {
        return Ok(());
    }
    let slot_key = format!(
        "{}:{}:{}",
        capability,
        provider.as_deref().unwrap_or("unknown"),
        model.as_deref().unwrap_or("unknown")
    );
    let fingerprint_input = json!({
        "capability": capability,
        "provider": provider,
        "model": model,
        "base_url": base_url,
        "parameters": parameters,
    });
    let fingerprint_bytes =
        serde_json::to_vec(&fingerprint_input).map_err(|error| error.to_string())?;
    let fingerprint = hex_sha256(&fingerprint_bytes);
    slots
        .entry(slot_key.clone())
        .or_insert(ProjectBundleProviderSlot {
            slot_key,
            capability: capability.to_string(),
            provider,
            backend: None,
            model,
            base_url,
            parameters: fingerprint_input["parameters"].clone(),
            configuration_fingerprint_sha256: fingerprint,
            requires_credentials: true,
        });
    let _ = source;
    Ok(())
}

fn validate_media_entries(entries: &[MediaFileEntry]) -> Result<(), String> {
    let mut paths = HashSet::with_capacity(entries.len());
    for entry in entries {
        if entry.package_path.is_empty()
            || entry.package_path.starts_with('/')
            || entry.package_path.split('/').any(|part| part == "..")
        {
            return Err(format!("媒体包路径不安全: {}", entry.package_path));
        }
        if !paths.insert(entry.package_path.clone()) {
            return Err(format!("媒体包路径重复: {}", entry.package_path));
        }
    }
    Ok(())
}

fn validate_provider_slots(slots: &[ProjectBundleProviderSlot]) -> Result<(), String> {
    let mut keys = HashSet::with_capacity(slots.len());
    for slot in slots {
        if slot.slot_key.is_empty() || slot.configuration_fingerprint_sha256.len() != 64 {
            return Err("项目包 Provider 槽位缺少稳定指纹".to_string());
        }
        if !keys.insert(slot.slot_key.clone()) {
            return Err(format!("Provider 槽位重复: {}", slot.slot_key));
        }
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{PROJECT_BUNDLE_PACKAGE_VERSION, ProjectBundlePackage, validate_media_entries};

    #[test]
    fn media_paths_reject_traversal() {
        assert!(
            validate_media_entries(&[a4n_api_contract::project_transfer::MediaFileEntry {
                owner_kind: "asset".to_string(),
                owner_id: 1,
                field: "file_path".to_string(),
                original_path: "data/a.png".to_string(),
                package_path: "../escape".to_string(),
            }])
            .is_err()
        );
    }

    #[test]
    fn bundle_version_is_explicit() {
        assert_eq!(PROJECT_BUNDLE_PACKAGE_VERSION, "a4n_project_bundle_v1");
        let _ = std::mem::size_of::<ProjectBundlePackage>();
    }
}
