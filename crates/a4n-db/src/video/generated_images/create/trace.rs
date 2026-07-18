use super::super::VideoDbError;
use a4n_core::models::ai::generated_image;
use serde_json::{Value, json};

pub(crate) fn image_generation_response_summary(
    summary: Option<Value>,
    image: &generated_image::Model,
) -> Result<Option<Value>, VideoDbError> {
    let generated = json!({
        "generated_image_id": image.id,
        "file_path": &image.file_path,
        "model": &image.model,
        "file_size": image.file_size,
    });
    match summary {
        Some(Value::Object(mut map)) => {
            if let Some(generated) = generated.as_object() {
                for (key, value) in generated {
                    map.insert(key.clone(), value.clone());
                }
            }
            Ok(Some(Value::Object(map)))
        }
        Some(_) => Err(VideoDbError::InvalidArgument(
            "图片生成运行记录 response_summary_json 必须是对象".to_string(),
        )),
        None => Ok(Some(generated)),
    }
}

pub(crate) fn image_generation_parsed_output(
    parsed_output: Option<Value>,
    image: &generated_image::Model,
) -> Result<Option<Value>, VideoDbError> {
    let generated = json!({
        "generated_image_id": image.id,
        "project_id": image.project_id,
        "image_type": &image.image_type,
        "file_path": &image.file_path,
        "prompt": &image.prompt,
        "model": &image.model,
        "width": image.width,
        "height": image.height,
        "file_size": image.file_size,
        "chapter_id": image.chapter_id,
        "panel_id": &image.panel_id,
        "metadata": &image.metadata_json,
    });
    match parsed_output {
        Some(Value::Object(mut map)) => {
            map.insert("generated_image".to_string(), generated);
            Ok(Some(Value::Object(map)))
        }
        Some(_) => Err(VideoDbError::InvalidArgument(
            "图片生成运行记录 parsed_output_json 必须是对象".to_string(),
        )),
        None => Ok(Some(generated)),
    }
}
