use super::*;

#[test]
fn declares_complete_project_family_integrity_trigger_set() {
    let function_count = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .filter(|sql| sql.contains("CREATE OR REPLACE FUNCTION "))
        .count();
    let trigger_count = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .filter(|sql| sql.contains("CREATE TRIGGER ") || sql.contains("CREATE CONSTRAINT TRIGGER "))
        .count();

    assert_eq!(
        function_count,
        PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES.len()
    );
    assert_eq!(
        trigger_count,
        PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES.len() * 2 + 6
    );

    for function_name in PROJECT_FAMILY_INTEGRITY_FUNCTION_NAMES {
        let declaration = format!("CREATE OR REPLACE FUNCTION {function_name}()");
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL
                .iter()
                .any(|sql| sql.contains(&declaration)),
            "missing trigger function declaration: {function_name}"
        );
    }

    for trigger_base in PROJECT_FAMILY_TRIGGER_BASE_NAMES {
        for suffix in ["insert", "update"] {
            let trigger_name = format!("CREATE TRIGGER {trigger_base}_{suffix}");
            let constraint_trigger_name =
                format!("CREATE CONSTRAINT TRIGGER {trigger_base}_{suffix}");
            assert!(
                PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
                    sql.contains(&trigger_name) || sql.contains(&constraint_trigger_name)
                }),
                "missing project family trigger: {trigger_base}_{suffix}"
            );
        }
    }
}

#[test]
fn baseline_choice_family_trigger_enforces_group_contract() {
    let line_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_family()"))
        .expect("Galgame line family trigger function");
    let choice_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_choice_family()"))
        .expect("Galgame choice family trigger function");

    for expected in [
        "Galgame choice 行选项数量必须是 2 到 3 个",
        "Galgame 选项 option_index 必须从 1 连续编号",
    ] {
        assert!(
            line_family_sql.contains(expected) && choice_family_sql.contains(expected),
            "missing baseline Galgame choice group contract: {expected}"
        );
    }
    assert!(
        PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
            sql.contains("CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_delete")
                && sql.contains("AFTER DELETE ON galgame_choice")
        }),
        "missing baseline Galgame choice delete trigger"
    );
}

#[test]
fn baseline_scene_family_trigger_enforces_participants_contract() {
    let scene_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_family()"))
        .expect("Galgame scene family trigger function");

    for expected in [
        "participants_json 项必须是对象",
        "participants_json.character_profile_id 必须是正整数",
        "participants_json.character_profile_id 必须属于剧本根项目",
        "jsonb_array_elements(scene_row.participants_json::jsonb)",
        "protagonist_profile profile",
    ] {
        assert!(
            scene_family_sql.contains(expected),
            "missing baseline Galgame scene participants contract: {expected}"
        );
    }
}

#[test]
fn baseline_scene_link_family_trigger_enforces_graph_contract() {
    let scene_link_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_link_family()")
        })
        .expect("Galgame scene link family trigger function");

    for expected in [
        "Galgame 场 next_scene_id 不能指向自身",
        "Galgame 场 next_scene_id 必须属于同一剧本",
        "Galgame branch 场缺少 next_scene_id",
        "Galgame branch 场 next_scene_id 必须指向主线场",
    ] {
        assert!(
            scene_link_sql.contains(expected),
            "missing baseline Galgame scene link contract: {expected}"
        );
    }
    assert!(
        PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
            sql.contains("CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_delete")
                && sql.contains("AFTER DELETE ON galgame_scene")
        }),
        "missing baseline Galgame scene link delete trigger"
    );
}

#[test]
fn baseline_line_order_family_trigger_enforces_group_contract() {
    let line_order_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_order_family()")
        })
        .expect("Galgame line order family trigger function");

    assert!(
        line_order_sql.contains("Galgame 行 line_index 必须在同一场内从 1 连续编号且不重复"),
        "missing baseline Galgame line order group contract"
    );
    for expected in [
        "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_insert",
        "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_update",
        "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_delete",
    ] {
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL
                .iter()
                .any(|sql| sql.contains(expected)),
            "missing baseline Galgame line order trigger: {expected}"
        );
    }
}

#[test]
fn baseline_line_family_trigger_enforces_event_cg_presentation_contract() {
    let line_family_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_family()"))
        .expect("Galgame line family trigger function");

    for expected in [
        "presentation_json.cg 必须是布尔值",
        "presentation_json.cg_importance 必须是字符串",
        "presentation_json.cg_importance 无效",
        "启用事件 CG 时必须提供 cg_note",
    ] {
        assert!(
            line_family_sql.contains(expected),
            "missing baseline Galgame line trigger contract: {expected}"
        );
    }
}

