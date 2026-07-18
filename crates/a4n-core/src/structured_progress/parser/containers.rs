use crate::structured_progress::StructuredProgressEvent;

use super::state::{Container, ContainerKind, StructuredJsonProgressTracker};

impl StructuredJsonProgressTracker {
    pub(super) fn push_container(&mut self, container: Container) {
        self.stack.push(container);
    }

    pub(super) fn pop_container(
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

    pub(super) fn handle_string_complete(
        &mut self,
        value: String,
        events: &mut Vec<StructuredProgressEvent>,
    ) {
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

    pub(super) fn take_direct_root_item_pending_key(&mut self) -> Option<String> {
        let container = self.stack.last_mut()?;
        if container.kind == ContainerKind::Object && container.is_root_item {
            container.expecting_key = false;
            return container.pending_key.take();
        }
        None
    }

    pub(super) fn clear_parent_pending_key(&mut self) {
        if let Some(container) = self.stack.last_mut()
            && container.kind == ContainerKind::Object
        {
            container.expecting_key = false;
            container.pending_key = None;
        }
    }
}
