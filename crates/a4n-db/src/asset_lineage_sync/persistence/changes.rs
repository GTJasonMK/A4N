use sea_orm::Set;
use serde_json::Value;

pub(super) fn set_if_changed<T>(
    field: &mut sea_orm::ActiveValue<T>,
    current: T,
    next: T,
    changed: &mut bool,
) where
    T: PartialEq + Into<sea_orm::Value>,
{
    if current != next {
        *field = Set(next);
        *changed = true;
    }
}

pub(super) fn set_string_if_changed(
    field: &mut sea_orm::ActiveValue<String>,
    current: &str,
    next: String,
    changed: &mut bool,
) {
    if current != next {
        *field = Set(next);
        *changed = true;
    }
}

pub(super) fn set_value_if_changed(
    field: &mut sea_orm::ActiveValue<Value>,
    current: &Value,
    next: Value,
    changed: &mut bool,
) {
    if current != &next {
        *field = Set(next);
        *changed = true;
    }
}
