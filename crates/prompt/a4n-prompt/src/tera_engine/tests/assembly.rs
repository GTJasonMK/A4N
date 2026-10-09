use std::collections::HashMap;

use a4n_prompt_contract::{
    PromptAssemblyBlock, PromptAssemblyBlockKind, PromptAssemblyConfig, PromptAssemblySnapshot,
    PromptAssemblySource,
};
use serde_json::json;

use super::support::make_entry;
use crate::tera_engine::TeraPromptEngine;

#[test]
fn renders_enabled_template_custom_and_variable_blocks_in_order() {
    let engine = TeraPromptEngine::with_defaults(vec![
        make_entry("base", "Base {{ topic }}"),
        make_entry("detail", "Detail {{ topic }}"),
    ])
    .unwrap();
    let snapshot = PromptAssemblySnapshot {
        call_key: "test.call".into(),
        prompt_name: "base".into(),
        config: PromptAssemblyConfig {
            blocks: vec![
                PromptAssemblyBlock {
                    id: "base".into(),
                    kind: PromptAssemblyBlockKind::Template,
                    value: "base".into(),
                    enabled: true,
                },
                PromptAssemblyBlock {
                    id: "disabled".into(),
                    kind: PromptAssemblyBlockKind::Template,
                    value: "detail".into(),
                    enabled: false,
                },
                PromptAssemblyBlock {
                    id: "custom".into(),
                    kind: PromptAssemblyBlockKind::Custom,
                    value: "Extra {{ topic }}".into(),
                    enabled: true,
                },
                PromptAssemblyBlock {
                    id: "variable".into(),
                    kind: PromptAssemblyBlockKind::Variable,
                    value: "topic".into(),
                    enabled: true,
                },
            ],
        },
        source: PromptAssemblySource::Global,
    };
    let variables = HashMap::from([("topic".to_string(), json!("arrival"))]);

    let rendered = engine
        .render_assembled(&snapshot, &variables, None)
        .expect("valid assembly should render");

    assert_eq!(rendered, "Base arrival\n\nExtra arrival\n\narrival");
}

#[test]
fn rejects_unavailable_variables_and_invalid_custom_templates() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("base", "Base {{ topic }}")]).unwrap();
    let variables = HashMap::new();
    let snapshot = |value: &str| PromptAssemblySnapshot {
        call_key: "test.call".into(),
        prompt_name: "base".into(),
        config: PromptAssemblyConfig {
            blocks: vec![PromptAssemblyBlock {
                id: "custom".into(),
                kind: PromptAssemblyBlockKind::Custom,
                value: value.into(),
                enabled: true,
            }],
        },
        source: PromptAssemblySource::SystemDefault,
    };

    assert!(
        engine
            .render_assembled(&snapshot("{{ missing }}"), &variables, None)
            .is_err()
    );
    assert!(
        engine
            .render_assembled(&snapshot("{{"), &variables, None)
            .is_err()
    );
}
