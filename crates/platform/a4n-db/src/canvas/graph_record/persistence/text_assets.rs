use a4n_db_models::models::canvas::canvas_text_asset;
use a4n_domain::canvas::CanvasTextAssetRecord;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use sha2::{Digest, Sha256};

use super::super::validation::required_canvas_record_text;
use super::context::require_canvas_project_record;
use crate::canvas::CanvasDbError;
use crate::canvas::records::canvas_text_asset_record;
use a4n_domain::canvas::CanvasTextAssetRecordInput;

pub async fn upsert_canvas_text_asset_record<C>(
    db: &C,
    project_id: i64,
    input: CanvasTextAssetRecordInput,
    now: chrono::NaiveDateTime,
) -> Result<CanvasTextAssetRecord, CanvasDbError>
where
    C: ConnectionTrait,
{
    require_canvas_project_record(db, project_id).await?;
    let title = required_canvas_record_text(&input.title, "title")?;
    let text_kind = normalize_canvas_record_text_kind(&input.text_kind)?;
    let content = input.content.trim().to_string();
    if content.is_empty() {
        return Err(CanvasDbError::InvalidArgument(
            "content 不能为空".to_string(),
        ));
    }
    let content_hash = canvas_record_sha256_hex(content.as_bytes());
    match input.id {
        Some(id) => {
            let existing = canvas_text_asset::Entity::find_by_id(id)
                .one(db)
                .await?
                .ok_or_else(|| CanvasDbError::NotFound(format!("画布文本资产不存在: {id}")))?;
            if existing.project_id != project_id {
                return Err(CanvasDbError::Forbidden(
                    "文本资产不属于当前项目".to_string(),
                ));
            }
            let mut active: canvas_text_asset::ActiveModel = existing.into();
            active.title = Set(title);
            active.text_kind = Set(text_kind.to_string());
            active.content = Set(content);
            active.content_hash = Set(content_hash);
            active.updated_at = Set(now);
            active
                .update(db)
                .await
                .map(canvas_text_asset_record)
                .map_err(CanvasDbError::Database)
        }
        None => canvas_text_asset::ActiveModel {
            project_id: Set(project_id),
            title: Set(title),
            text_kind: Set(text_kind.to_string()),
            content: Set(content),
            content_hash: Set(content_hash),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map(canvas_text_asset_record)
        .map_err(CanvasDbError::Database),
    }
}

fn normalize_canvas_record_text_kind(value: &str) -> Result<&'static str, CanvasDbError> {
    match value.trim() {
        "script" => Ok("script"),
        "shot_note" => Ok("shot_note"),
        "dialogue" => Ok("dialogue"),
        "narration" => Ok("narration"),
        "character_note" => Ok("character_note"),
        "freeform" => Ok("freeform"),
        "canvas_output" => Ok("canvas_output"),
        other => Err(CanvasDbError::InvalidArgument(format!(
            "未知画布文本类型: {other}"
        ))),
    }
}

fn canvas_record_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::normalize_canvas_record_text_kind;

    #[test]
    fn canvas_output_is_a_canonical_canvas_text_kind() {
        assert_eq!(
            normalize_canvas_record_text_kind(" canvas_output ").unwrap(),
            "canvas_output"
        );
    }

    #[test]
    fn unknown_canvas_text_kind_is_rejected() {
        let error =
            normalize_canvas_record_text_kind("legacy_output").expect_err("未知文本类型必须失败");
        assert!(error.to_string().contains("未知画布文本类型"));
    }
}
