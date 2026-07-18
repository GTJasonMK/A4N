//! 角色快照差异投影。

use a4n_core::error::AppError;
use serde_json::{Value, json};

mod attributes;
mod summary;

use attributes::{diff_attributes, required_category_attributes};
use summary::snapshot_diff_summary;

pub(super) fn diff_character_snapshot(from: &Value, to: &Value) -> Result<Option<Value>, AppError> {
    let has_character_attrs = from.get("attributes").is_some() || to.get("attributes").is_some();
    if !has_character_attrs {
        return Ok(None);
    }

    let explicit = diff_attributes(
        required_category_attributes(from, "explicit", "from")?,
        required_category_attributes(to, "explicit", "to")?,
    );
    let implicit = diff_attributes(
        required_category_attributes(from, "implicit", "from")?,
        required_category_attributes(to, "implicit", "to")?,
    );
    let social = diff_attributes(
        required_category_attributes(from, "social", "from")?,
        required_category_attributes(to, "social", "to")?,
    );

    Ok(Some(json!({
        "attributes": {
            "explicit": explicit,
            "implicit": implicit,
            "social": social,
        },
        "summary": snapshot_diff_summary(from, to),
    })))
}
