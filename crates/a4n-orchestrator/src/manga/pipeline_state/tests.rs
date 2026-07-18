use a4n_core::error::AppError;
use serde_json::json;

use super::contracts::PipelineProgress;
use super::{pipeline_progress_from_layout, pipeline_status_from_layout};

#[test]
fn reads_pipeline_status_from_layout() {
    let layout = json!({"pipeline_status": " running "});

    assert_eq!(
        pipeline_status_from_layout(Some(&layout), "漫画流水线状态").unwrap(),
        "running"
    );
}

#[test]
fn rejects_missing_pipeline_status() {
    let layout = json!({"status": "cancelled"});

    let err = pipeline_status_from_layout(Some(&layout), "漫画流水线状态").unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("pipeline_status"));
}

#[test]
fn reads_pipeline_progress_from_layout() {
    let layout = json!({"current": 3, "total": 8});

    assert_eq!(
        pipeline_progress_from_layout(Some(&layout), "漫画流水线状态").unwrap(),
        PipelineProgress {
            current: 3,
            total: 8
        }
    );
}

#[test]
fn rejects_missing_pipeline_progress_counts() {
    let layout = json!({"current": 3});

    let err = pipeline_progress_from_layout(Some(&layout), "漫画流水线状态").unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("total"));
}

#[test]
fn rejects_current_greater_than_total() {
    let layout = json!({"current": 8, "total": 7});

    let err = pipeline_progress_from_layout(Some(&layout), "漫画流水线状态").unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("current 不能大于"));
}