#[test]
fn baseline_generation_spec_family_trigger_enforces_galgame_asset_contract() {
    let generation_spec_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_family()")
        })
        .expect("generation_spec family trigger function");

    for expected in [
        "生成意图 params_json 必须是 JSON 对象",
        "Galgame 生成意图 target_kind/target_entity_kind 配对无效",
        "Galgame 生成意图 target_entity_id 必填",
        "Galgame 背景生成意图 source_snapshot_chapter_id 与生成意图不一致",
        "Galgame CG 生成意图 scene.id 与目标行所属场不一致",
        "Galgame 立绘差分目标角色必须属于源快照",
        "Galgame 立绘差分 source_line_ids 行上下文不一致",
        "Galgame 立绘差分 position_samples 与 source_line_ids 行站位不一致",
        "生成意图已有媒体引用必须属于更新后的同一项目族且指向已完成媒体资产",
        "生成意图已有媒体来源必须匹配更新后的项目族和源快照上下文",
        "生成意图已有提示词媒体来源必须匹配更新后的项目族和源快照上下文",
        "Galgame 生成意图已有媒体来源 request_snapshot_json 必须匹配更新后的目标",
    ] {
        assert!(
            generation_spec_sql.contains(expected),
            "missing baseline Galgame generation_spec trigger contract: {expected}"
        );
    }
    assert!(
        PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
            sql.contains("CREATE TRIGGER trg_generation_spec_family_update")
                && sql.contains("target_kind")
                && sql.contains("target_entity_kind")
                && sql.contains("target_entity_id")
                && sql.contains("params_json ON generation_spec")
        }),
        "baseline generation_spec update trigger must cover Galgame target and params columns"
    );
}

#[test]
fn baseline_generation_spec_text_anchor_trigger_enforces_chapter_scope() {
    let text_anchor_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains(
                "CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_text_anchor_family()",
            )
        })
        .expect("generation_spec text_anchor family trigger function");

    for expected in [
        "生成意图文本锚点必须属于同一源快照章节",
        "生成意图文本锚点 source_chapter_id 必须与生成意图一致",
    ] {
        assert!(
            text_anchor_sql.contains(expected),
            "missing baseline generation_spec text_anchor chapter contract: {expected}"
        );
    }
}

#[test]
fn baseline_generation_spec_entity_ref_trigger_enforces_snapshot_profile_scope() {
    let entity_ref_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains(
                "CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_entity_ref_family()",
            )
        })
        .expect("generation_spec entity_ref family trigger function");

    for expected in [
        "生成意图实体引用 entity_kind 无效",
        "生成意图实体引用 role 无效",
        "事件档案尚未建立持久模型，不能作为生成意图实体引用",
        "生成意图实体引用必须属于生成意图源快照",
    ] {
        assert!(
            entity_ref_sql.contains(expected),
            "missing baseline generation_spec entity_ref contract: {expected}"
        );
    }
    for expected in [
        "CREATE TRIGGER trg_generation_spec_entity_ref_family_insert",
        "CREATE TRIGGER trg_generation_spec_entity_ref_family_update",
    ] {
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL
                .iter()
                .any(|sql| sql.contains(expected)),
            "missing baseline generation_spec entity_ref trigger: {expected}"
        );
    }
}

#[test]
fn baseline_media_asset_trigger_enforces_paired_chapter_context() {
    let media_asset_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_media_asset_family()"))
        .expect("media asset family trigger function");

    for expected in [
        "source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空",
        "chapter.source_chapter_id = NEW.source_chapter_id",
        "completed 媒体资产必须有 file_path",
        "媒体资产更新会破坏已有生成意图媒体引用",
        "媒体资产更新会破坏已有 Galgame 场媒体绑定",
        "媒体资产更新会破坏已有 Galgame 行媒体绑定",
        "媒体资产更新会破坏已有 generation_spec 媒体来源",
        "媒体资产更新会破坏已有 prompt_artifact 媒体来源",
        "媒体资产更新会破坏已有 source_media_asset 媒体来源",
        "媒体资产更新会破坏引用它的媒体来源",
    ] {
        assert!(
            media_asset_sql.contains(expected),
            "missing baseline media asset trigger contract: {expected}"
        );
    }
}

#[test]
fn baseline_generation_spec_media_ref_trigger_enforces_role_and_asset_readiness() {
    let media_ref_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains(
                "CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_media_ref_family()",
            )
        })
        .expect("generation spec media ref family trigger function");

    for expected in [
        "生成意图媒体引用 role 无效",
        "'character_reference'",
        "'style_reference'",
        "asset.status = 'completed'",
        "asset.file_path IS NOT NULL",
        "生成意图媒体引用必须使用同项目族已完成媒体资产",
    ] {
        assert!(
            media_ref_sql.contains(expected),
            "missing baseline generation_spec media_ref trigger contract: {expected}"
        );
    }
    assert!(
        PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
            sql.contains("CREATE TRIGGER trg_generation_spec_media_ref_family_update")
                && sql.contains(
                    "generation_spec_id, media_asset_id, role ON generation_spec_media_ref",
                )
        }),
        "baseline generation_spec_media_ref update trigger must cover role"
    );
}

