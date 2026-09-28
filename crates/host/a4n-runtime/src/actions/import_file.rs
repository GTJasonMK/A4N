mod upload;

use upload::decode_import_upload;

use a4n_error::AppError;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::AppState;

use a4n_api_contract::import::ImportProjectTextUploadInput;
use a4n_api_contract::import_review::ImportReviewSummary;
use a4n_domain::import_review::ImportChapterDraft;

pub async fn import_project_text_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectTextUploadInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    let upload = decode_import_upload(input).map_err(AppError::InvalidArgument)?;
    let chapters = crate::sidecar_clients::parse_import_document(
        state.runtime_profile(),
        state.capabilities().import.clone(),
        a4n_import_contract::ImportParseDocumentInput {
            file_type: upload.file_type,
            bytes_base64: BASE64.encode(&upload.file_bytes),
            document_title: upload.file_name.clone(),
        },
    )
    .await?
    .chapters;
    let chapters = chapters
        .into_iter()
        .map(|chapter| ImportChapterDraft {
            number: chapter.number,
            title: chapter.title,
            content: chapter.content,
        })
        .collect::<Vec<_>>();
    state
        .import_review_ingestion_application()
        .create_from_parsed(user_id, upload.project_id, &upload.file_name, &chapters)
        .await
}
