use a4n_api_contract::command_manifest::{
    API_COMMANDS, COMMAND_MANIFEST_VERSION, CommandDescriptor, CommandManifest, HOST_IDENTITY,
    command_descriptor, command_manifest,
};
use a4n_api_contract::package::{PackageCatalog, package_catalog};
use axum::Json;
use axum::extract::Path;
use serde_json::{Value, json};

use super::error::ApiError;

pub(super) async fn get() -> Json<Value> {
    Json(json!({
        "host": HOST_IDENTITY,
        "server_version": env!("CARGO_PKG_VERSION"),
        "api_contract_version": 1,
        "command_manifest_version": COMMAND_MANIFEST_VERSION,
        "command_count": API_COMMANDS.len(),
        "transports": ["http", "sse"],
        "features": ["host-api", "generation-tasks", "media", "command-discovery"],
    }))
}

pub(super) async fn commands() -> Json<CommandManifest> {
    Json(command_manifest(None, None, None))
}

pub(super) async fn packages() -> Json<PackageCatalog> {
    Json(package_catalog())
}

pub(super) async fn command(
    Path(command): Path<String>,
) -> Result<Json<CommandDescriptor>, ApiError> {
    command_descriptor(&command)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("未知 Host API 命令: {command}")))
}
