use super::types::{CanvasNodeContract, CanvasPortContract};

pub(super) fn contract(
    kind: &str,
    label: &str,
    category: &str,
    runnable: bool,
    inputs: Vec<CanvasPortContract>,
    outputs: Vec<CanvasPortContract>,
) -> CanvasNodeContract {
    CanvasNodeContract {
        kind: kind.to_string(),
        label: label.to_string(),
        category: category.to_string(),
        runnable,
        inputs,
        outputs,
    }
}

pub(super) fn input(
    id: &str,
    label: &str,
    data_type: &str,
    required: bool,
    multiple: bool,
) -> CanvasPortContract {
    port(id, label, data_type, "input", required, multiple)
}

pub(super) fn out(id: &str, label: &str, data_type: &str) -> CanvasPortContract {
    port(id, label, data_type, "output", false, true)
}

fn port(
    id: &str,
    label: &str,
    data_type: &str,
    direction: &str,
    required: bool,
    multiple: bool,
) -> CanvasPortContract {
    CanvasPortContract {
        id: id.to_string(),
        label: label.to_string(),
        data_type: data_type.to_string(),
        direction: direction.to_string(),
        required,
        multiple,
    }
}
