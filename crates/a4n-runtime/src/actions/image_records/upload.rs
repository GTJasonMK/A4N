mod file;
mod metadata;
mod record;

use crate::{
    AppState,
    actions::{access, image_generation::GeneratedImageSummary, uploaded_image},
};

use super::{IMPORTED_IMAGE_MODEL, UploadImageInput};

const MAX_IMPORT_IMAGE_BYTES: usize = 50 * 1024 * 1024;
pub(crate) type ImportedImageFile = uploaded_image::DecodedUploadedImage;

pub async fn upload_image(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let image_type = super::rules::required_text(&input.image_type, "图片类型")?;
    let prompt = super::rules::required_text(&input.prompt, "图片提示词")?;
    let model = match input.model.as_deref() {
        Some(value) => super::rules::required_text(value, "图片模型")?,
        None => IMPORTED_IMAGE_MODEL.to_string(),
    };
    let imported = read_uploaded_image(&input)?;
    record::create_imported_image_record(
        state,
        project_id,
        record::ImportedImageRecordInput {
            image_type,
            prompt,
            model,
            chapter_id: input.chapter_id,
            panel_id: input.panel_id,
            metadata_json: input.metadata_json,
            imported,
        },
    )
    .await
}

fn read_uploaded_image(input: &UploadImageInput) -> Result<ImportedImageFile, String> {
    read_uploaded_image_data(&input.file_name, &input.mime_type, &input.data_base64)
}

pub(crate) fn read_uploaded_image_data(
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
) -> Result<ImportedImageFile, String> {
    uploaded_image::decode_uploaded_image_data(
        file_name,
        mime_type,
        data_base64,
        MAX_IMPORT_IMAGE_BYTES,
    )
}
