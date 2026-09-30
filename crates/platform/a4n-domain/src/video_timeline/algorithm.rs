use super::contracts::{TimelineError, VideoTimelineGap};

pub(super) fn insert_timeline_item(
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

pub fn timeline_gap_index(order: &[i64], gap: VideoTimelineGap) -> Result<usize, TimelineError> {
    if order.iter().any(|id| *id <= 0) {
        return Err(TimelineError::InvalidId);
    }
    if order.len() != order.iter().collect::<std::collections::HashSet<_>>().len() {
        return Err(TimelineError::InvalidOrder);
    }
    gap_index(order, gap.validate()?)
}

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
