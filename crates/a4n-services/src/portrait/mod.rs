//! 人物参考图服务：三视图生成与管理
//!
//! 对应文档 09-角色立绘系统

use chrono::Utc;
use sea_orm::*;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::character::{character_portrait, protagonist_profile};

pub mod design_brief;

mod prompt_generation;
mod types;

pub use prompt_generation::{
    PortraitPromptGenerationInput, PortraitPromptRequestInput, PortraitPromptSpec,
    normalize_portrait_prompt_response,
};

#[cfg(test)]
use self::prompt_generation::portrait_prompt_request;
use self::types::canonical_portrait_type;
#[cfg(test)]
use self::types::portrait_type_description;

#[derive(Clone)]
pub struct PortraitService {
    db: Arc<DatabaseConnection>,
}

pub struct CreatePortraitInput<'a> {
    pub profile_id: i64,
    pub project_id: i64,
    pub portrait_type: &'a str,
    pub image_path: &'a str,
    pub prompt: &'a str,
    pub expression: Option<&'a str>,
    pub pose: Option<&'a str>,
}

impl PortraitService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list(&self, profile_id: i64) -> Result<Vec<character_portrait::Model>, AppError> {
        character_portrait::Entity::find()
            .filter(character_portrait::Column::ProfileId.eq(profile_id))
            .order_by_desc(character_portrait::Column::IsPrimary)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<character_portrait::Model>, AppError> {
        character_portrait::Entity::find()
            .filter(character_portrait::Column::ProjectId.eq(project_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn primary(
        &self,
        profile_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        character_portrait::Entity::find()
            .filter(character_portrait::Column::ProfileId.eq(profile_id))
            .filter(character_portrait::Column::IsPrimary.eq(true))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn create(
        &self,
        input: CreatePortraitInput<'_>,
    ) -> Result<character_portrait::Model, AppError> {
        let CreatePortraitInput {
            profile_id,
            project_id,
            portrait_type,
            image_path,
            prompt,
            expression,
            pose,
        } = input;
        validate_positive_i64(profile_id, "profile_id")?;
        validate_positive_i64(project_id, "project_id")?;
        let profile = protagonist_profile::Entity::find_by_id(profile_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {profile_id}")))?;
        if profile.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "角色档案 {profile_id} 不属于项目 {project_id}"
            )));
        }
        let portrait_type = canonical_portrait_type(portrait_type)?;
        let image_path = required_trimmed(image_path, "image_path")?;
        let prompt = required_trimmed(prompt, "prompt")?;
        let expression = optional_trimmed(expression);
        let pose = optional_trimmed(pose);

        let now = Utc::now().naive_utc();
        let existing = self.list(profile_id).await?;
        let is_primary = existing.is_empty();

        character_portrait::ActiveModel {
            profile_id: Set(profile_id),
            project_id: Set(project_id),
            portrait_type: Set(portrait_type.to_string()),
            image_path: Set(image_path),
            prompt: Set(prompt),
            expression: Set(expression),
            pose: Set(pose),
            is_primary: Set(is_primary),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn set_primary(&self, portrait_id: i64) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let portrait = character_portrait::Entity::find_by_id(portrait_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("立绘不存在".into()))?;

        character_portrait::Entity::update_many()
            .col_expr(
                character_portrait::Column::IsPrimary,
                sea_orm::sea_query::Expr::value(false),
            )
            .filter(character_portrait::Column::ProfileId.eq(portrait.profile_id))
            .filter(character_portrait::Column::IsPrimary.eq(true))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;

        let mut active: character_portrait::ActiveModel = portrait.into();
        active.is_primary = Set(true);
        active.update(&txn).await.map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }
}

fn required_trimmed(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn optional_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn validate_positive_i64(value: i64, field: &str) -> Result<i64, AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_engine::prompt::tera_engine::TeraPromptEngine;
    use sea_orm::{EntityTrait, QueryFilter, QueryOrder, Set};
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    fn service(db: DatabaseConnection) -> PortraitService {
        PortraitService::new(Arc::new(db))
    }

    fn prompts_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../prompts")
    }

    fn prompt_engine() -> TeraPromptEngine {
        TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
    }

    async fn insert_profile(
        db: &DatabaseConnection,
        project_id: i64,
    ) -> protagonist_profile::Model {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        protagonist_profile::ActiveModel {
            project_id: Set(project_id),
            name: Set("林烬".to_string()),
            gender: Set(None),
            age: Set(None),
            base_attributes: Set(json!({})),
            current_state: Set(json!({})),
            character_arc: Set(None),
            voice_fingerprint: Set(None),
            appearance: Set(None),
            background: Set(None),
            personality: Set(None),
            last_synced_chapter: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn create_portrait(
        service: &PortraitService,
        profile: &protagonist_profile::Model,
        portrait_type: &str,
        image_path: &str,
    ) -> character_portrait::Model {
        service
            .create(CreatePortraitInput {
                profile_id: profile.id,
                project_id: profile.project_id,
                portrait_type,
                image_path,
                prompt: " portrait prompt ",
                expression: Some(" calm "),
                pose: Some(" standing "),
            })
            .await
            .unwrap()
    }

    async fn portrait_count(db: &DatabaseConnection) -> u64 {
        character_portrait::Entity::find().count(db).await.unwrap()
    }

    fn prompt_spec<'a>(profile_name: &'a str, portrait_type: &'a str) -> PortraitPromptSpec<'a> {
        PortraitPromptSpec {
            profile_name,
            appearance: None,
            gender: None,
            age: None,
            personality: None,
            background: None,
            profile_context: None,
            project_context: None,
            portrait_type,
            art_style: None,
            expression: None,
            existing_portrait_count: None,
        }
    }

    #[test]
    fn portrait_prompt_request_requires_chinese_prompt() {
        let engine = prompt_engine();
        let prompt = portrait_prompt_request(
            1,
            PortraitPromptSpec {
                appearance: Some("短发"),
                gender: Some("女"),
                age: Some("二十岁"),
                ..prompt_spec(
                    "林岚",
                    "人物三视图与拆解设定图（正面、侧面、背面，全身，含服饰配饰拆解）",
                )
            },
            &engine,
        )
        .expect("prompt should serialize");

        assert!(prompt.contains("中文人物三视图与人物拆解设定图 prompt"));
        assert!(prompt.contains(r#""image_goal": "单张人物三视图与人物拆解设定图"#));
        assert!(prompt.contains("正面全身视图"));
        assert!(prompt.contains("侧面全身视图"));
        assert!(prompt.contains("背面全身视图"));
        assert!(prompt.contains(r#""breakdown_sections""#));
        assert!(prompt.contains(r#""layout_spec""#));
        assert!(prompt.contains("服饰层级拆解"));
        assert!(prompt.contains("鞋履拆解"));
        assert!(prompt.contains("材质与色板拆解"));
        assert!(prompt.contains("至少包含服饰层级、发型、鞋履、材质色板四类视觉拆解"));
        assert!(prompt.contains("最终图像不能只有三视图"));
        assert!(prompt.contains("档案明确出现的项链"));
        assert!(prompt.contains("不要硬塞项链、戒指、武器等具体配饰"));
        assert!(!prompt.contains("英文"));
    }

    #[test]
    fn normalize_portrait_prompt_response_rejects_structured_or_fenced_output() {
        assert!(normalize_portrait_prompt_response("  高质量中文人物设定图提示词  ").is_ok());
        for raw in [
            r#"{"prompt":"高质量中文人物设定图提示词"}"#,
            r#"[{"prompt":"高质量中文人物设定图提示词"}]"#,
            "```text\n高质量中文人物设定图提示词\n```",
            "# 人物设定图提示词\n高质量中文人物设定图提示词",
            "<think>先分析</think>\n高质量中文人物设定图提示词",
        ] {
            let err =
                normalize_portrait_prompt_response(raw).expect_err("structured output must fail");
            assert!(err.to_string().contains("必须是纯 prompt 文本"));
        }
    }

    async fn primary_ids(db: &DatabaseConnection, profile_id: i64) -> Vec<i64> {
        character_portrait::Entity::find()
            .filter(character_portrait::Column::ProfileId.eq(profile_id))
            .filter(character_portrait::Column::IsPrimary.eq(true))
            .order_by_asc(character_portrait::Column::Id)
            .all(db)
            .await
            .unwrap()
            .into_iter()
            .map(|portrait| portrait.id)
            .collect()
    }

    #[test]
    fn portrait_type_description_rejects_unknown_type() {
        let err = portrait_type_description("headshot").expect_err("unknown type must fail");
        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn portrait_prompt_request_omits_missing_appearance() {
        let engine = prompt_engine();
        let prompt = portrait_prompt_request(
            1,
            prompt_spec(
                "林烬",
                "人物三视图与拆解设定图（正面、侧面、背面，全身，含服饰配饰拆解）",
            ),
            &engine,
        )
        .expect("prompt should serialize");

        assert!(prompt.contains(r#""character_name": "林烬""#));
        assert!(prompt.contains(
            r#""portrait_type": "人物三视图与拆解设定图（正面、侧面、背面，全身，含服饰配饰拆解）""#
        ));
        assert!(prompt.contains(r#""appearance": null"#));
        assert!(prompt.contains(r#""gender": null"#));
        assert!(!prompt.contains(r#""appearance": "外观描述""#));
    }

    #[test]
    fn portrait_prompt_request_omits_blank_appearance() {
        let engine = prompt_engine();
        let prompt = portrait_prompt_request(
            1,
            PortraitPromptSpec {
                appearance: Some(" \n\t "),
                gender: Some(" \n\t "),
                ..prompt_spec(
                    "林烬",
                    "人物三视图与拆解设定图（正面、侧面、背面，全身，含服饰配饰拆解）",
                )
            },
            &engine,
        )
        .expect("prompt should serialize");

        assert!(prompt.contains(r#""appearance": null"#));
        assert!(prompt.contains(r#""gender": null"#));
        assert!(!prompt.contains(r#""appearance": "外观描述""#));
    }

    #[test]
    fn portrait_prompt_request_includes_explicit_appearance() {
        let engine = prompt_engine();
        let profile_context = json!({"current_state":{"explicit_attributes":{"衣着":"青色官袍"}}});
        let project_context = json!({"genre":"历史穿越","tone":"紧张克制"});
        let prompt = portrait_prompt_request(
            1,
            PortraitPromptSpec {
                appearance: Some(" 灰色制服 "),
                gender: Some(" 男 "),
                age: Some(" 二十岁 "),
                personality: Some(" 冷静克制 "),
                background: Some(" 东宫书吏 "),
                profile_context: Some(&profile_context),
                project_context: Some(&project_context),
                art_style: Some(" 水彩风格 "),
                expression: Some(" calm "),
                existing_portrait_count: Some(2),
                ..prompt_spec(
                    "林烬",
                    "人物三视图与拆解设定图（正面、侧面、背面，重点拆解服饰层级与配饰细节）",
                )
            },
            &engine,
        )
        .expect("prompt should serialize");

        assert!(prompt.contains(r#""character_name": "林烬""#));
        assert!(prompt.contains(
            r#""portrait_type": "人物三视图与拆解设定图（正面、侧面、背面，重点拆解服饰层级与配饰细节）""#
        ));
        assert!(prompt.contains(r#""appearance": "灰色制服""#));
        assert!(prompt.contains(r#""gender": "男""#));
        assert!(prompt.contains(r#""age": "二十岁""#));
        assert!(prompt.contains(r#""personality": "冷静克制""#));
        assert!(prompt.contains(r#""background": "东宫书吏""#));
        assert!(prompt.contains(r#""profile_context""#));
        assert!(prompt.contains("portrait_profile_project_context.profile_context"));
        assert!(!prompt.contains("青色官袍"));
        assert!(prompt.contains(r#""project_context""#));
        assert!(prompt.contains("portrait_profile_project_context.project_context"));
        assert!(!prompt.contains("历史穿越"));
        assert!(prompt.contains(r#""art_style": "水彩风格""#));
        assert!(prompt.contains(r#""expression": "calm""#));
        assert!(prompt.contains(r#""existing_portrait_count": 2"#));
    }

    #[tokio::test]
    async fn create_rejects_project_mismatch_before_insert() {
        let db = setup_db().await;
        let profile = insert_profile(&db, 1).await;
        let service = service(db.clone());

        let err = service
            .create(CreatePortraitInput {
                profile_id: profile.id,
                project_id: 2,
                portrait_type: "full_body",
                image_path: "portraits/lin.png",
                prompt: "portrait prompt",
                expression: None,
                pose: None,
            })
            .await
            .expect_err("portrait project must match profile project");

        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message.contains("不属于项目 2")
        ));
        assert_eq!(portrait_count(&db).await, 0);
    }

    #[tokio::test]
    async fn create_rejects_unsupported_type_and_blank_path_before_insert() {
        let db = setup_db().await;
        let profile = insert_profile(&db, 1).await;
        let service = service(db.clone());

        let unsupported = service
            .create(CreatePortraitInput {
                profile_id: profile.id,
                project_id: profile.project_id,
                portrait_type: "headshot",
                image_path: "portraits/lin.png",
                prompt: "portrait prompt",
                expression: None,
                pose: None,
            })
            .await
            .expect_err("unsupported portrait type should fail");
        let blank_path = service
            .create(CreatePortraitInput {
                profile_id: profile.id,
                project_id: profile.project_id,
                portrait_type: "full_body",
                image_path: "  ",
                prompt: "portrait prompt",
                expression: None,
                pose: None,
            })
            .await
            .expect_err("blank image path should fail");

        assert!(matches!(unsupported, AppError::InvalidArgument(_)));
        assert!(matches!(
            blank_path,
            AppError::InvalidArgument(message) if message.contains("image_path 不能为空")
        ));
        assert_eq!(portrait_count(&db).await, 0);
    }

    #[tokio::test]
    async fn create_normalizes_fields_and_marks_first_portrait_primary() {
        let db = setup_db().await;
        let profile = insert_profile(&db, 1).await;
        let service = service(db.clone());

        let portrait = service
            .create(CreatePortraitInput {
                profile_id: profile.id,
                project_id: profile.project_id,
                portrait_type: " half_body ",
                image_path: " portraits/lin.png ",
                prompt: " portrait prompt ",
                expression: Some("  "),
                pose: Some(" standing "),
            })
            .await
            .expect("valid portrait should be inserted");

        assert_eq!(portrait.portrait_type, "half_body");
        assert_eq!(portrait.image_path, "portraits/lin.png");
        assert_eq!(portrait.prompt, "portrait prompt");
        assert_eq!(portrait.expression, None);
        assert_eq!(portrait.pose, Some("standing".to_string()));
        assert!(portrait.is_primary);
        assert_eq!(primary_ids(&db, profile.id).await, vec![portrait.id]);
    }

    #[tokio::test]
    async fn set_primary_portrait_leaves_exactly_one_primary() {
        let db = setup_db().await;
        let profile = insert_profile(&db, 1).await;
        let service = service(db.clone());
        let first = create_portrait(&service, &profile, "full_body", "portraits/1.png").await;
        let second = create_portrait(&service, &profile, "half_body", "portraits/2.png").await;

        service
            .set_primary(second.id)
            .await
            .expect("second portrait should become primary");

        assert_ne!(first.id, second.id);
        assert_eq!(primary_ids(&db, profile.id).await, vec![second.id]);
    }
}
