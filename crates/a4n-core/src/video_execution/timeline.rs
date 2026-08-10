//! 章节视频时间线的稳定身份与顺序规则。
//!
//! 章节视频规划不能用展示用的 `segment_number` 作为身份：插入一个片段
//! 不应重新编号或覆盖原生规划。本模块只包含无 IO 的规则，数据库事务和
//! API 层负责把这些规则应用到持久化记录。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 原生片段的稳定匹配键。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeVideoTimelineKey {
    pub source_shot_id: String,
    pub segment_index_in_shot: i32,
}

impl NativeVideoTimelineKey {
    pub fn new(
        source_shot_id: impl Into<String>,
        segment_index_in_shot: i32,
    ) -> Result<Self, String> {
        let source_shot_id = source_shot_id.into().trim().to_string();
        if source_shot_id.is_empty() {
            return Err("source_shot_id 不能为空".to_string());
        }
        if segment_index_in_shot <= 0 {
            return Err("segment_index_in_shot 必须大于 0".to_string());
        }
        Ok(Self {
            source_shot_id,
            segment_index_in_shot,
        })
    }
}

/// 用于事务插入/移动的时间线间隙。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VideoTimelineGap {
    pub previous_item_id: Option<i64>,
    pub next_item_id: Option<i64>,
}

impl VideoTimelineGap {
    pub fn validate(self) -> Result<Self, TimelineError> {
        if self.previous_item_id.is_some_and(|id| id <= 0)
            || self.next_item_id.is_some_and(|id| id <= 0)
        {
            return Err(TimelineError::InvalidId);
        }
        if self.previous_item_id.is_some() && self.previous_item_id == self.next_item_id {
            return Err(TimelineError::SameNeighbor);
        }
        Ok(self)
    }
}

/// 时间线项顺序错误。
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimelineError {
    #[error("时间线项 ID 必须为正整数")]
    InvalidId,
    #[error("时间线前后项不能是同一个 ID")]
    SameNeighbor,
    #[error("时间线间隙不属于当前顺序")]
    GapNotFound,
    #[error("时间线中存在重复项或空项")]
    InvalidOrder,
}

/// 在现有顺序中插入一个新项，不改变任何已有项的相对顺序。
pub fn insert_timeline_item(
    order: &[i64],
    new_item_id: i64,
    gap: VideoTimelineGap,
) -> Result<Vec<i64>, TimelineError> {
    if new_item_id <= 0 {
        return Err(TimelineError::InvalidId);
    }
    if order.contains(&new_item_id) {
        return Err(TimelineError::InvalidOrder);
    }
    let insertion_index = timeline_gap_index(order, gap)?;
    let mut next = Vec::with_capacity(order.len() + 1);
    next.extend_from_slice(&order[..insertion_index]);
    next.push(new_item_id);
    next.extend_from_slice(&order[insertion_index..]);
    Ok(next)
}

/// 返回合法相邻间隙的插入位置，不创建临时时间线项。
pub fn timeline_gap_index(order: &[i64], gap: VideoTimelineGap) -> Result<usize, TimelineError> {
    if order.iter().any(|id| *id <= 0) {
        return Err(TimelineError::InvalidId);
    }
    if order.len() != order.iter().collect::<std::collections::HashSet<_>>().len() {
        return Err(TimelineError::InvalidOrder);
    }
    gap_index(order, gap.validate()?)
}

/// 移动一个插入项；原生项不可通过此规则移动。
pub fn move_inserted_timeline_item(
    order: &[i64],
    item_id: i64,
    gap: VideoTimelineGap,
) -> Result<Vec<i64>, TimelineError> {
    if item_id <= 0 {
        return Err(TimelineError::InvalidId);
    }
    if !order.contains(&item_id) {
        return Err(TimelineError::GapNotFound);
    }
    let mut without = order.to_vec();
    without.retain(|id| *id != item_id);
    insert_timeline_item(&without, item_id, gap)
}

