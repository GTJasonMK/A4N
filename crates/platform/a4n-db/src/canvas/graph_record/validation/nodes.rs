use crate::canvas::CanvasDbError;
use a4n_domain::CanvasNodeRefKind;

use super::super::types::CanvasNodeRecordInput;

pub(in crate::canvas::graph_record) fn validate_canvas_record_node_ref_shape(
    node: &CanvasNodeRecordInput,
) -> Result<Option<CanvasNodeRefKind>, CanvasDbError> {
    let Some(ref_type) = node.ref_type.as_deref() else {
        if node.ref_id.is_some() {
            return Err(CanvasDbError::InvalidArgument(
                "node.ref_id 不能在 node.ref_type 为空时设置".to_string(),
            ));
        }
        return Ok(None);
    };
    let kind = ref_type
        .parse::<CanvasNodeRefKind>()
        .map_err(|error| CanvasDbError::InvalidArgument(error.to_string()))?;
    match kind {
        CanvasNodeRefKind::Inline => {
            if node.ref_id.is_some() {
                return Err(CanvasDbError::InvalidArgument(
                    "inline 节点引用不能设置 ref_id".to_string(),
                ));
            }
        }
        CanvasNodeRefKind::Config => {
            if let Some(ref_id) = node.ref_id {
                ensure_canvas_record_positive_ref_id(ref_id, ref_type)?;
            }
        }
        _ => {
            let ref_id = node.ref_id.ok_or_else(|| {
                CanvasDbError::InvalidArgument(format!("{ref_type} 节点引用必须设置 ref_id"))
            })?;
            ensure_canvas_record_positive_ref_id(ref_id, ref_type)?;
        }
    }
    Ok(Some(kind))
}

fn ensure_canvas_record_positive_ref_id(ref_id: i64, ref_type: &str) -> Result<(), CanvasDbError> {
    if ref_id > 0 {
        Ok(())
    } else {
        Err(CanvasDbError::InvalidArgument(format!(
            "{ref_type} 节点引用 ref_id 必须是正整数"
        )))
    }
}
