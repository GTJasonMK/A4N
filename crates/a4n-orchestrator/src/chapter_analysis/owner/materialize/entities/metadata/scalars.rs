use a4n_error::AppError;
use serde_json::Map;

use super::super::super::values::{normalize_optional_string, normalize_string_array};

pub(super) fn normalize_metadata_scalars(
    metadata: &mut Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    normalize_string_array(metadata.get_mut("tags"), "metadata.tags")?;
    normalize_optional_string(metadata.get_mut("tone"), "metadata.tone")?;
    normalize_optional_string(
        metadata.get_mut("timeline_marker"),
        "metadata.timeline_marker",
    )
}
