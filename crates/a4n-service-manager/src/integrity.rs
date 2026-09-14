use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use a4n_process_catalog::service_catalog;
use a4n_service_protocol::{
    PROTOCOL_VERSION, RELEASE_MANIFEST_ENV, RELEASE_MANIFEST_SCHEMA_VERSION, ReleaseBinaryManifest,
    ReleaseManifest, ReleaseRuntimeFileManifest, release_tool_catalog,
};
use sha2::{Digest, Sha256};

use crate::ServiceManagerError;

#[derive(Debug, Clone)]
pub(super) struct BinaryIntegrityVerifier {
    by_service: HashMap<String, ReleaseBinaryManifest>,
    runtime_files: Vec<ReleaseRuntimeFileManifest>,
    package_root: Option<PathBuf>,
}

impl BinaryIntegrityVerifier {
    pub(super) fn from_env() -> Result<Option<Self>, ServiceManagerError> {
        let Some(path) = std::env::var_os(RELEASE_MANIFEST_ENV).map(PathBuf::from) else {
            return Ok(None);
        };
        if path.as_os_str().to_string_lossy().trim().is_empty() {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest path is blank".to_string(),
            ));
        }
        let bytes = std::fs::read(&path).map_err(|_| {
            ServiceManagerError::ReleaseManifestInvalid("manifest is unavailable".to_string())
        })?;
        let package_root = path
            .parent()
            .ok_or_else(|| {
                ServiceManagerError::ReleaseManifestInvalid(
                    "manifest path has no parent directory".to_string(),
                )
            })?
            .to_path_buf();
        let manifest: ReleaseManifest = serde_json::from_slice(&bytes).map_err(|_| {
            ServiceManagerError::ReleaseManifestInvalid("manifest JSON is invalid".to_string())
        })?;
        let verifier = Self::new(manifest, Some(package_root))?;
        verifier.verify_runtime_files()?;
        Ok(Some(verifier))
    }

    fn new(
        manifest: ReleaseManifest,
        package_root: Option<PathBuf>,
    ) -> Result<Self, ServiceManagerError> {
        if manifest.schema_version != RELEASE_MANIFEST_SCHEMA_VERSION
            || manifest.protocol_version != PROTOCOL_VERSION
            || !valid_catalog_identifier(&manifest.package)
            || manifest.version.trim().is_empty()
            || manifest.target.trim().is_empty()
        {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest schema or protocol version is incompatible".to_string(),
            ));
        }
        let mut names = HashSet::new();
        let mut services = HashSet::new();
        let mut by_service = HashMap::new();
        for entry in manifest.binaries {
            if entry.name.trim().is_empty()
                || entry.service.trim().is_empty()
                || !valid_manifest_path(&entry.path)
                || manifest_file_name(&entry.path) != Some(entry.name.as_str())
                || entry.version.trim().is_empty()
                || entry.target != manifest.target
                || entry.protocol_version != manifest.protocol_version
                || !is_sha256(&entry.sha256)
                || !names.insert(entry.name.clone())
                || !services.insert(entry.service.clone())
            {
                return Err(ServiceManagerError::ReleaseManifestInvalid(
                    "manifest binary entries are inconsistent".to_string(),
                ));
            }
            by_service.insert(entry.service.clone(), entry);
        }
        if by_service.is_empty() {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest has no binaries".to_string(),
            ));
        }

        let catalog = service_catalog().map_err(|_| {
            ServiceManagerError::ReleaseManifestInvalid("service catalog is invalid".to_string())
        })?;
        let catalog_services = catalog
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<HashSet<_>>();
        let selected_service_names = if manifest.services.is_empty() {
            catalog_services.clone()
        } else {
            let selected = manifest.services.iter().cloned().collect::<HashSet<_>>();
            if selected.len() != manifest.services.len()
                || selected.iter().any(|name| !catalog_services.contains(name))
            {
                return Err(ServiceManagerError::ReleaseManifestInvalid(
                    "manifest services contain an unknown or duplicated service".to_string(),
                ));
            }
            selected
        };
        let expected_services = std::iter::once("host".to_string())
            .chain(selected_service_names.iter().cloned())
            .collect::<HashSet<_>>();
        if services != expected_services || by_service.len() != expected_services.len() {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest services do not match service catalog".to_string(),
            ));
        }

        let extension = if manifest.target.contains("windows") {
            ".exe"
        } else {
            ""
        };
        let expected_host_name = format!("{}{extension}", manifest.package);
        let host = by_service.get("host").expect("host is checked above");
        let tauri_app_root = manifest_tauri_app_root(&host.path);
        if tauri_app_root.is_some()
            && !(manifest.target.contains("darwin") || manifest.target.contains("macos"))
        {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "Tauri app layout requires a macOS target".to_string(),
            ));
        }
        let tool_catalog = release_tool_catalog().map_err(|_| {
            ServiceManagerError::ReleaseManifestInvalid(
                "release tool catalog is invalid".to_string(),
            )
        })?;
        if tool_catalog.iter().any(|entry| {
            !valid_catalog_identifier(&entry.package) || !valid_catalog_identifier(&entry.binary)
        }) {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "release tool catalog is invalid".to_string(),
            ));
        }
        let expected_tool_names = tool_catalog
            .iter()
            .map(|entry| format!("{}{extension}", entry.binary))
            .collect::<HashSet<_>>();
        let expected_tool_packages = tool_catalog
            .iter()
            .map(|entry| entry.package.as_str())
            .collect::<HashSet<_>>();
        if tool_catalog.is_empty()
            || expected_tool_names.len() != tool_catalog.len()
            || expected_tool_packages.len() != tool_catalog.len()
        {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "release tool catalog is invalid".to_string(),
            ));
        }
        let mut tool_names = HashSet::new();
        for tool in &manifest.tools {
            if tool.name.trim().is_empty()
                || tool.path != expected_tool_path(tauri_app_root, &tool.name)
                || !valid_manifest_path(&tool.path)
                || manifest_file_name(&tool.path) != Some(tool.name.as_str())
                || tool.version.trim().is_empty()
                || tool.target != manifest.target
                || !is_sha256(&tool.sha256)
                || !tool_names.insert(tool.name.clone())
            {
                return Err(ServiceManagerError::ReleaseManifestInvalid(
                    "manifest tool entries are inconsistent".to_string(),
                ));
            }
        }
        if tool_names != expected_tool_names {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest tools do not match release tool catalog".to_string(),
            ));
        }
        if host.name != expected_host_name
            || host.version != manifest.version
            || host.capability_version.is_some()
            || host.path != expected_host_path(tauri_app_root, &host.name)
        {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest host entry is invalid".to_string(),
            ));
        }
        for entry in catalog {
            if !selected_service_names.contains(&entry.name) {
                continue;
            }
            let binary_name = format!("{}{extension}", entry.binary);
            let binary = by_service
                .get(&entry.name)
                .expect("catalog service is checked above");
            if binary.name != binary_name
                || binary.path != expected_service_path(tauri_app_root, &binary_name)
                || binary.capability_version.as_deref() != Some(entry.capability_version.as_str())
            {
                return Err(ServiceManagerError::ReleaseManifestInvalid(
                    "manifest service catalog entry is inconsistent".to_string(),
                ));
            }
        }
        let expected_runtime_name = if manifest.target.contains("windows") {
            "onnxruntime.dll"
        } else if manifest.target.contains("darwin") || manifest.target.contains("macos") {
            "libonnxruntime.dylib"
        } else {
            "libonnxruntime.so.1"
        };
        if manifest.runtime_files.len() != 1 {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest must contain one runtime file".to_string(),
            ));
        }
        let runtime = &manifest.runtime_files[0];
        if runtime.name != expected_runtime_name
            || runtime.path != expected_runtime_path(tauri_app_root, expected_runtime_name)
            || !valid_manifest_path(&runtime.path)
            || manifest_file_name(&runtime.path) != Some(runtime.name.as_str())
            || !is_sha256(&runtime.sha256)
        {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest runtime file is inconsistent".to_string(),
            ));
        }
        if let Some(package_root) = &package_root {
            verify_manifest_files(
                package_root,
                "tool",
                manifest.tools.iter().map(|entry| {
                    (
                        entry.name.as_str(),
                        entry.path.as_str(),
                        entry.sha256.as_str(),
                    )
                }),
            )?;
        }
        Ok(Self {
            by_service,
            runtime_files: manifest.runtime_files,
            package_root,
        })
    }

    pub(super) fn verify(&self, service: &str, binary: &Path) -> Result<(), ServiceManagerError> {
        self.verify_runtime_files()?;
        let entry = self.by_service.get(service).ok_or_else(|| {
            ServiceManagerError::BinaryIntegrityFailed {
                service: service.to_string(),
                reason: "manifest entry is missing".to_string(),
            }
        })?;
        if Path::new(&entry.path).file_name() != binary.file_name() {
            return Err(ServiceManagerError::BinaryIntegrityFailed {
                service: service.to_string(),
                reason: "binary name does not match manifest".to_string(),
            });
        }
        let actual = sha256(binary).map_err(|_| ServiceManagerError::BinaryIntegrityFailed {
            service: service.to_string(),
            reason: "binary cannot be hashed".to_string(),
        })?;
        if !actual.eq_ignore_ascii_case(&entry.sha256) {
            return Err(ServiceManagerError::BinaryIntegrityFailed {
                service: service.to_string(),
                reason: "SHA-256 mismatch".to_string(),
            });
        }
        Ok(())
    }

    fn verify_runtime_files(&self) -> Result<(), ServiceManagerError> {
        let Some(package_root) = &self.package_root else {
            return Ok(());
        };
        verify_manifest_files(
            package_root,
            "runtime",
            self.runtime_files.iter().map(|entry| {
                (
                    entry.name.as_str(),
                    entry.path.as_str(),
                    entry.sha256.as_str(),
                )
            }),
        )
    }
}

