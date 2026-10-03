use a4n_db::fact_feedback::{FeedbackSource, current_feedback_sources, feedback_config};
use a4n_domain::fact_feedback::{
    FactAssertion, FactSourceRevision, pending_fact_sources, world_setting_document,
};
use a4n_domain::source_spans::source_spans_for_chapters;
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft,
    context_aware_system_user_request,
};
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{FACT_CONSOLIDATION_KIND, FACT_CONSOLIDATION_PROMPT, feedback_error};
use crate::{OrchestratorContext, source_markers::SourceMarkerMap};

pub(super) struct PreparedConsolidation {
    pub sources: Vec<FactSourceRevision>,
    pub expected_blueprint_version: Option<String>,
    pub request: ChatRequest,
    pub fingerprint: Value,
    pub source_markers: SourceMarkerMap,
}

impl OrchestratorContext {
    pub(super) async fn prepare_consolidation(
        &self,
        project_id: i64,
        force: bool,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Option<PreparedConsolidation>, AppError> {
        let db = self.database.connection();
        let (source_result, config_result, rows_result) = tokio::join!(
            current_feedback_sources(db, project_id),
            feedback_config(db, project_id),
            a4n_db::outline::load_outline_generation_source_rows(db, project_id),
        );
        let sources = source_result.map_err(feedback_error)?;
        let config = config_result?;
        let rows = rows_result?.ok_or_else(|| AppError::NotFound("项目不存在".into()))?;
        if rows.project.lifecycle_gate != a4n_db::generation::PROJECT_GATE_ACTIVE
            || rows.project.project_kind != "text"
        {
            return Err(AppError::InvalidArgument("当前项目不可归纳正文事实".into()));
        }
        if sources.is_empty() {
            return Ok(None);
        }
        let revisions: Vec<_> = sources
            .iter()
            .map(|source| source.revision.clone())
            .collect();
        let consolidated: Vec<FactSourceRevision> =
            serde_json::from_value(config.consolidated_sources_json)?;
        let pending = pending_fact_sources(&revisions, &consolidated);
        let pending_ids: BTreeSet<_> = pending.iter().map(|source| source.chapter_id).collect();
        let removed = consolidated.iter().any(|source| {
            !revisions
                .iter()
                .any(|current| current.chapter_id == source.chapter_id)
        });
        let part_end = rows.parts.iter().any(|part| {
            pending
                .iter()
                .any(|source| source.chapter_number == part.end_chapter)
        });
        let urgent = contains_new_constraint(&sources, &pending_ids);
        if !force
            && pending.len() < config.interval_chapters as usize
            && !removed
            && !part_end
            && !urgent
        {
            return Ok(None);
        }
        let selected: Vec<_> = if pending_ids.is_empty() {
            sources.iter().collect()
        } else {
            sources
                .iter()
                .filter(|source| pending_ids.contains(&source.chapter.id))
                .collect()
        };
        let spans_by_chapter = selected
            .iter()
            .map(|source| source_spans_for_chapters(std::slice::from_ref(&source.chapter)))
            .collect::<Vec<_>>();
        let all_spans = spans_by_chapter
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let source_markers = SourceMarkerMap::from_source_spans(&all_spans)?;
        let chapters = selected
            .iter()
            .zip(&spans_by_chapter)
            .map(|(source, spans)| {
                Ok(json!({
                    "id": source.chapter.id, "chapter_number": source.chapter.chapter_number,
                    "title": source.chapter.title,
                    "content": source_markers.mark_text(&source.chapter.content, spans)?,
                    "world_facts": source.world_facts,
                }))
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let source_data = source_markers.encode_input(&json!({
            "chapters": chapters,
        }))?;
        let blueprint_data = rows.blueprint.as_ref().map(|blueprint| {
            json!({
                "id": blueprint.id, "version": blueprint.current_version,
                "full_synopsis": blueprint.full_synopsis,
                "world_setting": world_setting_document(blueprint.world_setting.as_ref()),
            })
        });
        let material_data = json!({
            "blueprint": blueprint_data,
            "characters": rows.character_profiles.iter().map(|profile| json!({
                "id":profile.id,"name":profile.name,"current_state":profile.current_state,
                "voice_fingerprint":profile.voice_fingerprint,"character_arc":profile.character_arc,
            })).collect::<Vec<_>>(),
            "part_outlines":rows.parts.iter().map(|part| json!({
                "part_number": part.part_number, "start_chapter": part.start_chapter,
                "end_chapter": part.end_chapter, "summary": part.summary,
                "character_focus": part.character_focus, "key_events": part.key_events,
            })).collect::<Vec<_>>(),
            "planning_rule":"未来剧情和人物终点只形成 planning 建议",
        });
        let mut ledger = LlmContextLedger::new();
        for (kind, value) in [
            ("current_materials", material_data),
            ("body_fact_sources", source_data),
        ] {
            ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
                scope_kind: "project".into(),
                pack_kind: kind.into(),
                source_refs_json: Some(json!({"project_id":project_id,"sources":revisions})),
                config_identity_json: json!({"version":1,"kind":kind}),
                required: true,
                stable_prefix: true,
                variants: vec![LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, value)],
            })?;
        }
        let system = prompt_renderer
            .render_prompt(FACT_CONSOLIDATION_PROMPT, &HashMap::new(), Some(project_id))
            .await?;
        let request = context_aware_system_user_request(
            system,
            ledger.into_manifest(
                "按正文证据归纳资料变化，只返回 {\"changes\": [...]}。不需要变化时返回空数组。",
                Some("正文事实优先；不能把传闻、假设、回忆或未来剧情当作当前事实。".into()),
            ),
            Some(0.2),
            None,
        )?
        .with_json_response_format()
        .with_trace_context(LlmTraceContext {
            project_id: Some(project_id),
            root_project_id: None,
            workflow: FACT_CONSOLIDATION_KIND.into(),
            step_key: FACT_CONSOLIDATION_PROMPT.into(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(FACT_CONSOLIDATION_PROMPT.into()),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });
        let expected_blueprint_version = rows.blueprint.map(|blueprint| blueprint.current_version);
        let fingerprint = json!({"schema_version":1,"model":self.llm_model_name(),"sources":revisions,"blueprint_version":expected_blueprint_version,"request":request.snapshot_value()?});
        Ok(Some(PreparedConsolidation {
            sources: revisions,
            expected_blueprint_version,
            request,
            fingerprint,
            source_markers,
        }))
    }
}

fn contains_new_constraint(sources: &[FeedbackSource], pending: &BTreeSet<i64>) -> bool {
    let mut known = BTreeMap::new();
    for source in sources
        .iter()
        .filter(|source| !pending.contains(&source.chapter.id))
    {
        for fact in source
            .world_facts
            .iter()
            .filter(|fact| fact.assertion == FactAssertion::Observed)
        {
            known.insert(fact.identity(), fact.statement.as_str());
        }
    }
    sources
        .iter()
        .filter(|source| pending.contains(&source.chapter.id))
        .any(|source| {
            source
                .world_facts
                .iter()
                .filter(|fact| fact.assertion == FactAssertion::Observed)
                .any(|fact| {
                    let previous = known.get(&fact.identity()).copied();
                    previous != Some(fact.statement.as_str())
                        && (previous.is_some() || fact.field.requires_immediate_review())
                })
        })
}