/// 删除插入项并保持其余项顺序不变。
pub fn remove_inserted_timeline_item(
    order: &[i64],
    item_id: i64,
) -> Result<Vec<i64>, TimelineError> {
    if item_id <= 0 {
        return Err(TimelineError::InvalidId);
    }
    if !order.contains(&item_id) {
        return Err(TimelineError::GapNotFound);
    }
    Ok(order.iter().copied().filter(|id| *id != item_id).collect())
}

fn gap_index(order: &[i64], gap: VideoTimelineGap) -> Result<usize, TimelineError> {
    match (gap.previous_item_id, gap.next_item_id) {
        (None, None) => {
            if order.is_empty() {
                Ok(0)
            } else {
                Err(TimelineError::GapNotFound)
            }
        }
        (None, Some(next)) => {
            let index = order
                .iter()
                .position(|id| *id == next)
                .ok_or(TimelineError::GapNotFound)?;
            if index == 0 {
                Ok(0)
            } else {
                Err(TimelineError::GapNotFound)
            }
        }
        (Some(previous), None) => {
            let index = order
                .iter()
                .position(|id| *id == previous)
                .ok_or(TimelineError::GapNotFound)?;
            if index + 1 == order.len() {
                Ok(order.len())
            } else {
                Err(TimelineError::GapNotFound)
            }
        }
        (Some(previous), Some(next)) => {
            let previous_index = order
                .iter()
                .position(|id| *id == previous)
                .ok_or(TimelineError::GapNotFound)?;
            let next_index = order
                .iter()
                .position(|id| *id == next)
                .ok_or(TimelineError::GapNotFound)?;
            if previous_index + 1 == next_index {
                Ok(next_index)
            } else {
                Err(TimelineError::GapNotFound)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_preserves_native_order_and_supports_all_edges() {
        let order = [1, 2, 3];
        assert_eq!(
            insert_timeline_item(
                &order,
                9,
                VideoTimelineGap {
                    previous_item_id: None,
                    next_item_id: Some(1),
                },
            )
            .unwrap(),
            vec![9, 1, 2, 3]
        );
        assert_eq!(
            insert_timeline_item(
                &order,
                9,
                VideoTimelineGap {
                    previous_item_id: Some(1),
                    next_item_id: Some(2),
                },
            )
            .unwrap(),
            vec![1, 9, 2, 3]
        );
        assert_eq!(
            insert_timeline_item(
                &order,
                9,
                VideoTimelineGap {
                    previous_item_id: Some(3),
                    next_item_id: None,
                },
            )
            .unwrap(),
            vec![1, 2, 3, 9]
        );
    }

    #[test]
    fn stale_or_non_adjacent_gap_is_rejected() {
        assert_eq!(
            VideoTimelineGap {
                previous_item_id: Some(-1),
                next_item_id: None,
            }
            .validate(),
            Err(TimelineError::InvalidId)
        );
        let error = insert_timeline_item(
            &[1, 2, 3],
            9,
            VideoTimelineGap {
                previous_item_id: Some(1),
                next_item_id: Some(3),
            },
        )
        .expect_err("non-adjacent gap must fail");
        assert_eq!(error, TimelineError::GapNotFound);
        assert_eq!(
            timeline_gap_index(
                &[1, 2, 3],
                VideoTimelineGap {
                    previous_item_id: Some(1),
                    next_item_id: Some(2),
                },
            ),
            Ok(1)
        );
    }

    #[test]
    fn moving_and_removing_never_renumber_native_items() {
        let moved = move_inserted_timeline_item(
            &[1, 8, 2, 3],
            8,
            VideoTimelineGap {
                previous_item_id: Some(3),
                next_item_id: None,
            },
        )
        .unwrap();
        assert_eq!(moved, vec![1, 2, 3, 8]);
        let removed = remove_inserted_timeline_item(&moved, 8).unwrap();
        assert_eq!(removed, vec![1, 2, 3]);
    }
}
