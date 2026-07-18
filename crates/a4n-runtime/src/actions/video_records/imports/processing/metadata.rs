use super::super::prelude::{Value, chapter_video_prompt, json};

pub(in crate::actions::video_records) fn imported_chapter_video_metadata(
    prompt: &chapter_video_prompt::Model,
    source_file_name: &str,
    source_extension: &str,
    imported_at: String,
) -> Value {
    let mut metadata = json!({
        "prompt_id": prompt.id,
        "shot_number": prompt.shot_number,
        "source_prompt_type": prompt.prompt_type,
        "import": {
            "external": true,
            "source_file_name": source_file_name,
            "source_extension": source_extension,
            "imported_at": imported_at,
        }
    });
    insert_source_context(&mut metadata, prompt.metadata_json.as_ref());
    metadata
}

pub(super) fn insert_source_context(metadata: &mut Value, source_metadata: Option<&Value>) {
    if let Some(source_context) = source_metadata
        .and_then(|metadata| metadata.get("source"))
        .cloned()
        && let Some(object) = metadata.as_object_mut()
    {
        object.insert("source".to_string(), source_context);
    }
}
