use a4n_db::character::{
    AppliedCharacterImplicitDecisionAction, AppliedCharacterImplicitDecisionRecord,
};
use a4n_db_models::models::character::{attribute_event, protagonist_profile};
use a4n_error::AppError;

use super::{CharacterAttributeLifecycleError, implicit_review_attribute_event};

fn profile() -> protagonist_profile::Model {
    let now = chrono::Utc::now().naive_utc();
    protagonist_profile::Model {
        id: 7,
        project_id: 3,
        name: "林岚".to_string(),
        gender: None,
        age: None,
        base_attributes: serde_json::json!({}),
        current_state: serde_json::json!({}),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter: 2,
        created_at: now,
        updated_at: now,
    }
}

fn event() -> attribute_event::Model {
    attribute_event::Model {
        id: 11,
        profile_id: 7,
        chapter_number: 3,
        source_kind: attribute_event::SOURCE_KIND_AI_SYNC.to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        category: "implicit".to_string(),
        key: "谨慎".to_string(),
        operation: "modify".to_string(),
        change_description: "隐性属性复审更新".to_string(),
        old_value: serde_json::json!("观望"),
        new_value: serde_json::json!("审慎行动"),
        cause: "隐性属性复审".to_string(),
        evidence: "确认风险后采取行动".to_string(),
    }
}

#[test]
fn lifecycle_error_preserves_persistence_and_rag_messages() {
    let persistence = CharacterAttributeLifecycleError::Persistence(AppError::InvalidArgument(
        "属性 key 不能为空".to_string(),
    ));
    assert_eq!(persistence.to_string(), "参数无效：属性 key 不能为空");

    let rag = CharacterAttributeLifecycleError::rag("向量写入失败");
    assert_eq!(
        rag.to_string(),
        "角色属性事件写入后同步 RAG 失败: 向量写入失败"
    );
}

#[test]
fn implicit_review_syncs_rag_only_when_db_record_contains_event() {
    let written_event = event();
    let modified = AppliedCharacterImplicitDecisionRecord {
        profile: profile(),
        action: AppliedCharacterImplicitDecisionAction::Modify,
        event: Some(written_event.clone()),
        deletion_mark: None,
    };
    assert_eq!(
        implicit_review_attribute_event(&modified).map(|event| event.id),
        Some(written_event.id)
    );

    let kept = AppliedCharacterImplicitDecisionRecord {
        profile: profile(),
        action: AppliedCharacterImplicitDecisionAction::Keep,
        event: None,
        deletion_mark: None,
    };
    assert!(implicit_review_attribute_event(&kept).is_none());
}
