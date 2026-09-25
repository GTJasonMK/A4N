use super::*;

#[test]
fn validate_package_manifest_rejects_overlapping_part_ranges() {
    let package = package_with_part_outlines(vec![
        part_outline(101, 1, 1, 10),
        part_outline(102, 2, 10, 20),
    ]);

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject overlapping part ranges");

    assert!(err.contains("分部章节范围重叠"));
    assert!(err.contains("part_outline_id=101"));
    assert!(err.contains("第1部(1-10)"));
    assert!(err.contains("part_outline_id=102"));
    assert!(err.contains("第2部(10-20)"));
}

#[test]
fn validate_package_manifest_rejects_duplicate_text_core_row_ids() {
    let package = package_with_part_outlines(vec![
        part_outline(101, 1, 1, 10),
        part_outline(101, 2, 11, 20),
    ]);

    let err = validate_package_manifest(&package)
        .expect_err("duplicate old ids would overwrite import id mappings");

    assert!(err.contains("text_core.part_outlines.id"));
    assert!(err.contains("101"));
}

#[test]
fn validate_package_manifest_accepts_adjacent_part_ranges() {
    let package = package_with_part_outlines(vec![
        part_outline(101, 1, 1, 10),
        part_outline(102, 2, 11, 20),
    ]);

    validate_package_manifest(&package).expect("adjacent part ranges should remain importable");
}

#[test]
fn validate_package_manifest_rejects_chapter_outline_reference_for_other_chapter() {
    let mut package = package_with_part_outlines(Vec::new());
    package.text_core.chapter_outlines = vec![chapter_outline(102, 2)];
    package.text_core.chapters = vec![chapter_record(201, 1, Some(102))];

    let err = validate_package_manifest(&package)
        .expect_err("chapter records must not point at another chapter outline");

    assert!(err.contains("章节大纲属于第2章"));
    assert!(err.contains("chapter_id=201"));
    assert!(err.contains("chapter_number=1"));
    assert!(err.contains("outline_id=102"));
}

#[test]
fn validate_package_manifest_accepts_same_chapter_outline_reference() {
    let mut package = package_with_part_outlines(Vec::new());
    package.text_core.chapter_outlines = vec![chapter_outline(101, 1)];
    package.text_core.chapters = vec![chapter_record(201, 1, Some(101))];

    validate_package_manifest(&package).expect("same-chapter outline reference should import");
}

#[test]
fn validate_package_manifest_rejects_chapter_outline_part_outside_range() {
    let mut package = package_with_part_outlines(vec![part_outline(101, 1, 1, 10)]);
    package.text_core.chapter_outlines = vec![chapter_outline_with_part(201, 11, 101)];

    let err = validate_package_manifest(&package)
        .expect_err("chapter outline part_id must contain the chapter number");

    assert!(err.contains("章节大纲引用的第1部分部范围不包含该章节"));
    assert!(err.contains("chapter_outline_id=201"));
    assert!(err.contains("chapter_number=11"));
    assert!(err.contains("part_id=101"));
    assert!(err.contains("range=1-10"));
}

#[test]
fn validate_package_manifest_accepts_chapter_outline_part_inside_range() {
    let mut package = package_with_part_outlines(vec![part_outline(101, 1, 1, 10)]);
    package.text_core.chapter_outlines = vec![chapter_outline_with_part(201, 10, 101)];

    validate_package_manifest(&package)
        .expect("chapter outline may reference a part that contains the chapter");
}

#[test]
fn validate_package_manifest_rejects_unknown_chapter_outline_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut outline = chapter_outline(201, 1);
    outline.status = "bogus".to_string();
    package.text_core.chapter_outlines = vec![outline];

    let err = validate_package_manifest(&package)
        .expect_err("chapter outline status must use known states");

    assert!(err.contains("章节大纲状态无效"));
    assert!(err.contains("chapter_outline_id=201"));
    assert!(err.contains("status=bogus"));
}

#[test]
fn validate_package_manifest_accepts_known_chapter_outline_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut outline = chapter_outline(201, 1);
    outline.status = chapter_outline::STATUS_COMPLETED.to_string();
    package.text_core.chapter_outlines = vec![outline];

    validate_package_manifest(&package)
        .expect("completed chapter outline status should remain importable");
}
