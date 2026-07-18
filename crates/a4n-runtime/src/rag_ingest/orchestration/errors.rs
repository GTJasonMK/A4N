pub(super) fn rag_ingest_error_message(scope: &str, errors: &[String]) -> String {
    let details = error_details(errors);
    if details.is_empty() {
        format!("{scope}失败")
    } else {
        format!("{scope}失败：{details}")
    }
}

pub(super) fn scene_rag_sync_error_message(errors: &[String]) -> String {
    format!("场景 RAG 同步失败：{}", error_details(errors))
}

fn error_details(errors: &[String]) -> String {
    let details = errors
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join("；");
    if errors.len() > 8 {
        format!("{details}；另有 {} 个错误未显示", errors.len() - 8)
    } else {
        details
    }
}

#[cfg(test)]
mod tests {
    use super::{rag_ingest_error_message, scene_rag_sync_error_message};

    #[test]
    fn error_summary_keeps_first_eight_errors_and_reports_the_remainder() {
        let errors = (1..=10)
            .map(|index| format!("错误{index}"))
            .collect::<Vec<_>>();

        assert_eq!(
            rag_ingest_error_message("RAG 同步", &errors),
            "RAG 同步失败：错误1；错误2；错误3；错误4；错误5；错误6；错误7；错误8；另有 2 个错误未显示"
        );
    }

    #[test]
    fn scene_error_summary_preserves_the_scene_specific_prefix() {
        assert_eq!(
            scene_rag_sync_error_message(&["写入失败".to_string()]),
            "场景 RAG 同步失败：写入失败"
        );
    }
}
