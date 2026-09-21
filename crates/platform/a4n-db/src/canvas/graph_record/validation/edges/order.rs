use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use serde_json::Value;

use crate::canvas::CanvasDbError;

pub(in crate::canvas::graph_record::validation) fn canvas_record_edge_order_index(
    config: Option<&Value>,
    label: &str,
) -> Result<i64, CanvasDbError> {
    let order_index = config
        .and_then(|config| config.get("order_index"))
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!("{label} 片段连线必须设置 order_index"))
        })?;
    if order_index <= 0 {
        return Err(CanvasDbError::InvalidArgument(format!(
            "{label} 片段连线 order_index 必须是正整数"
        )));
    }
    Ok(order_index)
}

pub(in crate::canvas::graph_record::validation) fn validate_canvas_record_contiguous_order_indexes<
    K: Ord + Display,
>(
    orders_by_target: &BTreeMap<K, Vec<i64>>,
    label: &str,
) -> Result<(), CanvasDbError> {
    for (target, orders) in orders_by_target {
        let mut seen = BTreeSet::new();
        for order in orders {
            if !seen.insert(*order) {
                return Err(CanvasDbError::InvalidArgument(format!(
                    "{label} 片段顺序重复: target={target}, order_index={order}"
                )));
            }
        }
        for expected in 1..=orders.len() as i64 {
            if !seen.contains(&expected) {
                return Err(CanvasDbError::InvalidArgument(format!(
                    "{label} 片段顺序必须从 1 连续递增: target={target}, 缺少 order_index={expected}"
                )));
            }
        }
    }
    Ok(())
}
