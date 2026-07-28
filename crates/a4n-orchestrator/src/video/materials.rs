use serde_json::{Map, Value};

use crate::profile_material;

pub fn is_project_reusable_video_material_image(
    image_type: &str,
    metadata: Option<&Value>,
) -> bool {
    profile_material::is_project_reusable_video_material_image(image_type, metadata)
}

pub(crate) fn is_video_frame_input_metadata(metadata: &Map<String, Value>) -> bool {
    profile_material::is_video_frame_input_metadata(Some(&Value::Object(metadata.clone())))
}
