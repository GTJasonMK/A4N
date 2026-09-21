use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const IMAGE_READ_MAX_IMAGES: usize = 4;
pub const IMAGE_READ_MAX_INSTRUCTION_CHARS: usize = 4_000;

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadImageInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 4))]
    pub image_ids: Vec<i64>,
    #[serde(default)]
    pub chapter_id: Option<i64>,
    #[serde(default)]
    pub instruction: Option<String>,
}

impl ReadImageInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.project_id <= 0 {
            return Err("read_image projectId 必须为正整数".into());
        }
        if self.image_ids.is_empty() || self.image_ids.len() > IMAGE_READ_MAX_IMAGES {
            return Err(format!(
                "read_image imageIds 数量必须在 1-{} 之间",
                IMAGE_READ_MAX_IMAGES
            ));
        }
        if self.image_ids.iter().any(|id| *id <= 0) {
            return Err("read_image imageIds 必须全部为正整数".into());
        }
        if let Some(chapter_id) = self.chapter_id
            && chapter_id <= 0
        {
            return Err("read_image chapterId 必须为正整数".into());
        }
        if self
            .instruction
            .as_deref()
            .is_some_and(|value| value.chars().count() > IMAGE_READ_MAX_INSTRUCTION_CHARS)
        {
            return Err(format!(
                "read_image instruction 不能超过 {} 个字符",
                IMAGE_READ_MAX_INSTRUCTION_CHARS
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct ReadImageResult {
    pub project_id: i64,
    pub image_ids: Vec<i64>,
    pub analysis: String,
}

#[cfg(test)]
mod tests {
    use super::ReadImageInput;

    #[test]
    fn read_image_input_validates_scope_and_batch_limits() {
        let input: ReadImageInput = serde_json::from_value(serde_json::json!({
            "projectId": 7,
            "imageIds": [11, 12],
            "chapterId": 3,
            "instruction": "比较构图和动作连续性"
        }))
        .unwrap();
        input.validate().unwrap();

        let mut invalid = input;
        invalid.image_ids = vec![0];
        assert!(invalid.validate().is_err());
    }
}
