use std::collections::{BTreeMap, BTreeSet};

use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_domain::{canvas::CanvasTextAssetRecord, source_text_hash};
use a4n_error::AppError;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    canvas::{
        WORKFLOW_KIND_VIDEO_CANVAS_GRAPH, source_canvas_text_asset_id, upstream_canvas_node_ids,
    },
};

pub(crate) const CANVAS_TEXT_ASSET_SNAPSHOT_KEY: &str = "canvas_text_asset_snapshot";

pub(crate) fn canvas_source_snapshot_node_ids(
    document: &CanvasGraphRecordDocument,
    target_node_id: Option<i64>,
    single_node_id: Option<i64>,
) -> Result<BTreeSet<i64>, String> {
    if let Some(node_id) = single_node_id.or(target_node_id) {
        return upstream_canvas_node_ids(document, node_id);
    }
    Ok(document.nodes.iter().map(|node| node.id).collect())
}

pub(crate) fn capture_canvas_text_asset_snapshot(
    document: &CanvasGraphRecordDocument,
    node_ids: &BTreeSet<i64>,
) -> Result<Vec<CanvasTextAssetRecord>, String> {
    referenced_canvas_text_asset_ids(document, node_ids)?
        .into_iter()
        .map(|asset_id| {
            let mut asset = document
                .text_assets
                .iter()
                .find(|asset| asset.id == asset_id)
                .cloned()
                .ok_or_else(|| format!("画布工作流引用的文本素材不存在: {asset_id}"))?;
            if asset.current_version <= 0
                || asset.content.trim().is_empty()
                || asset.content_hash != source_text_hash(&asset.content)
            {
                return Err(format!(
                    "画布工作流引用的文本素材版本无效: {asset_id}/{}",
                    asset.current_version
                ));
            }
            asset.versions.clear();
            Ok(asset)
        })
        .collect()
}

pub(crate) fn validate_canvas_text_asset_snapshot(
    snapshot: Option<&Value>,
    document: &CanvasGraphRecordDocument,
    node_ids: &BTreeSet<i64>,
) -> Result<Vec<CanvasTextAssetRecord>, String> {
    let required_asset_ids = referenced_canvas_text_asset_ids(document, node_ids)?;

    let Some(snapshot) = snapshot else {
        if required_asset_ids.is_empty() {
            return Ok(Vec::new());
        }
        return Err("该画布运行缺少不可变文本素材快照，无法安全续跑；请重新运行工作流".to_string());
    };

    let assets = serde_json::from_value::<Vec<CanvasTextAssetRecord>>(snapshot.clone())
        .map_err(|error| format!("画布工作流文本素材快照无效: {error}"))?;
    let mut assets_by_id = BTreeMap::new();
    for mut asset in assets {
        if asset.id <= 0
            || asset.project_id != document.graph.project_id
            || asset.current_version <= 0
            || asset.content.trim().is_empty()
            || asset.content_hash != source_text_hash(&asset.content)
        {
            return Err(format!(
                "画布工作流文本素材快照字段无效: asset_id={}",
                asset.id
            ));
        }
        asset.versions.clear();
        if assets_by_id.insert(asset.id, asset).is_some() {
            return Err("画布工作流文本素材快照包含重复素材".to_string());
        }
    }
    for asset_id in required_asset_ids {
        if !assets_by_id.contains_key(&asset_id) {
            return Err(format!("画布工作流文本素材快照缺少引用素材: {asset_id}"));
        }
    }
    Ok(assets_by_id.into_values().collect())
}

fn referenced_canvas_text_asset_ids(
    document: &CanvasGraphRecordDocument,
    node_ids: &BTreeSet<i64>,
) -> Result<BTreeSet<i64>, String> {
    let mut asset_ids = BTreeSet::new();
    for node in document
        .nodes
        .iter()
        .filter(|node| node_ids.contains(&node.id))
    {
        if let Some(asset_id) = source_canvas_text_asset_id(node)? {
            if asset_id <= 0 {
                return Err(format!(
                    "画布节点 {} 引用了无效的文本素材编号: {asset_id}",
                    node.id
                ));
            }
            asset_ids.insert(asset_id);
        }
    }
    Ok(asset_ids)
}

impl OrchestratorContext {
    pub(crate) async fn canvas_document_with_workflow_text_asset_snapshot(
        &self,
        document: &CanvasGraphRecordDocument,
        workflow_run_id: Option<i64>,
        node_id: i64,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        let source_node_ids = canvas_source_snapshot_node_ids(document, None, Some(node_id))
            .map_err(AppError::InvalidArgument)?;
        let Some(workflow_run_id) = workflow_run_id else {
            return Ok(document.clone());
        };

        let run = self.workflow_run_summary(workflow_run_id).await?;
        if run.workflow_kind != WORKFLOW_KIND_VIDEO_CANVAS_GRAPH
            || run.project_id != Some(document.graph.project_id)
            || run.scope_table.as_deref() != Some("canvas_graph")
            || run.scope_id != Some(document.graph.id)
        {
            return Err(AppError::InvalidArgument(format!(
                "工作流运行 {workflow_run_id} 与当前画布不匹配"
            )));
        }
        let snapshot = run
            .input_fingerprint_json
            .as_ref()
            .and_then(|input| input.get(CANVAS_TEXT_ASSET_SNAPSHOT_KEY));
        let mut document = document.clone();
        document.text_assets =
            validate_canvas_text_asset_snapshot(snapshot, &document, &source_node_ids)
                .map_err(AppError::InvalidArgument)?;
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canvas_source_snapshot_node_ids, capture_canvas_text_asset_snapshot,
        validate_canvas_text_asset_snapshot,
    };
    use a4n_db::canvas::CanvasGraphRecordDocument;
    use a4n_domain::canvas::{
        CanvasEdgeRecord, CanvasGraphRecord, CanvasNodeRecord, CanvasTextAssetRecord,
    };
    use chrono::Utc;
    use serde_json::{Value, json};
    use std::collections::BTreeSet;

