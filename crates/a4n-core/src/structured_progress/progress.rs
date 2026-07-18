use super::contracts::StructuredProgressEvent;
use super::parser::StructuredJsonProgressTracker;

impl StructuredJsonProgressTracker {
    pub(super) fn complete_field(&mut self, key: &str, events: &mut Vec<StructuredProgressEvent>) {
        let Some(milestone) = self
            .spec
            .milestones
            .iter()
            .find(|milestone| milestone.key == key)
            .copied()
        else {
            return;
        };
        let weight = milestone.weight.max(1);
        let item_index = {
            let Some(item) = self.current_item.as_mut() else {
                return;
            };
            if !item.completed_fields.insert(milestone.key) {
                return;
            }
            item.completed_units = item
                .completed_units
                .checked_add(u128::from(weight))
                .expect("structured progress item units exceed u128");
            item.index
        };
        self.completed_units = self
            .completed_units
            .checked_add(u128::from(weight))
            .expect("structured progress completed units exceed u128");
        let current = self.current_progress();
        events.push(StructuredProgressEvent {
            current,
            total: 100,
            message: format!(
                "{}：{}已完成",
                self.item_progress_label(item_index),
                milestone.label
            ),
        });
    }

    pub(super) fn complete_item(&mut self, events: &mut Vec<StructuredProgressEvent>) {
        let item_index = {
            let Some(item) = self.current_item.as_mut() else {
                return;
            };
            if item.completed {
                return;
            }
            item.completed = true;
            item.completed_units = item
                .completed_units
                .checked_add(1)
                .expect("structured progress item units exceed u128");
            item.index
        };
        self.completed_units = self
            .completed_units
            .checked_add(1)
            .expect("structured progress completed units exceed u128");
        let current = self.current_progress();
        events.push(StructuredProgressEvent {
            current,
            total: 100,
            message: format!("{}已完成", self.item_progress_label(item_index)),
        });
    }

    fn item_progress_label(&self, item_index: usize) -> String {
        match self.expected_item_label_count() {
            Some(expected_count) => {
                format!(
                    "第 {}/{} 个{}",
                    item_index, expected_count, self.spec.item_label
                )
            }
            None => format!("第 {} 个{}", item_index, self.spec.item_label),
        }
    }

    fn expected_item_label_count(&self) -> Option<usize> {
        if self.spec.root_array_key.is_empty() {
            return Some(self.spec.expected_items.unwrap_or(1));
        }
        self.spec.expected_items
    }

    fn current_progress(&mut self) -> u32 {
        let current = if let Some(expected_items) = self.expected_item_label_count() {
            let total_units = self
                .unit_per_item
                .checked_mul(expected_items as u128)
                .expect("structured progress expected units exceed u128");
            self.span
                .current_for_units(self.completed_units, total_units)
        } else {
            self.current_progress_without_known_total()
        };
        self.max_reported_current = self.max_reported_current.max(current);
        self.max_reported_current
    }

    fn current_progress_without_known_total(&self) -> u32 {
        let width = self.span.end() - self.span.start();
        if width == 0 {
            return self.span.start();
        }
        let reserve_units = self.unit_per_item.max(1);
        let denominator = self
            .completed_units
            .checked_add(reserve_units)
            .expect("structured progress denominator exceeds u128")
            .max(1);
        let offset = u128::from(width) * self.completed_units / denominator;
        self.span.start()
            + u32::try_from(offset)
                .expect("structured progress offset is bounded by the u32 span width")
    }
}
