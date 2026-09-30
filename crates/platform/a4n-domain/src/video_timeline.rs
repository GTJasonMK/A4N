//! 章节视频时间线的稳定身份与顺序规则。
//!
//! 章节视频规划不能用展示用的 `segment_number` 作为身份：插入一个片段
//! 不应重新编号或覆盖原生规划。本模块只包含无 IO 的规则，数据库事务和
//! API 层负责把这些规则应用到持久化记录。

mod algorithm;
mod contracts;

pub use algorithm::{
    move_inserted_timeline_item, remove_inserted_timeline_item, timeline_gap_index,
};
pub use contracts::{NativeVideoTimelineKey, TimelineError, VideoTimelineGap};

#[cfg(test)]
mod tests {
    use super::*;
    use algorithm::insert_timeline_item;

    #[test]
    fn insertion_preserves_native_order_and_supports_all_edges() {
        let order = [1, 2, 3];
        assert_eq!(
            VideoTimelineGap {
                previous_item_id: None,
                next_item_id: Some(1),
            }
            .canonical_key()
            .unwrap(),
            "0:1"
        );
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
