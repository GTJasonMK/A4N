use super::{ChapterVisualPlanV3, ChapterVisualPlanValidationError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// IDs, never array offsets: inserting a point cannot redirect a pending edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisualPlanChange {
    pub collection: String,
    pub object_id: String,
    /// Empty for object creation/deletion.
    pub field: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VisualPlanEditVersion {
    pub record_id: i64,
    pub revision: i32,
    pub plan_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VisualPlanEditReceipt {
    pub sequence: i64,
    pub operation_id: String,
    pub project_id: i64,
    pub source_chapter_id: i64,
    pub source_revision_key: String,
    pub author_user_id: i64,
    pub before: Option<VisualPlanEditVersion>,
    pub after: VisualPlanEditVersion,
    pub changes: Vec<VisualPlanChange>,
}

impl VisualPlanChange {
    pub fn inverse(&self) -> Self {
        Self {
            before: self.after.clone(),
            after: self.before.clone(),
            ..self.clone()
        }
    }
}

impl ChapterVisualPlanV3 {
    pub fn merge_edit(
        &self,
        base: &Self,
        desired: &Self,
    ) -> Result<Self, ChapterVisualPlanValidationError> {
        let incoming = base.changes_to(desired);
        let concurrent = base.changes_to(self);
        let incoming_topology = topology_dependencies(base, desired, &incoming);
        let concurrent_topology = topology_dependencies(base, self, &concurrent);
        if !incoming_topology.is_disjoint(&concurrent_topology)
            && incoming
                .iter()
                .filter(|change| structural(change))
                .any(|change| !concurrent.contains(change))
        {
            return Err(ChapterVisualPlanValidationError {
                path: "topology".into(),
                message: "相邻时间点或镜头边界已变化，请重新规划结构操作".into(),
            });
        }
        self.apply_changes(&incoming)
    }

    pub fn changes_to(&self, next: &Self) -> Vec<VisualPlanChange> {
        let before = serde_json::to_value(self).expect("visual plan is JSON");
        let after = serde_json::to_value(next).expect("visual plan is JSON");
        let mut changes = Vec::new();
        for collection in ["frame", "points", "segments"] {
            let old = objects(&before, collection);
            let new = objects(&after, collection);
            let ids = old
                .keys()
                .chain(new.keys())
                .collect::<std::collections::BTreeSet<_>>();
            for id in ids {
                let left = old.get(id);
                let right = new.get(id);
                if left == right {
                    continue;
                }
                if let (Some(left), Some(right)) = (left, right) {
                    for (field, value) in left.as_object().expect("plan object") {
                        if Some(value) != right.get(field) {
                            changes.push(VisualPlanChange {
                                collection: collection.into(),
                                object_id: id.to_string(),
                                field: field.clone(),
                                before: Some(value.clone()),
                                after: right.get(field).cloned(),
                            });
                        }
                    }
                } else {
                    changes.push(VisualPlanChange {
                        collection: collection.into(),
                        object_id: id.to_string(),
                        field: String::new(),
                        before: left.map(|value| (*value).clone()),
                        after: right.map(|value| (*value).clone()),
                    });
                }
            }
        }
        changes
    }

    /// Three-way application also powers selective undo. Conflicts leave self intact.
    pub fn apply_changes(
        &self,
        changes: &[VisualPlanChange],
    ) -> Result<Self, ChapterVisualPlanValidationError> {
        let mut value = serde_json::to_value(self).expect("visual plan is JSON");
        for change in changes {
            let conflict = || ChapterVisualPlanValidationError {
                path: format!(
                    "{}.{}.{}",
                    change.collection, change.object_id, change.field
                ),
                message: "并发修改冲突，请重新读取并解决冲突".into(),
            };
            if !["frame", "points", "segments"].contains(&change.collection.as_str()) {
                return Err(conflict());
            }
            let target = if change.collection == "frame" {
                if change.object_id != "frame" || change.field.is_empty() {
                    return Err(conflict());
                }
                &mut value["frame"]
            } else {
                let items = value[&change.collection]
                    .as_array_mut()
                    .ok_or_else(conflict)?;
                let index = items
                    .iter()
                    .position(|item| item["id"].as_str() == Some(&change.object_id));
                if change.field.is_empty() {
                    let current = index.map(|index| &items[index]);
                    if current == change.after.as_ref() {
                        continue;
                    }
                    if current != change.before.as_ref() {
                        return Err(conflict());
                    }
                    match (index, &change.after) {
                        (Some(index), None) => {
                            items.remove(index);
                        }
                        (None, Some(object))
                            if object["id"].as_str() == Some(&change.object_id) =>
                        {
                            items.push(object.clone())
                        }
                        _ => return Err(conflict()),
                    }
                    continue;
                }
                &mut items[index.ok_or_else(conflict)?]
            };
            let current = target.get(&change.field);
            if current == change.after.as_ref() {
                continue;
            }
            if change.field == "id" || current != change.before.as_ref() || current.is_none() {
                return Err(conflict());
            }
            target[&change.field] = change.after.clone().ok_or_else(conflict)?;
        }
        let mut next: Self =
            serde_json::from_value(value).map_err(|error| ChapterVisualPlanValidationError {
                path: "changes".into(),
                message: error.to_string(),
            })?;
        next.points.sort_by_key(|point| point.at_ms);
        let times: BTreeMap<_, _> = next
            .points
            .iter()
            .map(|point| (point.id.as_str(), point.at_ms))
            .collect();
        next.segments
            .sort_by_key(|segment| times.get(segment.start_point_id.as_str()).copied());
        next.validate_draft()?;
        Ok(next)
    }
}

fn structural(change: &VisualPlanChange) -> bool {
    change.field.is_empty()
        || matches!(
            change.field.as_str(),
            "at_ms" | "start_point_id" | "end_point_id"
        )
}

fn topology_dependencies(
    base: &ChapterVisualPlanV3,
    desired: &ChapterVisualPlanV3,
    changes: &[VisualPlanChange],
) -> std::collections::BTreeSet<String> {
    let mut dependencies = std::collections::BTreeSet::new();
    for change in changes.iter().filter(|change| structural(change)) {
        dependencies.insert(format!("{}:{}", change.collection, change.object_id));
        for plan in [base, desired] {
            if change.collection == "points" {
                if let Some(index) = plan
                    .points
                    .iter()
                    .position(|point| point.id == change.object_id)
                {
                    for point in
                        &plan.points[index.saturating_sub(1)..(index + 2).min(plan.points.len())]
                    {
                        dependencies.insert(format!("points:{}", point.id));
                    }
                }
                for segment in &plan.segments {
                    if segment.start_point_id == change.object_id
                        || segment.end_point_id == change.object_id
                    {
                        dependencies.insert(format!("segments:{}", segment.id));
                    }
                }
            } else if let Some(segment) = plan
                .segments
                .iter()
                .find(|segment| segment.id == change.object_id)
            {
                dependencies.insert(format!("points:{}", segment.start_point_id));
                dependencies.insert(format!("points:{}", segment.end_point_id));
            }
        }
    }
    dependencies
}

fn objects<'a>(value: &'a Value, collection: &str) -> BTreeMap<&'a str, &'a Value> {
    if collection == "frame" {
        return BTreeMap::from([("frame", &value["frame"])]);
    }
    value[collection]
        .as_array()
        .expect("plan collection")
        .iter()
        .map(|item| (item["id"].as_str().expect("plan ID"), item))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disjoint_edits_merge_and_inverse_preserves_intervening_work() {
        let base = ChapterVisualPlanV3::starter();
        let mut desired = base.clone();
        desired.points[0].description = "Agent description".into();
        let changes = base.changes_to(&desired);
        let mut current = base.clone();
        current.points[0].label = "Manual label".into();
        let merged = current.apply_changes(&changes).unwrap();
        assert_eq!(merged.points[0].label, "Manual label");
        assert_eq!(merged.points[0].description, "Agent description");
        assert_eq!(merged.apply_changes(&changes).unwrap(), merged);
        let inverse: Vec<_> = changes
            .iter()
            .rev()
            .map(VisualPlanChange::inverse)
            .collect();
        assert_eq!(merged.apply_changes(&inverse).unwrap(), current);
        current.points[0].description = "Overlapping manual input".into();
        assert!(current.apply_changes(&changes).is_err());
    }
}
