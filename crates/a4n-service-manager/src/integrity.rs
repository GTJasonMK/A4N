use std::collections::{HashMap, HashSet};
use std::env::VarError;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use a4n_service_protocol::{
    PROTOCOL_VERSION, RELEASE_MANIFEST_ENV, ReleaseBinaryManifest, ReleaseManifest,
    ReleaseRuntimeFileManifest, service_catalog,
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
        let path = match std::env::var(RELEASE_MANIFEST_ENV) {
            Ok(path) if path.trim().is_empty() => {
                return Err(ServiceManagerError::ReleaseManifestInvalid(
                    "manifest path is blank".to_string(),
                ));
            }
            Ok(path) => PathBuf::from(path),
            Err(VarError::NotPresent) => return Ok(None),
            Err(_) => {
                return Err(ServiceManagerError::ReleaseManifestInvalid(
                    "manifest path is unreadable".to_string(),
                ));
            }
        };
        let package_root = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let bytes = std::fs::read(&path).map_err(|_| {
            ServiceManagerError::ReleaseManifestInvalid("manifest is unavailable".to_string())
        })?;
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
        if manifest.schema_version != 2
            || manifest.protocol_version != PROTOCOL_VERSION
            || manifest.package.trim().is_empty()
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
        let expected_services = std::iter::once("host".to_string())
            .chain(catalog.iter().map(|entry| entry.name.clone()))
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
        let host = by_service.get("host").expect("host is checked above");
        if host.capability_version.is_some()
            || !host.path.starts_with("bin/")
            || manifest_file_name(&host.path) != Some(host.name.as_str())
        {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest host entry is invalid".to_string(),
            ));
        }
        for entry in catalog {
            let binary_name = format!("{}{extension}", entry.binary);
            let binary = by_service
                .get(&entry.name)
                .expect("catalog service is checked above");
            if binary.name != binary_name
                || binary.path != format!("bin/services/{binary_name}")
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
            || runtime.path != format!("bin/services/{expected_runtime_name}")
            || !valid_manifest_path(&runtime.path)
            || manifest_file_name(&runtime.path) != Some(runtime.name.as_str())
            || !is_sha256(&runtime.sha256)
        {
            return Err(ServiceManagerError::ReleaseManifestInvalid(
                "manifest runtime file is inconsistent".to_string(),
            ));
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
        for entry in &self.runtime_files {
            let path = package_root.join(&entry.path);
            let actual = sha256(&path).map_err(|_| ServiceManagerError::BinaryIntegrityFailed {
                service: entry.name.clone(),
                reason: "runtime file cannot be hashed".to_string(),
            })?;
            if !actual.eq_ignore_ascii_case(&entry.sha256) {
                return Err(ServiceManagerError::BinaryIntegrityFailed {
                    service: entry.name.clone(),
                    reason: "runtime file SHA-256 mismatch".to_string(),
                });
            }
        }
        Ok(())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_manifest_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\0')
        && !Path::new(path).is_absolute()
        && !path
            .split(|character| character == '/' || character == '\\')
            .any(|part| part == "..")
}

fn manifest_file_name(path: &str) -> Option<&str> {
    path.rsplit(|character| character == '/' || character == '\\')
        .find(|part| !part.is_empty())
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
    use a4n_service_protocol::{
        PROTOCOL_VERSION, ReleaseBinaryManifest, ReleaseManifest, ReleaseRuntimeFileManifest,
        service_catalog,
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
        ReleaseManifest {
            schema_version: 2,
            package: "a4n-tauri".to_string(),
            version: "0.1.0".to_string(),
            target: "linux-x86_64".to_string(),
            protocol_version: PROTOCOL_VERSION,
            binaries,
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
        manifest.runtime_files[0].sha256 = sha256(&runtime).unwrap();
        let verifier = BinaryIntegrityVerifier::new(manifest, Some(directory.clone())).unwrap();
        verifier.verify_runtime_files().unwrap();
        std::fs::write(&runtime, b"tampered").unwrap();
        assert!(verifier.verify_runtime_files().is_err());
        let _ = std::fs::remove_dir_all(directory);
    }
}
