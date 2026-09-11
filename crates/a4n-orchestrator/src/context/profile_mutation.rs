use a4n_domain::DownstreamConflictStrategy;
use a4n_error::AppError;

pub(super) fn resolve_downstream_image_ids(
    image_ids: Vec<i64>,
    strategy: DownstreamConflictStrategy,
    subject: &str,
    operation: &str,
) -> Result<Vec<i64>, AppError> {
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    match strategy {
        DownstreamConflictStrategy::Reject => Err(AppError::InvalidArgument(format!(
            "{subject}已派生 {} 张素材图，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖源档案",
            image_ids.len()
        ))),
        DownstreamConflictStrategy::KeepDownstream => Ok(Vec::new()),
        DownstreamConflictStrategy::DeleteDownstream => Ok(image_ids),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_downstream_image_ids;
    use a4n_domain::DownstreamConflictStrategy;

    #[test]
    fn downstream_strategy_keeps_or_deletes_only_when_requested() {
        let ids = vec![1, 2];

        assert!(
            resolve_downstream_image_ids(
                ids.clone(),
                DownstreamConflictStrategy::Reject,
                "档案",
                "修改"
            )
            .is_err()
        );
        assert!(
            resolve_downstream_image_ids(
                ids.clone(),
                DownstreamConflictStrategy::KeepDownstream,
                "档案",
                "修改"
            )
            .expect("keep strategy should preserve downstream")
            .is_empty()
        );
        assert_eq!(
            resolve_downstream_image_ids(
                ids.clone(),
                DownstreamConflictStrategy::DeleteDownstream,
                "档案",
                "修改"
            )
            .expect("delete strategy should return cleanup ids"),
            ids
        );
    }
}
