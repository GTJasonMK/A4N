use super::{line_end_at_or_after, line_start_at_or_before};

#[test]
fn line_bounds_cover_first_middle_and_last_lines() {
    let text = "第1章 开始\n正文\n第2章 结束";

    assert_eq!(line_start_at_or_before(text, 0), 0);
    assert_eq!(line_end_at_or_after(text, 0), "第1章 开始".len());

    let middle = text.find("正文").expect("middle line exists");
    assert_eq!(line_start_at_or_before(text, middle), "第1章 开始\n".len());
    assert_eq!(line_end_at_or_after(text, middle), "第1章 开始\n正文".len());

    let last = text.find("第2章").expect("last line exists");
    assert_eq!(
        line_start_at_or_before(text, last),
        "第1章 开始\n正文\n".len()
    );
    assert_eq!(line_end_at_or_after(text, last), text.len());
}