    fn document(content: &str, version: i32) -> CanvasGraphRecordDocument {
        let now = Utc::now().naive_utc();
        CanvasGraphRecordDocument {
            graph: CanvasGraphRecord {
                id: 10,
                project_id: 20,
                name: "测试画布".to_string(),
                graph_kind: "video_studio".to_string(),
                version: 1,
                viewport_json: None,
                created_at: now,
                updated_at: now,
            },
            nodes: vec![
                CanvasNodeRecord {
                    id: 1,
                    graph_id: 10,
                    node_kind: "TextInputNode".to_string(),
                    ref_type: Some("canvas_text_asset".to_string()),
                    ref_id: Some(30),
                    config_json: json!({}),
                    position_json: json!({"x": 0, "y": 0}),
                    state: "ready".to_string(),
                    created_at: now,
                    updated_at: now,
                },
                CanvasNodeRecord {
                    id: 2,
                    graph_id: 10,
                    node_kind: "LlmTextNode".to_string(),
                    ref_type: None,
                    ref_id: None,
                    config_json: json!({}),
                    position_json: json!({"x": 1, "y": 0}),
                    state: "ready".to_string(),
                    created_at: now,
                    updated_at: now,
                },
                CanvasNodeRecord {
                    id: 3,
                    graph_id: 10,
                    node_kind: "TextInputNode".to_string(),
                    ref_type: None,
                    ref_id: None,
                    config_json: json!({"content": "未引用"}),
                    position_json: json!({"x": 5, "y": 0}),
                    state: "ready".to_string(),
                    created_at: now,
                    updated_at: now,
                },
            ],
            edges: vec![CanvasEdgeRecord {
                id: 12,
                graph_id: 10,
                source_node_id: 1,
                source_port: "text".to_string(),
                target_node_id: 2,
                target_port: "input".to_string(),
                role: "context".to_string(),
                config_json: json!({}),
            }],
            text_assets: vec![
                CanvasTextAssetRecord {
                    id: 30,
                    project_id: 20,
                    title: "脚本".to_string(),
                    text_kind: "script".to_string(),
                    content: content.to_string(),
                    content_hash: a4n_domain::source_text_hash(content),
                    current_version: version,
                    versions: Vec::new(),
                    created_at: now,
                    updated_at: now,
                },
                CanvasTextAssetRecord {
                    id: 31,
                    project_id: 20,
                    title: "未引用".to_string(),
                    text_kind: "script".to_string(),
                    content: "不应进入快照".to_string(),
                    content_hash: "unused".to_string(),
                    current_version: 1,
                    versions: Vec::new(),
                    created_at: now,
                    updated_at: now,
                },
            ],
        }
    }

    #[test]
    fn workflow_snapshot_contains_only_referenced_assets_and_their_version() {
        let document = document("第一版正文", 1);
        let source_nodes = canvas_source_snapshot_node_ids(&document, None, Some(2)).unwrap();
        let snapshot = capture_canvas_text_asset_snapshot(&document, &source_nodes).unwrap();

        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].id, 30);
        assert_eq!(snapshot[0].current_version, 1);
        assert_eq!(snapshot[0].content, "第一版正文");
        assert!(snapshot[0].versions.is_empty());
    }

    #[test]
    fn workflow_snapshot_keeps_the_captured_text_after_the_source_changes() {
        let original = document("第一版正文", 1);
        let source_nodes = canvas_source_snapshot_node_ids(&original, None, Some(2)).unwrap();
        let snapshot = capture_canvas_text_asset_snapshot(&original, &source_nodes).unwrap();
        let raw_snapshot: Value = serde_json::to_value(&snapshot).unwrap();
        let current = document("第二版正文", 2);

        let restored =
            validate_canvas_text_asset_snapshot(Some(&raw_snapshot), &current, &source_nodes)
                .unwrap();

        assert_eq!(restored[0].current_version, 1);
        assert_eq!(restored[0].content, "第一版正文");
        let source_node = current
            .nodes
            .iter()
            .find(|node| node.id == 1)
            .expect("文本输入节点应存在");
        let mut pinned_document = current.clone();
        pinned_document.text_assets = restored;
        assert_eq!(
            super::super::source_text_value(source_node, &pinned_document, None, "LlmTextNode",)
                .unwrap()
                .as_deref(),
            Some("第一版正文")
        );
    }

    #[test]
    fn old_workflows_can_resume_only_when_their_pending_source_has_no_asset() {
        let document = document("第一版正文", 1);
        let source_nodes = canvas_source_snapshot_node_ids(&document, None, Some(2)).unwrap();
        let error = validate_canvas_text_asset_snapshot(None, &document, &source_nodes)
            .expect_err("缺少来源快照必须阻止继续运行");
        assert!(error.contains("缺少不可变文本素材快照"));

        let unrelated_nodes = BTreeSet::from([3]);
        assert!(
            validate_canvas_text_asset_snapshot(None, &document, &unrelated_nodes)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn referenced_asset_does_not_fall_back_to_inline_text_when_missing_from_snapshot() {
        let mut document = document("第一版正文", 1);
        document.nodes[0].config_json = json!({"content": "旧的内嵌文本"});
        document.text_assets.clear();
        let source_node = &document.nodes[0];

        let error = super::super::source_text_value(source_node, &document, None, "LlmTextNode")
            .expect_err("缺少素材快照时应报错，而不是回退到内嵌文本");
        assert!(error.contains("不在当前运行快照中"));
    }
}