fn verify_manifest_files<'a>(
    package_root: &Path,
    kind: &str,
    entries: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
) -> Result<(), ServiceManagerError> {
    for (name, relative_path, expected_sha256) in entries {
        let path = package_root.join(relative_path);
        let actual = sha256(&path).map_err(|_| ServiceManagerError::BinaryIntegrityFailed {
            service: name.to_string(),
            reason: format!("{kind} file cannot be hashed"),
        })?;
        if !actual.eq_ignore_ascii_case(expected_sha256) {
            return Err(ServiceManagerError::BinaryIntegrityFailed {
                service: name.to_string(),
                reason: format!("{kind} file SHA-256 mismatch"),
            });
        }
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_catalog_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'0'..=b'9'))
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn valid_manifest_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\0')
        && !Path::new(path).is_absolute()
        && !path.split(['/', '\\']).any(|part| part == "..")
}

fn manifest_file_name(path: &str) -> Option<&str> {
    path.rsplit(['/', '\\']).find(|part| !part.is_empty())
}

fn manifest_tauri_app_root(host_path: &str) -> Option<&str> {
    let mut parts = host_path.split('/');
    let root = parts.next()?;
    if !root.ends_with(".app")
        || parts.next() != Some("Contents")
        || parts.next() != Some("MacOS")
        || parts.next().is_none()
        || parts.next().is_some()
    {
        return None;
    }
    Some(root)
}

