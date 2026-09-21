use super::*;

#[test]
fn reverse_integrity_rejects_stale_generation_targets() {
    for expected in [
        "Galgame 背景生成意图必须随目标场更新或清理",
        "Galgame CG 生成意图必须随目标行更新或清理",
        "Galgame 立绘差分生成意图必须随来源行更新或清理",
        "source_line_ids",
        "position_samples",
    ] {
        assert!(
            GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL.contains(expected),
            "missing Galgame reverse generation target contract: {expected}"
        );
    }
}

#[test]
fn reverse_integrity_rejects_stale_media_targets() {
    for expected in [
        "Galgame 场媒体绑定必须随目标场更新或清理",
        "Galgame 行媒体绑定必须随目标行更新或清理",
        "Galgame 音频媒体来源 target_line_id 必须随目标行更新或清理",
        "CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_line_delete",
        "DEFERRABLE INITIALLY DEFERRED",
    ] {
        assert!(
            GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL.contains(expected)
                || GALGAME_TARGET_REVERSE_INTEGRITY_UP_SQL
                    .iter()
                    .any(|sql| sql.contains(expected)),
            "missing Galgame reverse media target contract: {expected}"
        );
    }
}
