use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::generation_tasks::GenerationStartRequestMeta;

/// 人物图片生成的唯一启动合同。
///
/// 查询、上传和请求预览仍使用各自的旧合同；只有四个生产生成入口共用这里的
/// 一个 start/worker 路径。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartPortraitGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub operation: PortraitGenerationOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PortraitGenerationOperation {
    Generate {
        #[schemars(length(min = 1, max = 512))]
        character_name: String,
        #[schemars(length(max = 512))]
        style: Option<String>,
        #[schemars(length(max = 32))]
        expressions: Option<Vec<String>>,
        #[schemars(length(max = 512))]
        pose: Option<String>,
        #[schemars(length(max = 4))]
        #[serde(default)]
        reference_image_ids: Vec<i64>,
    },
    Regenerate {
        #[schemars(range(min = 1))]
        portrait_id: i64,
        #[schemars(length(max = 4))]
        #[serde(default)]
        reference_image_ids: Vec<i64>,
    },
    GenerateAgeVariant {
        #[schemars(range(min = 1))]
        portrait_id: i64,
        #[schemars(length(min = 1, max = 128))]
        age: String,
        #[schemars(length(max = 4))]
        #[serde(default)]
        reference_image_ids: Vec<i64>,
    },
    GenerateMissing,
}
