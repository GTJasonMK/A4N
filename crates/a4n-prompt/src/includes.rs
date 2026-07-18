//! `<!-- @include path -->` 指令解析器。
//!
//! Include 路径相对 prompts 根目录解析，支持循环/深度检测、路径越界保护，并在
//! 插入被引用 Markdown 时剥离其 frontmatter。

mod frontmatter;
mod resolver;
mod syntax;

pub use resolver::resolve_includes;

#[cfg(test)]
mod tests;
