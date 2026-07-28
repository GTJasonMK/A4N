use super::*;

#[test]
fn validate_package_manifest_rejects_unknown_project_type() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].project_type = "novel_medium".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject unknown project_type");

    assert!(err.contains("项目类型无效"));
    assert!(err.contains("novel_medium"));
}

#[test]
fn validate_package_manifest_rejects_unknown_project_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].status = "publishing".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject statuses outside the project state machine");

    assert!(err.contains("未知项目状态"));
    assert!(err.contains("publishing"));
}

#[test]
fn validate_package_manifest_rejects_unknown_project_source_sync_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].source_sync_status = "stale".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("project family import must reject unknown source sync states");

    assert!(err.contains("项目源同步状态无效"));
    assert!(err.contains("project_id=1"));
    assert!(err.contains("source_sync_status=stale"));
}

#[test]
fn validate_package_manifest_rejects_root_project_non_current_source_sync_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].source_sync_status = SOURCE_SYNC_ROOT_UPDATED.to_string();

    let err = validate_package_manifest(&package)
        .expect_err("text root projects must not carry derived source sync states");

    assert!(err.contains("文本项目源同步状态必须是 current"));
    assert!(err.contains("project_id=1"));
    assert!(err.contains("source_sync_status=root_updated"));
}

#[test]
fn validate_package_manifest_rejects_root_project_derived_status() {
    let mut package = package_with_part_outlines(Vec::new());
    package.projects[0].derived_project_status = Some(DERIVED_PROJECT_STATUS_DRAFT.to_string());

    let err = validate_package_manifest(&package)
        .expect_err("text root projects must not carry derived-only status");

    assert!(err.contains("文本项目不能包含派生项目状态"));
    assert!(err.contains("project_id=1"));
}

#[test]
fn validate_package_manifest_rejects_derived_project_missing_derived_status() {
    let mut package = package_with_derived_project();
    package.projects[1].derived_project_status = None;

    let err = validate_package_manifest(&package)
        .expect_err("derived projects must carry derived_project_status");

    assert!(err.contains("派生项目缺少 derived_project_status"));
    assert!(err.contains("project_id=2"));
}

#[test]
fn validate_package_manifest_rejects_unknown_derived_project_status() {
    let mut package = package_with_derived_project();
    package.projects[1].derived_project_status = Some("ready".to_string());

    let err = validate_package_manifest(&package)
        .expect_err("derived project status must use known values");

    assert!(err.contains("派生项目状态无效"));
    assert!(err.contains("project_id=2"));
    assert!(err.contains("derived_project_status=ready"));
}

#[test]
fn validate_package_manifest_accepts_known_derived_project_source_sync_statuses() {
    for status in [
        SOURCE_SYNC_CURRENT,
        SOURCE_SYNC_ROOT_UPDATED,
        SOURCE_SYNC_CONFLICT,
        SOURCE_SYNC_DETACHED,
    ] {
        let mut package = package_with_derived_project();
        package.projects[1].source_sync_status = status.to_string();

        validate_package_manifest(&package)
            .expect("known derived project source sync status should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_invalid_project_derivation_scope() {
    let mut package = package_with_derived_project();
    package.project_derivations[0].chapter_scope_json = json!({"start": 0, "end": 10});

    let err = validate_package_manifest(&package)
        .expect_err("derivation chapter scope must match source sync parser constraints");

    assert!(err.contains("派生关系 chapter_scope_json 范围无效"));
    assert!(err.contains("start=0"));
}

#[test]
fn validate_package_manifest_rejects_non_object_project_derivation_options() {
    let mut package = package_with_derived_project();
    package.project_derivations[0].creation_options_json = json!(["not", "an", "object"]);

    let err = validate_package_manifest(&package)
        .expect_err("creation_options_json must keep service object invariant");

    assert!(err.contains("creation_options_json 必须是 JSON 对象"));
    assert!(err.contains("project_derivation_id=911"));
}

#[test]
fn validate_package_manifest_rejects_derivation_scope_mismatched_snapshot_scope() {
    let mut package = package_with_derived_project();
    package.project_derivations[0].chapter_scope_json = json!({"start": 1, "end": 5});

    let err = validate_package_manifest(&package)
        .expect_err("derivation scope and snapshot scope describe the same frozen source");

    assert!(err.contains("派生关系章节范围与源文本快照不一致"));
    assert!(err.contains("project_derivation_id=911"));
    assert!(err.contains("source_snapshot_id=901"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_wrong_root() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].root_project_id = 99;

    let err = validate_package_manifest(&package)
        .expect_err("source snapshots must belong to the package root project");

    assert!(err.contains("源文本快照不属于根项目"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("root_project_id=99"));
}

#[test]
fn validate_package_manifest_rejects_invalid_source_snapshot_scope() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].chapter_scope_json = json!({"start": 10, "end": 1});

    let err = validate_package_manifest(&package)
        .expect_err("source snapshot chapter scope must match service parser constraints");

    assert!(err.contains("源文本快照 chapter_scope_json 范围无效"));
    assert!(err.contains("start=10"));
    assert!(err.contains("end=1"));
}

#[test]
fn validate_package_manifest_rejects_non_array_source_snapshot_hashes() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].profile_hashes_json = json!({});

    let err = validate_package_manifest(&package)
        .expect_err("source snapshot profile hashes must use service array shape");

    assert!(err.contains("profile_hashes_json 必须是数组"));
    assert!(err.contains("source_snapshot_id=901"));
}

