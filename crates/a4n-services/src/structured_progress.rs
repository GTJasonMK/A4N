use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub struct ProgressSpan {
    start: u32,
    end: u32,
}

impl ProgressSpan {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub const fn start(self) -> u32 {
        self.start
    }

    pub const fn end(self) -> u32 {
        self.end
    }

    pub fn split(self, index: usize, total: usize) -> Self {
        let total = total.max(1) as u32;
        let index = index.min(total as usize - 1) as u32;
        let width = self.end.saturating_sub(self.start);
        let start = self.start + width.saturating_mul(index) / total;
        let end = self.start + width.saturating_mul(index + 1) / total;
        Self { start, end }
    }

    pub fn first_half(self) -> Self {
        let mid = self.start + self.end.saturating_sub(self.start) / 2;
        Self {
            start: self.start,
            end: mid,
        }
    }

    pub fn second_half(self) -> Self {
        let mid = self.start + self.end.saturating_sub(self.start) / 2;
        Self {
            start: mid,
            end: self.end,
        }
    }

    fn current_for_units(self, completed_units: u32, total_units: u32) -> u32 {
        let total_units = total_units.max(1);
        let width = self.end.saturating_sub(self.start);
        self.start + width.saturating_mul(completed_units.min(total_units)) / total_units
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FieldMilestone {
    key: &'static str,
    label: &'static str,
    weight: u32,
}

impl FieldMilestone {
    pub const fn new(key: &'static str, label: &'static str, weight: u32) -> Self {
        Self { key, label, weight }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StructuredJsonProgressSpec {
    pub root_array_key: &'static str,
    pub item_label: &'static str,
    pub expected_items: Option<usize>,
    pub milestones: &'static [FieldMilestone],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredProgressEvent {
    pub current: u32,
    pub total: u32,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerKind {
    Object,
    Array,
}

#[derive(Debug, Clone)]
struct Container {
    kind: ContainerKind,
    expecting_key: bool,
    pending_key: Option<String>,
    is_root_array: bool,
    is_root_item: bool,
    field_key_for_root_item: Option<String>,
}

impl Container {
    fn object(is_root_item: bool, field_key_for_root_item: Option<String>) -> Self {
        Self {
            kind: ContainerKind::Object,
            expecting_key: true,
            pending_key: None,
            is_root_array: false,
            is_root_item,
            field_key_for_root_item,
        }
    }

    fn array(is_root_array: bool, field_key_for_root_item: Option<String>) -> Self {
        Self {
            kind: ContainerKind::Array,
            expecting_key: false,
            pending_key: None,
            is_root_array,
            is_root_item: false,
            field_key_for_root_item,
        }
    }
}

#[derive(Default)]
struct ItemProgress {
    index: usize,
    completed_fields: HashSet<&'static str>,
    completed_units: u32,
    completed: bool,
}

pub struct StructuredJsonProgressTracker {
    spec: StructuredJsonProgressSpec,
    span: ProgressSpan,
    started_json: bool,
    in_string: bool,
    escaping: bool,
    string_buffer: String,
    stack: Vec<Container>,
    current_item: Option<ItemProgress>,
    completed_units: u32,
    item_count: usize,
    unit_per_item: u32,
    max_reported_current: u32,
}

impl StructuredJsonProgressTracker {
    pub fn new(spec: StructuredJsonProgressSpec, span: ProgressSpan) -> Self {
        let milestone_units = spec
            .milestones
            .iter()
            .map(|milestone| milestone.weight.max(1))
            .sum::<u32>();
        Self {
            spec,
            span,
            started_json: false,
            in_string: false,
            escaping: false,
            string_buffer: String::new(),
            stack: Vec::new(),
            current_item: None,
            completed_units: 0,
            item_count: 0,
            unit_per_item: milestone_units + 1,
            max_reported_current: span.start(),
        }
    }

    pub fn consume(&mut self, delta: &str) -> Vec<StructuredProgressEvent> {
        let mut events = Vec::new();
        for ch in delta.chars() {
            self.consume_char(ch, &mut events);
        }
        events
    }

    fn consume_char(&mut self, ch: char, events: &mut Vec<StructuredProgressEvent>) {
        if !self.started_json {
            if ch == '{' {
                self.started_json = true;
                let track_root_object = self.spec.root_array_key.is_empty();
                if track_root_object {
                    self.item_count = 1;
                    self.current_item = Some(ItemProgress {
                        index: 1,
                        ..ItemProgress::default()
                    });
                }
                self.push_container(Container::object(track_root_object, None));
            }
            return;
        }

        if self.in_string {
            if self.escaping {
                self.string_buffer.push(ch);
                self.escaping = false;
                return;
            }
            match ch {
                '\\' => {
                    self.escaping = true;
                }
                '"' => {
                    self.in_string = false;
                    let value = std::mem::take(&mut self.string_buffer);
                    self.handle_string_complete(value, events);
                }
                _ => self.string_buffer.push(ch),
            }
            return;
        }

        match ch {
            '"' => {
                self.in_string = true;
                self.string_buffer.clear();
            }
            '{' => {
                let is_root_item = self.stack.last().is_some_and(|container| {
                    container.kind == ContainerKind::Array && container.is_root_array
                });
                if is_root_item {
                    self.item_count += 1;
                    self.current_item = Some(ItemProgress {
                        index: self.item_count,
                        ..ItemProgress::default()
                    });
                }
                let field_key = self.take_direct_root_item_pending_key();
                self.push_container(Container::object(is_root_item, field_key));
            }
            '[' => {
                let is_root_array = self
                    .stack
                    .last()
                    .and_then(|container| container.pending_key.as_deref())
                    == Some(self.spec.root_array_key);
                let field_key = if is_root_array {
                    self.clear_parent_pending_key();
                    None
                } else {
                    self.take_direct_root_item_pending_key()
                };
                self.push_container(Container::array(is_root_array, field_key));
            }
            '}' => self.pop_container(ContainerKind::Object, events),
            ']' => self.pop_container(ContainerKind::Array, events),
            ',' => {
                if let Some(container) = self.stack.last_mut()
                    && container.kind == ContainerKind::Object
                {
                    container.expecting_key = true;
                    container.pending_key = None;
                }
            }
            ':' => {}
            c if c.is_whitespace() => {}
            _ => {}
        }
    }

    fn push_container(&mut self, container: Container) {
        self.stack.push(container);
    }

    fn pop_container(
        &mut self,
        expected: ContainerKind,
        events: &mut Vec<StructuredProgressEvent>,
    ) {
        let Some(container) = self.stack.pop() else {
            return;
        };
        if container.kind != expected {
            return;
        }

        if let Some(field_key) = container.field_key_for_root_item {
            self.complete_field(&field_key, events);
        }
        if container.is_root_item {
            self.complete_item(events);
        }
        if let Some(parent) = self.stack.last_mut()
            && parent.kind == ContainerKind::Object
        {
            parent.expecting_key = false;
            parent.pending_key = None;
        }
    }

    fn handle_string_complete(&mut self, value: String, events: &mut Vec<StructuredProgressEvent>) {
        let Some(container) = self.stack.last_mut() else {
            return;
        };
        if container.kind != ContainerKind::Object {
            return;
        }
        if container.expecting_key {
            container.pending_key = Some(value);
            container.expecting_key = false;
            return;
        }
        let key = container.pending_key.take();
        container.expecting_key = false;
        if container.is_root_item
            && let Some(key) = key
        {
            self.complete_field(&key, events);
        }
    }

    fn take_direct_root_item_pending_key(&mut self) -> Option<String> {
        let container = self.stack.last_mut()?;
        if container.kind == ContainerKind::Object && container.is_root_item {
            container.expecting_key = false;
            return container.pending_key.take();
        }
        None
    }

    fn clear_parent_pending_key(&mut self) {
        if let Some(container) = self.stack.last_mut()
            && container.kind == ContainerKind::Object
        {
            container.expecting_key = false;
            container.pending_key = None;
        }
    }

    fn complete_field(&mut self, key: &str, events: &mut Vec<StructuredProgressEvent>) {
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
            item.completed_units = item.completed_units.saturating_add(weight);
            item.index
        };
        self.completed_units = self.completed_units.saturating_add(weight);
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

    fn complete_item(&mut self, events: &mut Vec<StructuredProgressEvent>) {
        let item_index = {
            let Some(item) = self.current_item.as_mut() else {
                return;
            };
            if item.completed {
                return;
            }
            item.completed = true;
            item.completed_units = item.completed_units.saturating_add(1);
            item.index
        };
        self.completed_units = self.completed_units.saturating_add(1);
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
            let total_units = self.unit_per_item.saturating_mul(expected_items as u32);
            self.span
                .current_for_units(self.completed_units, total_units)
        } else {
            self.current_progress_without_known_total()
        };
        self.max_reported_current = self.max_reported_current.max(current);
        self.max_reported_current
    }

    fn current_progress_without_known_total(&self) -> u32 {
        let width = self.span.end().saturating_sub(self.span.start());
        if width == 0 {
            return self.span.start();
        }
        let reserve_units = self.unit_per_item.max(1);
        let denominator = self.completed_units.saturating_add(reserve_units).max(1);
        self.span.start() + width.saturating_mul(self.completed_units) / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_MILESTONES: &[FieldMilestone] = &[
        FieldMilestone::new("title", "标题", 1),
        FieldMilestone::new("scene_summary", "剧情状态", 1),
        FieldMilestone::new("frame_prompt", "画面提示词", 1),
        FieldMilestone::new("blocking_state", "调度状态", 1),
        FieldMilestone::new("lighting_state", "光影状态", 1),
        FieldMilestone::new("negative_prompt", "负向提示词", 1),
    ];

    fn frame_spec(expected_items: Option<usize>) -> StructuredJsonProgressSpec {
        StructuredJsonProgressSpec {
            root_array_key: "frames",
            item_label: "边界帧",
            expected_items,
            milestones: FRAME_MILESTONES,
        }
    }

    #[test]
    fn tracks_split_json_fields_inside_root_items() {
        let mut tracker =
            StructuredJsonProgressTracker::new(frame_spec(Some(1)), ProgressSpan::new(20, 80));
        let mut events = Vec::new();
        for chunk in [
            r#"```json
{"frames":[{"#,
            r#""frame_number":1,"title":"开场","#,
            r#""scene_summary":"{不是对象}","#,
            r#""frame_prompt":"带有 \"引号\" 的提示","#,
            r#""blocking_state":{"characters":[]},"#,
            r#""lighting_state":{"time":"夜"},"#,
            r#""negative_prompt":"避免模糊"}]}"#,
        ] {
            events.extend(tracker.consume(chunk));
        }

        let messages = events
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages
                .iter()
                .any(|message| message.contains("标题已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("剧情状态已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("画面提示词已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("调度状态已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("光影状态已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("负向提示词已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("边界帧已完成"))
        );
        assert!(
            events
                .windows(2)
                .all(|pair| pair[0].current <= pair[1].current)
        );
    }

    #[test]
    fn ignores_matching_keys_inside_nested_objects() {
        let mut tracker =
            StructuredJsonProgressTracker::new(frame_spec(Some(1)), ProgressSpan::new(0, 100));
        let events = tracker.consume(
            r#"{"frames":[{"title":"外层","blocking_state":{"title":"内层"},"negative_prompt":"避免模糊"}]}"#,
        );
        let title_events = events
            .iter()
            .filter(|event| event.message.contains("标题已完成"))
            .count();
        assert_eq!(title_events, 1);
    }

    #[test]
    fn omits_moving_denominator_when_expected_item_count_is_unknown() {
        let mut tracker =
            StructuredJsonProgressTracker::new(frame_spec(None), ProgressSpan::new(20, 80));
        let events = tracker.consume(
            r#"{"frames":[{"title":"一","scene_summary":"一"},{"title":"二","scene_summary":"二"}]}"#,
        );
        let messages = events
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages
                .iter()
                .any(|message| message.contains("第 1 个边界帧：标题已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("第 2 个边界帧：标题已完成"))
        );
        assert!(
            messages
                .iter()
                .all(|message| !message.contains("/2") && !message.contains("/3"))
        );
        assert!(
            events
                .windows(2)
                .all(|pair| pair[0].current <= pair[1].current)
        );
    }

    #[test]
    fn tracks_root_object_fields_when_root_array_key_is_empty() {
        const ROOT_FIELDS: &[FieldMilestone] = &[
            FieldMilestone::new("metadata_additions", "元数据补充", 1),
            FieldMilestone::new("summaries", "摘要", 1),
        ];
        let mut tracker = StructuredJsonProgressTracker::new(
            StructuredJsonProgressSpec {
                root_array_key: "",
                item_label: "章节摘要阶段",
                expected_items: None,
                milestones: ROOT_FIELDS,
            },
            ProgressSpan::new(20, 40),
        );
        let events = tracker.consume(
            r#"{"metadata_additions":{"tags":["转折"],"tone":"紧张"},"summaries":{"one_line":"发现线索"}}"#,
        );
        let messages = events
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages
                .iter()
                .any(|message| message.contains("元数据补充已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("摘要已完成"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("章节摘要阶段已完成"))
        );
    }
}
