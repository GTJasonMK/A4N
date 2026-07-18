use crate::error::AppError;
use serde_json::{Value, json};

pub const PROMPT_OUTPUT_LANGUAGE_METADATA_KEY: &str = "prompt_output_language";
pub const PROMPT_OUTPUT_LANGUAGE_ZH_CN: &str = "zh_cn";
pub const PROMPT_OUTPUT_LANGUAGE_EN: &str = "en";

pub fn prompt_type_uses_prompt_output_language(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt"
            | "first_frame_prompt"
            | "chat_video_frame_prompt"
            | "storyboard_prompt"
            | "video_prompt"
    )
}

pub fn prompt_output_language_code_from_metadata(metadata: Option<&Value>) -> &'static str {
    match metadata
        .and_then(|metadata| metadata.get(PROMPT_OUTPUT_LANGUAGE_METADATA_KEY))
        .and_then(Value::as_str)
        .map(str::trim)
    {
        Some(PROMPT_OUTPUT_LANGUAGE_EN) => PROMPT_OUTPUT_LANGUAGE_EN,
        _ => PROMPT_OUTPUT_LANGUAGE_ZH_CN,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptOutputLanguage {
    ZhCn,
    En,
}

impl PromptOutputLanguage {
    pub fn from_optional_code(code: Option<&str>) -> Result<Self, AppError> {
        match code.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some(PROMPT_OUTPUT_LANGUAGE_ZH_CN) => Ok(Self::ZhCn),
            Some(PROMPT_OUTPUT_LANGUAGE_EN) => Ok(Self::En),
            Some(other) => Err(AppError::InvalidArgument(format!(
                "prompt_output_language 无效：{other}；允许值：zh_cn/en"
            ))),
        }
    }

    pub fn from_metadata(metadata: Option<&Value>) -> Self {
        match prompt_output_language_code_from_metadata(metadata) {
            PROMPT_OUTPUT_LANGUAGE_EN => Self::En,
            PROMPT_OUTPUT_LANGUAGE_ZH_CN => Self::ZhCn,
            _ => unreachable!("shared prompt language contract returned an unknown code"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ZhCn => PROMPT_OUTPUT_LANGUAGE_ZH_CN,
            Self::En => PROMPT_OUTPUT_LANGUAGE_EN,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ZhCn => "简体中文",
            Self::En => "English",
        }
    }

    pub fn instruction(self) -> &'static str {
        match self {
            Self::ZhCn => {
                "最终提交给图片/视频生成模型的自然语言提示词必须使用简体中文；角色名、地名、专有名词、参考图编号、用户提供的英文风格关键词和原文对白/台词短句必须保留原文写法，不要擅自翻译。JSON key、内部字段名和结构化审计字段不要翻译。"
            }
            Self::En => {
                "Final natural-language prompts sent to image/video generation models must be written in English. Preserve character names, place names, proper nouns, reference labels, user-provided style keywords, and original dialogue/quoted lines exactly as written; do not translate Chinese dialogue. Do not translate JSON keys, internal field names, or structured audit fields."
            }
        }
    }

    pub fn visual_sections(self) -> &'static [&'static str] {
        match self {
            Self::ZhCn => &[
                "[主体定义与素材绑定]",
                "[片段内容以及前因后果]",
                "[镜头分镜]",
                "[画面风格描述]",
                "[连续性与结尾状态]",
                "[限制与避免项]",
            ],
            Self::En => &[
                "[Subject And Asset Binding]",
                "[Segment Context]",
                "[Shot Storyboard]",
                "[Visual Style]",
                "[Continuity And End State]",
                "[Constraints And Avoidances]",
            ],
        }
    }

    pub fn motion_sections(self) -> &'static [&'static str] {
        match self {
            Self::ZhCn => &["[运镜与动作]", "[对白与表演]"],
            Self::En => &["[Camera And Action]", "[Dialogue And Performance]"],
        }
    }

    pub fn frame_sections(self) -> &'static [&'static str] {
        match self {
            Self::ZhCn => &[
                "【参考图约束】",
                "【画面主体】",
                "【空间与道具】",
                "【构图光线风格】",
                "【一致性与避免项】",
            ],
            Self::En => &[
                "[Reference Constraints]",
                "[Main Subject]",
                "[Space And Props]",
                "[Composition Lighting Style]",
                "[Continuity And Avoidances]",
            ],
        }
    }

    pub fn density_rule(self) -> &'static str {
        match self {
            Self::ZhCn => {
                "中文输出密度：视频 visual_prompt 至少 80 个字符，motion_prompt 至少 45 个字符；frame_prompt 至少 80 个字符，首尾帧 negative_prompt 至少 20 个字符。"
            }
            Self::En => {
                "English output density: video visual_prompt at least 60 English words or 320 characters; motion_prompt at least 30 English words or 180 characters; frame_prompt at least 60 English words or 350 characters; frame negative_prompt at least 8 English words or 60 characters."
            }
        }
    }

    pub fn to_prompt_json(self) -> Value {
        json!({
            "code": self.as_str(),
            "label": self.label(),
            "instruction": self.instruction(),
            "visual_prompt_sections": self.visual_sections(),
            "motion_prompt_sections": self.motion_sections(),
            "frame_prompt_sections": self.frame_sections(),
            "density_rule": self.density_rule()
        })
    }
}

pub fn metadata_matches_prompt_output_language(
    metadata: Option<&Value>,
    expected: PromptOutputLanguage,
) -> bool {
    PromptOutputLanguage::from_metadata(metadata) == expected
}

pub fn with_prompt_output_language_metadata(
    mut metadata: Value,
    language: PromptOutputLanguage,
) -> Value {
    if !metadata.is_object() {
        metadata = json!({});
    }
    if let Some(object) = metadata.as_object_mut() {
        object.insert(
            PROMPT_OUTPUT_LANGUAGE_METADATA_KEY.to_string(),
            Value::String(language.as_str().to_string()),
        );
    }
    metadata
}

pub fn prompt_output_language_from_instruction(
    instruction: &str,
    current: PromptOutputLanguage,
) -> PromptOutputLanguage {
    let normalized = instruction.to_ascii_lowercase();
    if normalized.contains("english")
        || normalized.contains("translate to en")
        || normalized.contains("translate into en")
        || instruction.contains("英文")
        || instruction.contains("英语")
    {
        return PromptOutputLanguage::En;
    }
    if normalized.contains("chinese")
        || normalized.contains(PROMPT_OUTPUT_LANGUAGE_ZH_CN)
        || normalized.contains("simplified chinese")
        || instruction.contains("中文")
        || instruction.contains("简体")
    {
        return PromptOutputLanguage::ZhCn;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn metadata_language_defaults_to_zh_cn_and_preserves_en() {
        assert_eq!(prompt_output_language_code_from_metadata(None), "zh_cn");
        assert_eq!(
            prompt_output_language_code_from_metadata(Some(&json!({
                "prompt_output_language": "en"
            }))),
            "en"
        );
        assert_eq!(
            prompt_output_language_code_from_metadata(Some(&json!({
                "prompt_output_language": "invalid"
            }))),
            "zh_cn"
        );
    }

    #[test]
    fn english_prompt_instruction_preserves_original_dialogue() {
        let instruction = PromptOutputLanguage::En.instruction();

        assert!(instruction.contains("original dialogue"));
        assert!(instruction.contains("do not translate Chinese dialogue"));
    }
}