#[test]
fn validate_package_manifest_accepts_consistent_source_snapshot_details() {
    let mut package = package_with_derived_project();
    let outline = source_snapshot_outline_record("part");
    let profile = source_snapshot_profile_record("character");
    package.source_snapshot_outlines = vec![outline];
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&profile)]);
    package.source_snapshot_profiles = vec![profile];

    validate_package_manifest(&package)
        .expect("service-shaped source snapshot details must remain importable");
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_missing_detail() {
    let mut package = package_with_derived_project();
    package
        .source_snapshot_chapters
        .retain(|chapter| chapter.chapter_number != 1);

    let err = validate_package_manifest(&package)
        .expect_err("snapshot scope must have one frozen chapter detail per chapter");

    assert!(err.contains("源文本快照缺少章节明细"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("chapter_number=1"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_out_of_scope() {
    let mut package = package_with_derived_project();
    package.source_snapshot_chapters[0].chapter_number = 11;

    let err = validate_package_manifest(&package)
        .expect_err("snapshot chapter details must stay inside the frozen scope");

    assert!(err.contains("源快照章节不在快照范围内"));
    assert!(err.contains("source_snapshot_chapter_id=1001"));
    assert!(err.contains("chapter_number=11"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_content_hash_mismatch() {
    let mut package = package_with_derived_project();
    package.source_snapshot_chapters[0].content = "篡改后的冻结正文".to_string();

    let err = validate_package_manifest(&package)
        .expect_err("snapshot chapter content_hash must describe frozen content");

    assert!(err.contains("content_hash 与正文不一致"));
    assert!(err.contains("source_snapshot_chapter_id=1001"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_chapter_hash_row_mismatch() {
    let mut package = package_with_derived_project();
    package.source_snapshots[0].chapter_hashes_json[0]["content_hash"] = json!("wrong");

    let err = validate_package_manifest(&package)
        .expect_err("snapshot chapter_hashes_json must match chapter detail rows");

    assert!(err.contains("chapter_hashes_json 与章节明细 content_hash 不一致"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("chapter_number=1"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_outline_unknown_kind() {
    let mut package = package_with_derived_project();
    package.source_snapshot_outlines = vec![source_snapshot_outline_record("timeline")];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot outlines can only freeze service-created outline kinds");

    assert!(err.contains("源快照大纲类型无效"));
    assert!(err.contains("source_snapshot_outline_id=1201"));
    assert!(err.contains("outline_kind=timeline"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_outline_payload_hash_mismatch() {
    let mut package = package_with_derived_project();
    let mut outline = source_snapshot_outline_record("chapter");
    outline.payload_json["title"] = json!("篡改后的大纲");
    package.source_snapshot_outlines = vec![outline];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot outline payload_hash must describe payload_json");

    assert!(err.contains("源快照大纲 payload_hash 与 payload_json 不一致"));
    assert!(err.contains("source_snapshot_outline_id=1201"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_outline_payload_id_mismatch() {
    let mut package = package_with_derived_project();
    let mut outline = source_snapshot_outline_record("part");
    outline.payload_json["id"] = json!(999);
    outline.payload_hash = hash_json(&outline.payload_json);
    package.source_snapshot_outlines = vec![outline];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot outline payload identity must match the detail row");

    assert!(err.contains("payload_json.id 与 source_outline_id 不一致"));
    assert!(err.contains("source_snapshot_outline_id=1201"));
    assert!(err.contains("payload_id=999"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_profile_unknown_kind() {
    let mut package = package_with_derived_project();
    package.source_snapshot_profiles = vec![source_snapshot_profile_record("event")];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot profiles only freeze persisted profile kinds");

    assert!(err.contains("源快照档案类型无效"));
    assert!(err.contains("source_snapshot_profile_id=1301"));
    assert!(err.contains("profile_kind=event"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_profile_hash_row_mismatch() {
    let mut package = package_with_derived_project();
    let profile = source_snapshot_profile_record("character");
    package.source_snapshot_profiles = vec![profile];
    package.source_snapshots[0].profile_hashes_json =
        json!([{"kind": "character", "id": 401, "hash": "wrong"}]);

    let err = validate_package_manifest(&package)
        .expect_err("snapshot profile_hashes_json must match profile detail rows");

    assert!(err.contains("profile_hashes_json 与档案明细 hash 不一致"));
    assert!(err.contains("source_snapshot_id=901"));
    assert!(err.contains("kind=character"));
    assert!(err.contains("id=401"));
}

#[test]
fn validate_package_manifest_rejects_source_snapshot_profile_payload_project_mismatch() {
    let mut package = package_with_derived_project();
    let mut profile = source_snapshot_profile_record("character");
    profile.payload_json["project_id"] = json!(99);
    profile.payload_hash = hash_json(&profile.payload_json);
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&profile)]);
    package.source_snapshot_profiles = vec![profile];

    let err = validate_package_manifest(&package)
        .expect_err("snapshot profile payload project_id must match the frozen root");

    assert!(err.contains("payload_json.project_id 与 root_project_id 不一致"));
    assert!(err.contains("source_snapshot_profile_id=1301"));
    assert!(err.contains("payload_project_id=99"));
}

#[test]
fn validate_package_manifest_accepts_consistent_text_anchor_and_profile_evidence() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let anchor = chapter_text_anchor_record(&chapter);
    package.text_core.chapters = vec![chapter];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_anchors = vec![anchor];
    package.profile_evidence = vec![profile_evidence_record()];

    validate_package_manifest(&package)
        .expect("service-shaped text anchor and profile evidence should remain importable");
}

#[test]
fn validate_package_manifest_rejects_text_anchor_quote_hash_mismatch() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let mut anchor = chapter_text_anchor_record(&chapter);
    anchor.quote_hash = "wrong".to_string();
    package.text_core.chapters = vec![chapter];
    package.text_anchors = vec![anchor];

    let err = validate_package_manifest(&package)
        .expect_err("text anchor quote_hash must describe the quote");

    assert!(err.contains("文本锚点 quote_hash 与 quote 不一致"));
    assert!(err.contains("text_anchor_id=701"));
}

#[test]
fn validate_package_manifest_rejects_text_anchor_source_version_mismatch() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let mut anchor = chapter_text_anchor_record(&chapter);
    anchor.source_version_hash = "stale".to_string();
    package.text_core.chapters = vec![chapter];
    package.text_anchors = vec![anchor];

    let err = validate_package_manifest(&package)
        .expect_err("chapter content anchor source_version_hash must match chapter content");

    assert!(err.contains("文本锚点 source_version_hash 与源内容不一致"));
    assert!(err.contains("text_anchor_id=701"));
}

#[test]
fn validate_package_manifest_rejects_snapshot_text_anchor_source_mismatch() {
    let mut package = package_with_derived_project();
    let mut anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[0]);
    anchor.source_id = package.source_snapshot_chapters[1].id;
    package.text_anchors = vec![anchor];

    let err = validate_package_manifest(&package).expect_err(
        "snapshot content anchor source_id and snapshot chapter id are the same identity",
    );

    assert!(err.contains("source_id 与 source_snapshot_chapter_id 不一致"));
    assert!(err.contains("text_anchor_id=702"));
}

#[test]
fn validate_package_manifest_rejects_unknown_profile_evidence_role() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let anchor = chapter_text_anchor_record(&chapter);
    let mut evidence = profile_evidence_record();
    evidence.evidence_role = "origin_story".to_string();
    package.text_core.chapters = vec![chapter];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_anchors = vec![anchor];
    package.profile_evidence = vec![evidence];

    let err = validate_package_manifest(&package)
        .expect_err("profile evidence role must use the canonical role set");

    assert!(err.contains("档案证据 evidence_role 无效"));
    assert!(err.contains("origin_story"));
}

#[test]
fn validate_package_manifest_rejects_duplicate_profile_evidence_key() {
    let mut package = package_with_part_outlines(Vec::new());
    let chapter = chapter_record(201, 1, None);
    let anchor = chapter_text_anchor_record(&chapter);
    let first = profile_evidence_record();
    let mut duplicate = profile_evidence_record();
    duplicate.id = 802;
    duplicate.evidence_role = format!(" {} ", EVIDENCE_ROLE_FIRST_SEEN);
    package.text_core.chapters = vec![chapter];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_anchors = vec![anchor];
    package.profile_evidence = vec![first, duplicate];

    let err = validate_package_manifest(&package)
        .expect_err("profile evidence uniqueness uses normalized evidence_role");

    assert!(err.contains("档案证据重复"));
    assert!(err.contains("profile_id=601"));
    assert!(err.contains("text_anchor_id=701"));
    assert!(err.contains(EVIDENCE_ROLE_FIRST_SEEN));
}
