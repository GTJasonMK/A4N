//! EPUB3 电子书导出
//!
//! 生成符合 EPUB3 规范的电子书容器。

mod builder;
mod constants;
mod cover;
mod model;
mod package {
    mod ncx;
    mod opf;
}
#[cfg(test)]
mod test_support;
mod xhtml;

pub use builder::EpubBuilder;
pub use constants::builtin_epub_stylesheet;
pub use model::{EpubChapter, EpubMetadata};
