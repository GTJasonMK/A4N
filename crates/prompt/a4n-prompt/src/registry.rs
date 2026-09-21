//! 提示词注册表（registry）。
//!
//! `prompts/_registry.yaml` 是提示词元数据的唯一来源；模板 frontmatter 只声明模板本地
//! 字段，展示、分类、调用方和依赖等跨文件元数据均由注册表统一承载。

mod contracts;
mod loading;
mod state;
mod validation;

pub(crate) use contracts::{CategoryDef, PromptRegistryEntry, RegistryFile};
pub(crate) use state::PromptRegistry;

#[cfg(test)]
mod tests;
