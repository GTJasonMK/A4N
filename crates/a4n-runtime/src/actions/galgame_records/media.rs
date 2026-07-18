mod audio;
mod binding;
mod cleanup;
mod rules;

use serde::{Deserialize, Serialize};

pub use self::audio::import_galgame_audio;
pub use self::binding::{bind_galgame_media, unbind_galgame_media};
pub(super) use self::binding::{bind_galgame_media_inner, validate_galgame_media_target};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindGalgameMediaInput {
    pub project_id: i64,
    pub media_asset_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnbindGalgameMediaInput {
    pub project_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
    pub media_asset_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportGalgameAudioInput {
    pub project_id: i64,
    pub target_line_id: i64,
    pub role: String,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[cfg(test)]
use self::audio::persist_imported_galgame_audio;
#[cfg(test)]
use self::binding::validate_galgame_media_asset_source_context;
#[cfg(test)]
use self::cleanup::append_cleanup_error;

#[cfg(test)]
mod tests;
