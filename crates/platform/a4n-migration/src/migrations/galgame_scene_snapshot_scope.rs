//! A script keeps its initial provenance; each scene owns its actual source revision.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

use super::{
    galgame_generation_spec_contract::GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
    galgame_media_binding_contract::GALGAME_MEDIA_BINDING_FUNCTION_SQL,
    galgame_script_integrity_contract::{
        GALGAME_LINE_FAMILY_FUNCTION_SQL, GALGAME_SCENE_FAMILY_FUNCTION_SQL,
        GALGAME_SCRIPT_FAMILY_FUNCTION_SQL,
    },
    galgame_target_reverse_integrity_contract::GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL,
    media_asset_generation_ref_contract::MEDIA_ASSET_FAMILY_STRICT_FUNCTION_SQL,
    media_provenance_snapshot_context::MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in contracts()? {
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "galgame_scene_snapshot_scope 不可逆：逐场更新后的剧本不再具有单一源快照".into(),
        ))
    }
}

const PROJECT_SNAPSHOT_SCOPE_SQL: &str = r#"
CREATE FUNCTION a4n_project_source_snapshot_allowed(target_project_id BIGINT, target_snapshot_id BIGINT)
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM novel_project project
        JOIN project_source_snapshot snapshot ON snapshot.id = target_snapshot_id
            AND snapshot.root_project_id = CASE WHEN project.project_kind = 'text'
                THEN project.id ELSE project.root_project_id END
        WHERE project.id = target_project_id
          AND (
              project.project_kind = 'text'
              OR project.source_snapshot_id = target_snapshot_id
              OR (project.project_kind = 'galgame' AND EXISTS (
                  SELECT 1 FROM galgame_script script
                  JOIN galgame_scene scene ON scene.script_id = script.id
                  JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
                  WHERE script.project_id = project.id AND chapter.snapshot_id = target_snapshot_id
              ))
          )
    )
$$;
"#;

// Derive the new contracts from immutable deployed SQL. Matching counts make any
// historical edit fail explicitly instead of silently installing a partial change.
fn rewrite(sql: &str, changes: &[(&str, &str, usize)]) -> Result<String, DbErr> {
    let mut sql = sql.to_string();
    for (old, new, count) in changes {
        if sql.matches(old).count() != *count {
            return Err(DbErr::Custom(format!(
                "galgame_scene_snapshot_scope: expected {count} occurrences of {old:?}"
            )));
        }
        sql = sql.replace(old, new);
    }
    Ok(sql)
}

pub(crate) fn contracts() -> Result<Vec<String>, DbErr> {
    Ok(vec![
        PROJECT_SNAPSHOT_SCOPE_SQL.to_string(),
        rewrite(
            GALGAME_SCRIPT_FAMILY_FUNCTION_SQL,
            &[
                (
                    "\n       OR project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id",
                    "",
                    1,
                ),
                (
                    "必须匹配派生项目的根项目和源快照",
                    "必须匹配派生项目的根项目",
                    1,
                ),
            ],
        )?,
        rewrite(
            GALGAME_SCENE_FAMILY_FUNCTION_SQL,
            &[
                (
                    "      AND chapter.snapshot_id = script.source_snapshot_id\n",
                    "",
                    1,
                ),
                ("必须属于剧本源快照", "必须属于剧本根项目", 1),
            ],
        )?,
        rewrite(
            GALGAME_LINE_FAMILY_FUNCTION_SQL,
            &[
                ("script.source_snapshot_id", "chapter.snapshot_id", 1),
                ("script_source_snapshot_id", "scene_source_snapshot_id", 3),
            ],
        )?,
        rewrite(
            GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
            &[
                (
                    "project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id",
                    "NOT a4n_project_source_snapshot_allowed(NEW.project_id, NEW.source_snapshot_id)",
                    1,
                ),
                (
                    "必须属于同一源快照的 Galgame 派生项目",
                    "必须属于 Galgame 项目实际使用的源快照",
                    1,
                ),
                (
                    "               OR script_row.source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id\n",
                    "",
                    1,
                ),
                (
                    "script_row.source_snapshot_id",
                    "chapter_row.snapshot_id",
                    2,
                ),
                (
                    "target_script_source_snapshot_id",
                    "target_scene_source_snapshot_id",
                    5,
                ),
            ],
        )?,
        rewrite(
            GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL,
            &[
                (
                    "                  OR chapter.snapshot_id IS DISTINCT FROM script.source_snapshot_id\n",
                    "",
                    1,
                ),
                (
                    "必须仍属于剧本源快照章节",
                    "必须仍属于剧本根项目的源快照章节",
                    1,
                ),
                (
                    "anchor.source_snapshot_id IS DISTINCT FROM script.source_snapshot_id",
                    "anchor.source_snapshot_id IS DISTINCT FROM chapter.snapshot_id",
                    1,
                ),
                (
                    "                             OR script.source_snapshot_id IS DISTINCT FROM spec.source_snapshot_id\n",
                    "",
                    1,
                ),
                (
                    "script.source_snapshot_id IS DISTINCT FROM spec.source_snapshot_id",
                    "chapter.snapshot_id IS DISTINCT FROM spec.source_snapshot_id",
                    2,
                ),
                (
                    "script.source_snapshot_id IS DISTINCT FROM asset.source_snapshot_id",
                    "chapter.snapshot_id IS DISTINCT FROM asset.source_snapshot_id",
                    3,
                ),
                (
                    "LEFT JOIN galgame_script script ON script.id = scene.script_id\n            WHERE",
                    "LEFT JOIN galgame_script script ON script.id = scene.script_id\n            LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id\n            WHERE",
                    3,
                ),
            ],
        )?,
        rewrite(
            GALGAME_MEDIA_BINDING_FUNCTION_SQL,
            &[(
                "asset.source_snapshot_id = script.source_snapshot_id",
                "asset.source_snapshot_id = chapter.snapshot_id",
                2,
            )],
        )?,
        rewrite(
            MEDIA_ASSET_FAMILY_STRICT_FUNCTION_SQL,
            &[
                (
                    "project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id",
                    "NOT a4n_project_source_snapshot_allowed(NEW.project_id, NEW.source_snapshot_id)",
                    1,
                ),
                (
                    "必须等于派生项目源快照",
                    "必须属于派生项目实际使用的源快照",
                    1,
                ),
                (
                    "NEW.source_snapshot_id IS DISTINCT FROM script.source_snapshot_id",
                    "NEW.source_snapshot_id IS DISTINCT FROM chapter.snapshot_id",
                    3,
                ),
                (
                    "LEFT JOIN galgame_script script ON script.id = scene.script_id\n        WHERE",
                    "LEFT JOIN galgame_script script ON script.id = scene.script_id\n        LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id\n        WHERE",
                    1,
                ),
            ],
        )?,
        rewrite(
            MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL,
            &[
                (
                    "asset.source_snapshot_id = script.source_snapshot_id",
                    "asset.source_snapshot_id = chapter.snapshot_id\n              AND (asset.source_snapshot_chapter_id IS NULL OR asset.source_snapshot_chapter_id = scene.source_snapshot_chapter_id)",
                    1,
                ),
                (
                    "JOIN galgame_script script ON script.id = scene.script_id\n            WHERE",
                    "JOIN galgame_script script ON script.id = scene.script_id\n            JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id\n            WHERE",
                    1,
                ),
            ],
        )?,
    ])
}

#[cfg(test)]
mod tests;