#[test]
fn baseline_media_provenance_trigger_enforces_galgame_request_snapshot_contract() {
    let media_provenance_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_media_provenance_family()")
        })
        .expect("media provenance family trigger function");

    for expected in [
        "request_snapshot_json 必须是 JSON 对象",
        "request_snapshot_json.source 必须是 galgame_asset_generation",
        "request_snapshot_json.target_entity_kind 缺失",
        "request_snapshot_json.target_entity_id 缺失",
        "request_snapshot_json 与 generation_spec 目标不一致",
        "request_snapshot_json.target_line_id 缺失",
        "request_snapshot_json.target_line_id 必须是整数",
        "request_snapshot_json.target_line_id 必须指向同项目 Galgame 行",
        "source.status = 'completed'",
        "source.file_path IS NOT NULL",
    ] {
        assert!(
            media_provenance_sql.contains(expected),
            "missing baseline media provenance trigger contract: {expected}"
        );
    }
    assert!(
        PROJECT_FAMILY_INTEGRITY_SQL.iter().any(|sql| {
            sql.contains("CREATE TRIGGER trg_media_provenance_family_update")
                && sql.contains("request_snapshot_json ON media_provenance")
        }),
        "baseline media provenance update trigger must cover request_snapshot_json"
    );
}

#[test]
fn baseline_galgame_media_binding_trigger_requires_ready_media_asset() {
    let media_binding_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| sql.contains("CREATE OR REPLACE FUNCTION a4n_validate_galgame_media_binding()"))
        .expect("Galgame media binding trigger function");

    for expected in ["asset.status = 'completed'", "asset.file_path IS NOT NULL"] {
        assert!(
            media_binding_sql.contains(expected),
            "missing baseline Galgame media binding asset readiness contract: {expected}"
        );
    }
}

#[test]
fn baseline_galgame_target_reverse_integrity_contract_is_installed() {
    let reverse_sql = PROJECT_FAMILY_INTEGRITY_SQL
        .iter()
        .find(|sql| {
            sql.contains(
                "CREATE OR REPLACE FUNCTION a4n_validate_galgame_target_reverse_integrity()",
            )
        })
        .expect("Galgame target reverse integrity trigger function");

    for expected in [
        "Galgame 背景生成意图必须随目标场更新或清理",
        "Galgame CG 生成意图必须随目标行更新或清理",
        "Galgame 立绘差分生成意图必须随来源行更新或清理",
        "Galgame 场媒体绑定必须随目标场更新或清理",
        "Galgame 行媒体绑定必须随目标行更新或清理",
        "Galgame 音频媒体来源 target_line_id 必须随目标行更新或清理",
    ] {
        assert!(
            reverse_sql.contains(expected),
            "missing baseline Galgame reverse integrity contract: {expected}"
        );
    }

    for expected_trigger in [
        "trg_galgame_target_reverse_script_delete",
        "trg_galgame_target_reverse_scene_update",
        "trg_galgame_target_reverse_scene_delete",
        "trg_galgame_target_reverse_line_update",
        "trg_galgame_target_reverse_line_delete",
    ] {
        assert!(
            PROJECT_FAMILY_INTEGRITY_SQL
                .iter()
                .any(|sql| sql.contains(expected_trigger)
                    && sql.contains("DEFERRABLE INITIALLY DEFERRED")),
            "missing baseline Galgame reverse integrity trigger: {expected_trigger}"
        );
    }
}

#[test]
fn declares_text_source_sync_triggers_for_snapshot_inputs() {
    let function_count = TEXT_SOURCE_SYNC_SQL
        .iter()
        .filter(|sql| sql.contains("CREATE OR REPLACE FUNCTION "))
        .count();
    let trigger_count = TEXT_SOURCE_SYNC_SQL
        .iter()
        .filter(|sql| sql.contains("CREATE TRIGGER "))
        .count();

    assert_eq!(function_count, TEXT_SOURCE_SYNC_FUNCTION_NAMES.len());
    assert_eq!(trigger_count, TEXT_SOURCE_SYNC_TRIGGER_BASE_NAMES.len() * 3);

    for function_name in TEXT_SOURCE_SYNC_FUNCTION_NAMES {
        let declaration = format!("CREATE OR REPLACE FUNCTION {function_name}()");
        assert!(
            TEXT_SOURCE_SYNC_SQL
                .iter()
                .any(|sql| sql.contains(&declaration)),
            "missing text source sync function declaration: {function_name}"
        );
    }

    for trigger_base in TEXT_SOURCE_SYNC_TRIGGER_BASE_NAMES {
        for suffix in ["insert", "update", "delete"] {
            let trigger_name = format!("CREATE TRIGGER {trigger_base}_{suffix}");
            assert!(
                TEXT_SOURCE_SYNC_SQL
                    .iter()
                    .any(|sql| sql.contains(&trigger_name)),
                "missing text source sync trigger: {trigger_base}_{suffix}"
            );
        }
    }
}
