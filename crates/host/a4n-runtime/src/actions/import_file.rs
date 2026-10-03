mod upload;

use upload::decode_import_upload;

use a4n_error::AppError;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::AppState;

use a4n_api_contract::import::ImportProjectTextUploadInput;
use a4n_api_contract::canvas::{CanvasTextAssetInput, CanvasTextAssetSummary, SaveCanvasTextAssetInput};
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

pub async fn import_project_text_asset_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectTextUploadInput,
) -> Result<CanvasTextAssetSummary, AppError> {
    let upload = decode_import_upload(input).map_err(AppError::InvalidArgument)?;
    let parsed = crate::sidecar_clients::parse_import_document(
        state.runtime_profile(),
        state.capabilities().import.clone(),
        a4n_import_contract::ImportParseDocumentInput {
            file_type: upload.file_type,
            bytes_base64: BASE64.encode(&upload.file_bytes),
            document_title: upload.file_name.clone(),
        },
    )
    .await?;
    if parsed.chapters.is_empty() {
        return Err(AppError::InvalidArgument(
            "导入文件没有可用的正文".to_string(),
        ));
    }
    let content = parsed
        .chapters
        .into_iter()
        .map(|chapter| {
            let title = chapter.title.trim();
            if title.is_empty() {
                chapter.content.trim().to_string()
            } else {
                format!("# {title}\n\n{}", chapter.content.trim())
            }
        })
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "导入文件没有可用的正文".to_string(),
        ));
    }
    state
        .canvas_mutation_application()
        .save_text_asset(
            user_id,
            SaveCanvasTextAssetInput {
                project_id: upload.project_id,
                text_asset: CanvasTextAssetInput {
                    id: None,
                    expected_version: None,
                    title: upload.file_name,
                    text_kind: "script".to_string(),
                    content,
                },
            },
        )
        .await
}
