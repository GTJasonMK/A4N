use crate::StructuredProgressEvent;

use super::state::{Container, ContainerKind, ItemProgress, StructuredJsonProgressTracker};

impl StructuredJsonProgressTracker {
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
}
