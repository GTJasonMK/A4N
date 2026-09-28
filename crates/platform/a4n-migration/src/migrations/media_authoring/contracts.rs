use sea_orm::DbErr;

/// Keep the deployed cast, choice and per-scene source constraints. Only the
/// versioned presentation and local adaptation identity change here. Constraint
/// triggers are deferred, so old rows are converted before this is evaluated.
pub(super) fn line_contract() -> Result<String, DbErr> {
    let mut sql = crate::migrations::galgame_scene_snapshot_scope::contracts()?
        .into_iter()
        .find(|sql| sql.contains("FUNCTION a4n_validate_galgame_line_family()"))
        .ok_or_else(|| DbErr::Custom("缺少已部署的 Galgame 行约束".into()))?;
    replace(
        &mut sql,
        "    line_row galgame_line%ROWTYPE;",
        "    line_row galgame_line%ROWTYPE;\n    adaptation_ref JSONB;",
        1,
    )?;
    replace(
        &mut sql,
        "'schema_version',",
        "'schema_version', 'origin', 'beat_ref',",
        2,
    )?;
    replace(
        &mut sql,
        "'galgame_line_presentation_v2'",
        "'galgame_line_presentation_v3'",
        1,
    )?;
    sql = sql.replace("v2", "v3");
    replace(
        &mut sql,
        "    IF line_row.expression IS NOT NULL",
        &format!("{SOURCE_IDENTITY_SQL}\n    IF line_row.expression IS NOT NULL"),
        1,
    )?;
    Ok(sql)
}

fn replace(sql: &mut String, old: &str, new: &str, count: usize) -> Result<(), DbErr> {
    if sql.matches(old).count() != count {
        return Err(DbErr::Custom(format!(
            "媒体创作迁移的约束前置版本不一致: {old}"
        )));
    }
    *sql = sql.replace(old, new);
    Ok(())
}

const SOURCE_IDENTITY_SQL: &str = r#"
    IF COALESCE(line_row.presentation_json->>'origin','') NOT IN ('source','adapted','added','historical') THEN
        RAISE EXCEPTION 'Galgame 行内容来源无效';
    END IF;
    IF line_row.presentation_json->>'origin' IN ('source','adapted')
       AND line_row.presentation_json::jsonb->'beat_ref' = 'null'::jsonb THEN
        RAISE EXCEPTION '原文或改编演出必须引用本地节拍';
    END IF;
    IF line_row.presentation_json->>'origin' = 'added' AND line_row.text_anchor_id IS NOT NULL THEN
        RAISE EXCEPTION '新增演出不能冒充原文依据';
    END IF;
    FOREACH adaptation_ref IN ARRAY ARRAY[line_row.presentation_json::jsonb->'beat_ref', line_row.presentation_json::jsonb->'after_beat_ref'] LOOP
        IF adaptation_ref = 'null'::jsonb THEN CONTINUE; END IF;
        IF jsonb_typeof(adaptation_ref) IS DISTINCT FROM 'object'
           OR NOT (adaptation_ref ?& ARRAY['source_chapter_id','unit_id','beat_id'])
           OR adaptation_ref - ARRAY['source_chapter_id','unit_id','beat_id'] <> '{}'::jsonb
           OR jsonb_typeof(adaptation_ref->'source_chapter_id') IS DISTINCT FROM 'number'
           OR adaptation_ref->>'source_chapter_id' !~ '^[1-9][0-9]*$'
           OR jsonb_typeof(adaptation_ref->'unit_id') IS DISTINCT FROM 'string'
           OR jsonb_typeof(adaptation_ref->'beat_id') IS DISTINCT FROM 'string'
           OR NULLIF(BTRIM(adaptation_ref->>'unit_id'),'') IS NULL
           OR NULLIF(BTRIM(adaptation_ref->>'beat_id'),'') IS NULL THEN
            RAISE EXCEPTION 'Galgame 本地节拍引用格式无效';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM galgame_scene scene JOIN galgame_script script ON script.id=scene.script_id
            JOIN source_snapshot_chapter source ON source.id=scene.source_snapshot_chapter_id
            JOIN media_authoring_document plan ON plan.id=scene.adaptation_plan_id
            CROSS JOIN LATERAL jsonb_array_elements(plan.document_json->'units') unit(value)
            CROSS JOIN LATERAL jsonb_array_elements(unit.value->'beats') beat(value)
            WHERE scene.id=line_row.scene_id AND plan.project_id=script.project_id
              AND plan.kind='adaptation_plan' AND plan.status IN ('adopted','superseded')
              AND plan.source_snapshot_id=source.snapshot_id AND plan.source_chapter_id=source.source_chapter_id
              AND adaptation_ref->>'source_chapter_id'=source.source_chapter_id::text
              AND adaptation_ref->>'unit_id'=scene.adaptation_unit_id AND unit.value->>'id'=scene.adaptation_unit_id
              AND beat.value->>'id'=adaptation_ref->>'beat_id'
              AND (line_row.presentation_json->>'origin' NOT IN ('source','adapted')
                  OR line_row.text_anchor_id IS NOT NULL
                  OR adaptation_ref IS DISTINCT FROM line_row.presentation_json::jsonb->'beat_ref'
                  OR jsonb_array_length(beat.value->'source_span_ids') > 0)
        ) THEN
            RAISE EXCEPTION '演出引用的节拍不属于该场景的本地改编版本';
        END IF;
    END LOOP;
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn v3_contract_preserves_existing_integrity_and_checks_local_ownership() {
        let sql = super::line_contract().unwrap();
        assert!(sql.contains("galgame_line_presentation_v3"));
        assert!(!sql.contains("galgame_line_presentation_v2"));
        for required in [
            "visible_cast",
            "choice_count NOT BETWEEN 2 AND 3",
            "plan.project_id=script.project_id",
            "'origin', 'beat_ref'",
            "新增演出不能冒充原文依据",
        ] {
            assert!(sql.contains(required), "missing {required}");
        }
    }
}
