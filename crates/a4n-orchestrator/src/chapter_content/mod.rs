//! Chapter content 公共纯规则。
//!
//! 数据库行为与后处理编排由 `context/chapter_content.rs` 的私有
//! `ChapterContentApplication` 持有。DB 输入 DTO 不再经本模块穿透导出。

mod application;
mod sync;
