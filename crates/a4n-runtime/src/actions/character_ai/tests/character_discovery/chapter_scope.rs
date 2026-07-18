#[test]
fn select_character_discovery_chapters_uses_available_content_only() {
    let chapters = vec![
        discovery_chapter(1, "completed", "有效正文"),
        discovery_chapter(3, "completed", "另一章正文"),
    ];

    let selected = select_character_discovery_chapters(chapters, 1, Some(3))
        .expect("selected available chapter should pass");

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].chapter_number, 3);
}

#[test]
fn select_character_discovery_chapters_rejects_missing_available_chapter() {
    let chapters = vec![discovery_chapter(1, "completed", "有效正文")];

    let err = select_character_discovery_chapters(chapters, 1, Some(2))
        .expect_err("missing available chapter should fail");

    assert!(err.contains("没有可用于角色发现的有效正文"));
}

#[test]
fn select_character_discovery_chapters_rejects_empty_project_scope() {
    let err = select_character_discovery_chapters(Vec::new(), 9, None)
        .expect_err("empty available chapter list should fail");

    assert!(err.contains("项目还没有可用于角色发现的章节"));
    assert!(err.contains("project_id=9"));
}

#[test]
fn validate_discovered_character_chapter_scope_accepts_analyzed_chapters() {
    let characters = vec![json!({
        "name": "沈砚",
        "first_seen_chapter": 1,
        "latest_seen_chapter": 3
    })];
    let analyzed = [1, 3].into_iter().collect::<BTreeSet<_>>();

    validate_discovered_character_chapter_scope(&characters, &analyzed)
        .expect("first/latest seen chapters from input should pass");
}

#[test]
fn validate_discovered_character_chapter_scope_rejects_out_of_input_chapter() {
    let characters = vec![json!({
        "name": "沈砚",
        "first_seen_chapter": 1,
        "latest_seen_chapter": 2
    })];
    let analyzed = [1, 3].into_iter().collect::<BTreeSet<_>>();

    let err = validate_discovered_character_chapter_scope(&characters, &analyzed)
        .expect_err("out-of-input discovery chapter should fail");

    assert!(err.contains("latest_seen_chapter"));
    assert!(err.contains("不在本次角色发现输入章节中"));
}
