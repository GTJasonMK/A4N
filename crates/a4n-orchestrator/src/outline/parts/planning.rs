use std::collections::BTreeMap;

use a4n_api_contract::outline::OutlineDeleteTarget;
use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;

pub(super) fn plan_next_parts(
    existing_parts: &[part_outline::Model],
    part_count: i32,
    chapters_per_part: i32,
) -> Result<Vec<(i32, i32, i32)>, AppError> {
    if part_count <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 part_count 必须大于 0".to_string(),
        ));
    }
    if chapters_per_part <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 chapters_per_part 必须大于 0".to_string(),
        ));
    }

    let (next_part_number, next_start_chapter) = existing_parts
        .iter()
        .max_by_key(|part| part.part_number)
        .map(|part| (part.part_number + 1, part.end_chapter + 1))
        .unwrap_or((1, 1));
    Ok((0..part_count)
        .map(|offset| {
            let part_number = next_part_number + offset;
            let start_chapter = next_start_chapter + offset * chapters_per_part;
            let end_chapter = start_chapter + chapters_per_part - 1;
            (part_number, start_chapter, end_chapter)
        })
        .collect())
}

pub(super) fn plan_replacement_parts(
    part_count: i32,
    chapters_per_part: i32,
) -> Result<Vec<(i32, i32, i32)>, AppError> {
    plan_next_parts(&[], part_count, chapters_per_part)
}

pub(super) fn validate_replacement_part_sources(
    existing_parts: &[part_outline::Model],
    expected_parts: &[OutlineDeleteTarget],
) -> Result<i32, AppError> {
    if expected_parts.is_empty() || expected_parts.len() > 1024 {
        return Err(AppError::InvalidArgument(
            "重写全部分部必须提交 1..=1024 条当前分部版本".to_string(),
        ));
    }
    let mut expected_by_id = BTreeMap::new();
    for target in expected_parts {
        let expected_updated_at = target.expected_updated_at.trim();
        if target.id <= 0 || expected_updated_at.is_empty() || expected_updated_at.len() > 64 {
            return Err(AppError::InvalidArgument(
                "分部版本无效，请刷新后重新确认".to_string(),
            ));
        }
        if expected_by_id
            .insert(target.id, expected_updated_at)
            .is_some()
        {
            return Err(AppError::InvalidArgument(format!(
                "重写目标包含重复分部: {}",
                target.id
            )));
        }
    }
    if existing_parts.len() != expected_parts.len()
        || existing_parts.iter().any(|part| {
            expected_by_id
                .get(&part.id)
                .is_none_or(|expected_updated_at| {
                    **expected_updated_at != part.updated_at.to_string()
                })
        })
    {
        return Err(AppError::InvalidArgument(
            "分部列表或内容已变化，请刷新后重新确认重写全部分部".to_string(),
        ));
    }
    let existing_part_count = i32::try_from(existing_parts.len()).map_err(|_| {
        AppError::InvalidArgument("当前分部数量超过 i32 范围，无法重新生成".to_string())
    })?;
    Ok(existing_part_count)
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::outline::OutlineDeleteTarget;
    use a4n_core::models::chapter::part_outline;
    use chrono::DateTime;

    use super::validate_replacement_part_sources;

    fn part(id: i64, updated_at_seconds: i64) -> part_outline::Model {
        let updated_at = DateTime::from_timestamp(updated_at_seconds, 0)
            .unwrap()
            .naive_utc();
        part_outline::Model {
            id,
            project_id: 1,
            part_number: id as i32,
            title: format!("第 {id} 部"),
            start_chapter: id as i32,
            end_chapter: id as i32,
            summary: None,
            theme: None,
            key_events: None,
            character_focus: None,
            created_at: updated_at,
            updated_at,
        }
    }

    fn target(part: &part_outline::Model) -> OutlineDeleteTarget {
        OutlineDeleteTarget {
            id: part.id,
            expected_updated_at: part.updated_at.to_string(),
        }
    }

    #[test]
    fn replacement_requires_the_exact_current_part_versions() {
        let parts = vec![part(1, 1), part(2, 2)];
        let targets = parts.iter().map(target).collect::<Vec<_>>();

        assert_eq!(
            validate_replacement_part_sources(&parts, &targets).unwrap(),
            2
        );

        let mut stale = targets.clone();
        stale[1].expected_updated_at = part(2, 1).updated_at.to_string();
        assert!(validate_replacement_part_sources(&parts, &stale).is_err());
        assert!(validate_replacement_part_sources(&parts, &targets[..1]).is_err());
        assert!(validate_replacement_part_sources(&parts, &[]).is_err());
    }
}
