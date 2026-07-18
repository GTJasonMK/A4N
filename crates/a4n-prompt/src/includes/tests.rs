use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;

use super::frontmatter::strip_frontmatter;
use super::resolve_includes;

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

#[test]
fn strip_frontmatter_basic() {
    let source = "---\nname: x\n---\nhello";
    assert_eq!(strip_frontmatter(source), "hello");
}

#[test]
fn strip_frontmatter_no_frontmatter() {
    assert_eq!(strip_frontmatter("hello"), "hello");
}

#[test]
fn basic_include() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let main = root.join("main.md");
    let partial = root.join("_partials").join("p.md");
    write(&partial, "PARTIAL_BODY");
    write(&main, "BEFORE\n<!-- @include _partials/p.md -->\nAFTER");
    let body = fs::read_to_string(&main).unwrap();
    let output = resolve_includes(&body, &main, root).unwrap();
    assert!(output.contains("BEFORE"));
    assert!(output.contains("PARTIAL_BODY"));
    assert!(output.contains("AFTER"));
}

#[test]
fn include_strips_inner_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let main = root.join("main.md");
    let partial = root.join("_partials").join("p.md");
    write(&partial, "---\nname: p\n---\nONLY_BODY");
    write(&main, "<!-- @include _partials/p.md -->");
    let output = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, root).unwrap();
    assert!(output.contains("ONLY_BODY"));
    assert!(!output.contains("name: p"));
}

#[test]
fn cycle_detected() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let first = root.join("a.md");
    let second = root.join("b.md");
    write(&first, "<!-- @include b.md -->");
    write(&second, "<!-- @include a.md -->");
    let error = resolve_includes(&fs::read_to_string(&first).unwrap(), &first, root).unwrap_err();
    assert!(format!("{error}").contains("循环引用"));
}

#[test]
fn path_outside_root_rejected() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("inner");
    fs::create_dir_all(&root).unwrap();
    let outside = tmp.path().join("outside.md");
    write(&outside, "OUTSIDE");
    let main = root.join("main.md");
    write(&main, "<!-- @include ../outside.md -->");
    let result = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, &root);
    assert!(result.is_err());
}

#[test]
fn missing_file_errors() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let main = root.join("main.md");
    write(&main, "<!-- @include nope.md -->");
    let result = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, root);
    assert!(result.is_err());
}

#[test]
fn current_file_must_exist() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let missing = root.join("missing.md");
    let error = resolve_includes("body", &missing, root).unwrap_err();
    assert!(format!("{error}").contains("current_file"));
}

#[test]
fn quoted_path_works() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let main = root.join("main.md");
    let partial = root.join("_partials").join("q.md");
    write(&partial, "QBODY");
    write(&main, r#"<!-- @include "_partials/q.md" -->"#);
    let output = resolve_includes(&fs::read_to_string(&main).unwrap(), &main, root).unwrap();
    assert!(output.contains("QBODY"));
}
