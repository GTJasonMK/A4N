use std::collections::HashMap;
use std::sync::Arc;

use a4n_import_client::ImportClient;
use a4n_import_contract::{
    IMPORT_TRANSFORM_CAPABILITY_VERSION, ImportReadMediaPackageZipInput,
    ImportReadMediaPackageZipOutput,
};
use serde_json::Value;

use crate::state::RuntimeProfile;

use super::super::{
    pure_transform_typed_service_request, service_error_string, typed_response_payload,
};

pub(crate) async fn read_import_media_package_zip(
    profile: RuntimeProfile,
    client: Option<Arc<dyn ImportClient>>,
    zip_bytes: Vec<u8>,
    manifest_path: &str,
    package_label: &str,
    manifest_label: &str,
) -> Result<(Value, HashMap<String, Vec<u8>>), String> {
    if profile.uses_sidecars() {
        let client = client
            .ok_or_else(|| "import sidecar capability 未配置，无法读取媒体项目包".to_string())?;
        let input = ImportReadMediaPackageZipInput::from_zip_bytes(
            zip_bytes,
            manifest_path,
            package_label,
            manifest_label,
        );
        let response = client
            .read_media_package_zip(pure_transform_typed_service_request(
                IMPORT_TRANSFORM_CAPABILITY_VERSION,
                input,
                None,
                None,
            )?)
            .await
            .map_err(service_error_string)?;
        let output: ImportReadMediaPackageZipOutput = typed_response_payload(response, "import")?;
        return output
            .into_manifest_and_entries_bytes()
            .map_err(|err| err.to_string());
    }

    a4n_import_core::read_media_package_zip_bytes(
        &zip_bytes,
        manifest_path,
        package_label,
        manifest_label,
    )
    .map_err(|err| err.to_string())
}
