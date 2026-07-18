const TEST_WORKFLOW_RUN_POOL_SIZE: i64 = 64;

fn next_test_workflow_run_id(llm_switch: &SwitchableLlmProvider, chapter_id: i64) -> i64 {
    static NEXT_SLOT_BY_SWITCH: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<usize, i64>>,
    > = std::sync::OnceLock::new();
    let switch_key = std::ptr::from_ref(llm_switch).addr();
    let mut next_slot_by_switch = NEXT_SLOT_BY_SWITCH
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
        .expect("video test workflow slot lock should not be poisoned");
    let next_slot = next_slot_by_switch.entry(switch_key).or_insert(1);
    let slot = *next_slot;
    *next_slot = if slot == TEST_WORKFLOW_RUN_POOL_SIZE {
        1
    } else {
        slot + 1
    };
    chapter_id * 100 + slot
}

fn generation_source() -> ChapterVideoSourceContext<'static> {
    ChapterVideoSourceContext {
        content: "小宦官推门入殿，忽然察觉暗处有人，回头压住恐惧。",
        summary: Some("小宦官夜入宫殿并察觉危险。"),
        analysis: None,
    }
}

fn empty_prompt_variables() -> &'static HashMap<String, Value> {
    static EMPTY: OnceLock<HashMap<String, Value>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn prompt_stage_input<'a, L>(
    llm_switch: &SwitchableLlmProvider,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &'static str,
    llm: &L,
    prompt_renderer: &'a dyn a4n_prompt::rendering::PromptRenderer,
    mut options: ChapterVideoPromptOptions,
    cancel_token: Option<&'a CancellationToken>,
) -> ChapterVideoPromptStageInput<'a, 'static>
where
    L: LlmProvider + Clone + 'static,
{
    llm_switch.set(Arc::new(llm.clone()));
    if options.workflow_run_id.is_none() {
        options.workflow_run_id = Some(next_test_workflow_run_id(llm_switch, chapter_id));
    }
    ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode,
        source: generation_source(),
        prompt_renderer,
        variables: empty_prompt_variables(),
        options,
        cancel_token,
    }
}