fn expected_host_path(tauri_app_root: Option<&str>, name: &str) -> String {
    tauri_app_root.map_or_else(
        || format!("bin/{name}"),
        |root| format!("{root}/Contents/MacOS/{name}"),
    )
}

fn expected_service_path(tauri_app_root: Option<&str>, name: &str) -> String {
    tauri_app_root.map_or_else(
        || format!("bin/services/{name}"),
        |root| format!("{root}/Contents/MacOS/{name}"),
    )
}

fn expected_tool_path(tauri_app_root: Option<&str>, name: &str) -> String {
    tauri_app_root.map_or_else(
        || format!("bin/{name}"),
        |root| format!("{root}/Contents/MacOS/{name}"),
    )
}

fn expected_runtime_path(tauri_app_root: Option<&str>, name: &str) -> String {
    tauri_app_root.map_or_else(
        || format!("bin/services/{name}"),
        |root| format!("{root}/Contents/Resources/{name}"),
    )
}

fn sha256(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::{BinaryIntegrityVerifier, is_sha256, sha256};
    use a4n_process_catalog::service_catalog;
    use a4n_service_protocol::{
        PROTOCOL_VERSION, RELEASE_MANIFEST_SCHEMA_VERSION, ReleaseBinaryManifest, ReleaseManifest,
        ReleaseRuntimeFileManifest, ReleaseToolManifest, release_tool_catalog,
    };

    fn valid_manifest(prompt_sha256: String) -> ReleaseManifest {
        let mut binaries = vec![ReleaseBinaryManifest {
            name: "a4n-tauri".to_string(),
            service: "host".to_string(),
            path: "bin/a4n-tauri".to_string(),
            version: "0.1.0".to_string(),
            target: "linux-x86_64".to_string(),
            sha256: "a".repeat(64),
            protocol_version: PROTOCOL_VERSION,
            capability_version: None,
        }];
        for entry in service_catalog().unwrap() {
            let sha256 = if entry.name == "prompt" {
                prompt_sha256.clone()
            } else {
                "a".repeat(64)
            };
            let binary = entry.binary;
            binaries.push(ReleaseBinaryManifest {
                name: binary.clone(),
                service: entry.name,
                path: format!("bin/services/{binary}"),
                version: "0.1.0".to_string(),
                target: "linux-x86_64".to_string(),
                sha256,
                protocol_version: PROTOCOL_VERSION,
                capability_version: Some(entry.capability_version),
            });
        }
        let tools = release_tool_catalog()
            .unwrap()
            .into_iter()
            .map(|entry| ReleaseToolManifest {
                name: entry.binary.clone(),
                path: format!("bin/{}", entry.binary),
                version: "0.1.0".to_string(),
                target: "linux-x86_64".to_string(),
                sha256: "c".repeat(64),
            })
            .collect();
        ReleaseManifest {
            schema_version: RELEASE_MANIFEST_SCHEMA_VERSION,
            package: "a4n-tauri".to_string(),
            version: "0.1.0".to_string(),
            target: "linux-x86_64".to_string(),
            protocol_version: PROTOCOL_VERSION,
            services: service_catalog()
                .unwrap()
                .into_iter()
                .map(|entry| entry.name)
                .collect(),
            binaries,
            tools,
            runtime_files: vec![ReleaseRuntimeFileManifest {
                name: "libonnxruntime.so.1".to_string(),
                path: "bin/services/libonnxruntime.so.1".to_string(),
                sha256: "b".repeat(64),
            }],
        }
    }

    #[test]
    fn sha256_shape_is_strict() {
        assert!(is_sha256(&"a".repeat(64)));
        assert!(!is_sha256(&"g".repeat(64)));
        assert!(!is_sha256(&"a".repeat(63)));
    }

    #[test]
    fn binary_hash_mismatch_is_rejected_before_spawn() {
        let directory = std::env::temp_dir().join(format!(
            "a4n-integrity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("a4n-prompt-service");
        std::fs::write(&path, b"original").unwrap();
        let digest = sha256(&path).unwrap();
        let verifier = BinaryIntegrityVerifier::new(valid_manifest(digest), None).unwrap();
        verifier.verify("prompt", &path).unwrap();
        std::fs::write(&path, b"tampered").unwrap();
        assert!(verifier.verify("prompt", &path).is_err());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn manifest_path_traversal_is_rejected_before_spawn() {
        let mut manifest = valid_manifest("a".repeat(64));
        manifest
            .binaries
            .iter_mut()
            .find(|entry| entry.service == "prompt")
            .unwrap()
            .path = "bin/../escape".to_string();
        let error = BinaryIntegrityVerifier::new(manifest, None)
            .expect_err("manifest traversal must be rejected");
        assert!(
            error
                .to_string()
                .contains("manifest binary entries are inconsistent")
        );
    }

    #[test]
    fn manifest_requires_every_catalog_service() {
        let mut manifest = valid_manifest("a".repeat(64));
        manifest.binaries.retain(|entry| entry.service != "video");
        let error = BinaryIntegrityVerifier::new(manifest, None)
            .expect_err("manifest must include every catalog service");
        assert!(error.to_string().contains("service catalog"));
    }

    #[test]
    fn manifest_accepts_declared_service_subset() {
        let mut manifest = valid_manifest("a".repeat(64));
        manifest.binaries.retain(|entry| entry.service != "video");
        manifest.services.retain(|service| service != "video");
        BinaryIntegrityVerifier::new(manifest, None)
            .expect("a manifest may omit an undeclared optional service");
    }

    #[test]
    fn manifest_requires_every_release_tool() {
        let mut manifest = valid_manifest("a".repeat(64));
        manifest.tools.pop();
        let error = BinaryIntegrityVerifier::new(manifest, None)
            .expect_err("manifest must include every release tool");
        assert!(error.to_string().contains("release tool catalog"));
    }

    #[test]
    fn runtime_file_tampering_is_rejected_before_spawn() {
        let directory = std::env::temp_dir().join(format!(
            "a4n-runtime-integrity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let runtime = directory.join("bin/services/libonnxruntime.so.1");
        std::fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        std::fs::write(&runtime, b"runtime").unwrap();
        let mut manifest = valid_manifest("a".repeat(64));
        for tool in &mut manifest.tools {
            let path = directory.join(&tool.path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, tool.name.as_bytes()).unwrap();
            tool.sha256 = sha256(&path).unwrap();
        }
        manifest.runtime_files[0].sha256 = sha256(&runtime).unwrap();
        let verifier = BinaryIntegrityVerifier::new(manifest, Some(directory.clone())).unwrap();
        verifier.verify_runtime_files().unwrap();
        std::fs::write(&runtime, b"tampered").unwrap();
        assert!(verifier.verify_runtime_files().is_err());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn macos_tauri_manifest_paths_are_accepted() {
        let directory = std::env::temp_dir().join(format!(
            "a4n-tauri-integrity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app_root = directory.join("A4N.app");
        let macos_dir = app_root.join("Contents/MacOS");
        let resources_dir = app_root.join("Contents/Resources");
        std::fs::create_dir_all(&macos_dir).unwrap();
        std::fs::create_dir_all(&resources_dir).unwrap();

        let mut manifest = valid_manifest("a".repeat(64));
        manifest.target = "aarch64-apple-darwin".to_string();
        for binary in &mut manifest.binaries {
            binary.target = manifest.target.clone();
            binary.path = format!("A4N.app/Contents/MacOS/{}", binary.name);
        }
        for tool in &mut manifest.tools {
            tool.target = manifest.target.clone();
            tool.path = format!("A4N.app/Contents/MacOS/{}", tool.name);
            let path = directory.join(&tool.path);
            std::fs::write(&path, tool.name.as_bytes()).unwrap();
            tool.sha256 = sha256(&path).unwrap();
        }
        let runtime = resources_dir.join("libonnxruntime.dylib");
        std::fs::write(&runtime, b"runtime").unwrap();
        manifest.runtime_files[0].name = "libonnxruntime.dylib".to_string();
        manifest.runtime_files[0].path =
            "A4N.app/Contents/Resources/libonnxruntime.dylib".to_string();
        manifest.runtime_files[0].sha256 = sha256(&runtime).unwrap();

        let verifier = BinaryIntegrityVerifier::new(manifest, Some(directory.clone())).unwrap();
        verifier.verify_runtime_files().unwrap();
        let _ = std::fs::remove_dir_all(directory);
    }
}
