use super::{
    delete_foreshadowing_rag_by_ids, ingest_foreshadowing_rag, refresh_project_current_chapter,
    sync_chapter_rag_records, sync_project_status_from_artifacts,
};
use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::actions::workflow_checkpoint::{
    load_completed_workflow_checkpoint, required_checkpoint_field, required_checkpoint_json,
    save_completed_workflow_checkpoint,
};
use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_core::models::{
    chapter::chapter, character::foreshadowing_index, generation::workflow_run,
};
use a4n_services::chapter::analysis::{
    ChapterAnalysisService, ForeshadowingEntry, foreshadowing_entries, priority_to_importance,
    validate_saved_chapter_analysis,
};
use a4n_services::source::source_content_sha256;
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use tokio_util::sync::CancellationToken;

pub const CHAPTER_ANALYSIS_PROGRESS_TOTAL: u32 = 100;
const CHAPTER_ANALYSIS_STEP_FACTS: &str = "chapter_analysis.facts";
const CHAPTER_ANALYSIS_STEP_DERIVED_SYNC: &str = "chapter_analysis.derived_sync";
const CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS: &str = "chapter_analysis.sync_outputs";
const CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY: &str = "chapter_analysis.object_discovery";
const CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY: &str = "chapter_analysis.character_discovery";
const CHAPTER_ANALYSIS_STEP_SCENE_SYNC: &str = "chapter_analysis.scene_sync";
const CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC: &str = "chapter_analysis.character_sync";
const CHAPTER_ANALYSIS_STEP_REFRESH_STATUS: &str = "chapter_analysis.refresh_status";
pub type ChapterAnalysisProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterAnalysisInput {
    pub chapter_id: i64,
}

pub struct ChapterAnalysisProgress<'a> {
    reporter: Option<ChapterAnalysisProgressReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ChapterAnalysisProgress<'a> {
    pub fn new(
        reporter: Option<ChapterAnalysisProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    fn report(
        &self,
        current: u32,
        total: u32,
        message: &str,
        cancelled_message: &str,
    ) -> Result<(), String> {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(cancelled_message.to_string())
        } else {
            Ok(())
        }
    }
}

pub async fn preflight_generate_chapter_analysis(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    preflight_chapter_analysis(
        state,
        user_id,
        chapter_id,
        "生成结构化分析",
        "章节正文为空，不能生成结构化分析",
        false,
    )
    .await
}

pub async fn preflight_resume_chapter_analysis_sync(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    preflight_chapter_analysis(
        state,
        user_id,
        chapter_id,
        "继续同步结构化分析",
        "章节正文为空，不能继续同步结构化分析",
        true,
    )
    .await
}

pub async fn generate_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    report_chapter_analysis_progress(
        progress,
        0,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在检查章节正文",
        "章节结构化分析已停止",
    )?;
    let ch = preflight_generate_chapter_analysis(state, user_id, chapter_id).await?;

    report_chapter_analysis_progress(
        progress,
        5,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在清理旧的章节派生数据",
        "章节结构化分析已停止",
    )?;
    super::postprocess_chapter_after_content_change(state, ch.id)
        .await
        .map_err(|err| err.to_string())?;

    generate_and_sync_chapter_analysis_with_progress(state, user_id, ch.id, progress).await
}

pub async fn resume_chapter_analysis_sync_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    report_chapter_analysis_progress(
        progress,
        0,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在检查已保存的章节分析",
        "章节分析继续同步已停止",
    )?;
    let ch = preflight_resume_chapter_analysis_sync(state, user_id, chapter_id).await?;
    let workflow_run = state
        .workflow_run
        .upsert_run(chapter_analysis_workflow_run_input(
            &ch,
            CHAPTER_ANALYSIS_STEP_DERIVED_SYNC,
        ))
        .await
        .map_err(|err| format!("创建章节分析继续同步工作流失败: {err}"))?;

    report_chapter_analysis_progress(
        progress,
        45,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "已找到结构化分析，正在继续同步派生数据",
        "章节分析继续同步已停止",
    )?;
    if let Err(err) = sync_project_state_after_chapter_analysis_with_progress(
        state,
        user_id,
        &ch,
        Some(workflow_run.id),
        progress,
    )
    .await
    {
        state
            .workflow_run
            .mark_status(workflow_run.id, WORKFLOW_STATUS_FAILED, true, Some(&err))
            .await
            .map_err(|status_err| {
                format!("{err}; 同步章节分析继续同步工作流状态失败: {status_err}")
            })?;
        return Err(err);
    }
    state
        .workflow_run
        .mark_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await
        .map_err(|err| format!("同步章节分析继续同步完成状态失败: {err}"))?;

    chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| format!("继续同步后刷新章节失败: {err}"))?
        .ok_or_else(|| format!("继续同步后刷新章节失败：章节不存在: chapter_id={chapter_id}"))
}

pub async fn resume_chapter_analysis_workflow_run(
    state: &AppState,
    user_id: i64,
    run: &workflow_run::Model,
) -> Result<(), String> {
    if run.workflow_kind != "chapter_analysis" {
        return Err(format!(
            "工作流类型 {} 不是章节结构化分析",
            run.workflow_kind
        ));
    }
    if !run.can_resume {
        return Err(run
            .resume_block_reason
            .as_deref()
            .unwrap_or("该章节结构化分析工作流当前不可恢复")
            .to_string());
    }

    let chapter_id = run
        .chapter_id
        .ok_or_else(|| "章节结构化分析工作流缺少 chapter_id".to_string())?;
    let ch = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| format!("恢复章节结构化分析失败：查询章节失败: {err}"))?
        .ok_or_else(|| format!("恢复章节结构化分析失败：章节不存在: chapter_id={chapter_id}"))?;
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    ensure_chapter_not_generating(&ch, "恢复结构化分析")?;
    if !chapter_analysis_run_input_matches(&run.input_fingerprint_json, &ch) {
        return Err(
            "章节正文、标题或章节归属已变化，不能从旧的章节结构化分析断点恢复；请重新生成结构化分析"
                .to_string(),
        );
    }

    if ch.analysis_data.is_some() {
        resume_chapter_analysis_sync_with_progress(state, user_id, ch.id, None).await?;
    } else {
        generate_and_sync_chapter_analysis_with_progress(state, user_id, ch.id, None).await?;
    }
    Ok(())
}

pub async fn preview_chapter_analysis_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    let ch = preflight_chapter_analysis(
        state,
        user_id,
        chapter_id,
        "预览结构化分析提示词",
        "章节正文为空，不能预览结构化分析提示词",
        false,
    )
    .await?;

    let analyzer = ChapterAnalysisService::new(state.db.clone());
    let analysis_steps = analyzer
        .build_analysis_requests(chapter_id, state.engine.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let mut steps = analysis_steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                Some(step.prompt_name),
                "ready",
                None,
                Some(step.request),
            )
        })
        .collect::<Vec<_>>();
    steps.push(blocked_chapter_analysis_step(
        "object_discovery",
        "分析后物体档案发现请求",
        Some("object_discovery"),
        "该请求依赖结构化分析保存后的章节数据，发送前无法提前构造真实请求",
    ));
    steps.extend(character_discovery_chapter_analysis_steps(
        crate::actions::character_ai::character_discovery_preview_steps(
            state,
            user_id,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some("结构化分析保存后会继续执行该请求".to_string()),
        )
        .await?,
    ));
    steps.extend(blocked_scene_profile_sync_chapter_analysis_steps(
        ch.chapter_number,
        "该请求依赖结构化分析保存后的章节数据，发送前无法提前构造真实请求",
    ));
    steps.push(blocked_chapter_analysis_step(
        "analysis_character_sync",
        "分析后角色档案同步请求",
        Some("character_sync"),
        "该请求依赖结构化分析输出和角色发现模型返回结果，发送前无法提前构造真实请求",
    ));
    chat_status_preview_for_state(
        state,
        "chapter_analysis",
        format!("第 {} 章结构化分析请求预览", ch.chapter_number),
        steps,
        "llm_chat",
    )
    .await
}

pub async fn preview_resume_chapter_analysis_sync_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    let ch = preflight_resume_chapter_analysis_sync(state, user_id, chapter_id).await?;

    let mut steps = object_discovery_chapter_analysis_steps(
        crate::actions::object_discovery::object_discovery_preview_steps(
            state,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some("继续同步会先重新执行物体档案发现".to_string()),
        )
        .await?,
    );
    steps.extend(character_discovery_chapter_analysis_steps(
        crate::actions::character_ai::character_discovery_preview_steps(
            state,
            user_id,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some("继续同步会先重新执行角色发现".to_string()),
        )
        .await?,
    ));
    steps.extend(scene_profile_sync_chapter_analysis_steps(
        crate::actions::scene_profile_sync::scene_profile_sync_preview_steps(
            state,
            user_id,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some(
                "继续同步会先执行场景档案候选识别；后续场景同步请求依赖该阶段返回结果".to_string(),
            ),
        )
        .await?,
    ));
    steps.push(blocked_chapter_analysis_step(
        "analysis_character_sync",
        "分析后角色档案同步请求",
        Some("character_sync"),
        "该请求依赖角色发现模型返回结果；当前只能确认会在发现结果返回后构造",
    ));
    chat_status_preview_for_state(
        state,
        "chapter_analysis_resume_sync",
        format!("第 {} 章分析继续同步请求预览", ch.chapter_number),
        steps,
        "llm_chat",
    )
    .await
}

pub async fn generate_and_sync_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    report_chapter_analysis_progress(
        progress,
        10,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在读取章节并校验角色同步前置条件",
        "章节结构化分析已停止",
    )?;
    let ch = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| format!("章节结构化分析失败：查询章节失败: {err}"))?
        .ok_or_else(|| format!("章节结构化分析失败：章节不存在: chapter_id={chapter_id}"))?;
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    ensure_previous_chapter_analysis_complete(state, &ch, "生成结构化分析").await?;
    crate::actions::character_ai::preflight_analysis_character_profile_sync_to_chapter(
        state,
        ch.project_id,
        ch.chapter_number,
    )
    .await?;
    let workflow_run = state
        .workflow_run
        .upsert_run(chapter_analysis_workflow_run_input(
            &ch,
            CHAPTER_ANALYSIS_STEP_FACTS,
        ))
        .await
        .map_err(|err| format!("创建章节分析工作流失败: {err}"))?;

    report_chapter_analysis_progress(
        progress,
        20,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在调用模型生成结构化分析",
        "章节结构化分析已停止",
    )?;
    let analyzer = ChapterAnalysisService::new(state.db.clone());
    let analysis_result = if let Some(progress_state) = progress {
        let cancel_token = progress_state.cancel_token;
        let mut analysis_progress = |current: u32, total: u32, message: &str| {
            report_chapter_subprogress(progress, current, total, 20, 45, message);
            Ok::<(), AppError>(())
        };
        analyzer
            .analyze_and_store_with_progress_and_workflow(
                chapter_id,
                state.llm.as_ref(),
                state.engine.as_ref(),
                Some(&mut analysis_progress),
                cancel_token,
                Some(workflow_run.id),
            )
            .await
    } else {
        analyzer
            .analyze_and_store_with_progress_and_workflow(
                chapter_id,
                state.llm.as_ref(),
                state.engine.as_ref(),
                None,
                None,
                Some(workflow_run.id),
            )
            .await
    };
    if let Err(err) = analysis_result {
        let err_text = format!("章节结构化分析失败: {err}");
        state
            .workflow_run
            .mark_status(
                workflow_run.id,
                WORKFLOW_STATUS_FAILED,
                true,
                Some(&err_text),
            )
            .await
            .map_err(|status_err| {
                format!("{err_text}; 同步章节分析工作流状态失败: {status_err}")
            })?;
        return Err(err_text);
    }

    report_chapter_analysis_progress(
        progress,
        45,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "结构化分析已保存，正在读取章节",
        "章节结构化分析已停止",
    )?;
    let updated_chapter = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| format!("章节结构化分析失败：查询章节失败: {err}"))?
        .ok_or_else(|| format!("章节结构化分析失败：章节不存在: chapter_id={chapter_id}"))?;

    if let Err(err) = sync_project_state_after_chapter_analysis_with_progress(
        state,
        user_id,
        &updated_chapter,
        Some(workflow_run.id),
        progress,
    )
    .await
    {
        state
            .workflow_run
            .mark_status(workflow_run.id, WORKFLOW_STATUS_FAILED, true, Some(&err))
            .await
            .map_err(|status_err| format!("{err}; 同步章节分析工作流状态失败: {status_err}"))?;
        return Err(err);
    }
    state
        .workflow_run
        .mark_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await
        .map_err(|err| format!("同步章节分析工作流完成状态失败: {err}"))?;
    Ok(updated_chapter)
}

fn chapter_analysis_workflow_run_input(
    ch: &chapter::Model,
    current_step_key: &str,
) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: chapter_analysis_workflow_business_key(ch.project_id, ch.id),
        workflow_kind: "chapter_analysis".to_string(),
        project_id: Some(ch.project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter".to_string()),
        scope_id: Some(ch.id),
        chapter_id: Some(ch.id),
        mode: None,
        input_fingerprint_json: Some(chapter_analysis_workflow_fingerprint(ch)),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(current_step_key.to_string()),
        active_task_id: None,
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    }
}

fn chapter_analysis_workflow_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("chapter_analysis:{project_id}:{chapter_id}")
}

fn chapter_analysis_workflow_fingerprint(ch: &chapter::Model) -> serde_json::Value {
    json!({
        "schema_version": 1,
        "workflow_kind": "chapter_analysis",
        "project_id": ch.project_id,
        "chapter_id": ch.id,
        "chapter_number": ch.chapter_number,
        "content_sha256": source_content_sha256(&ch.content),
        "title": ch.title,
    })
}

pub(crate) fn chapter_analysis_run_input_matches(
    run_fingerprint_json: &Option<Value>,
    ch: &chapter::Model,
) -> bool {
    run_fingerprint_json
        .as_ref()
        .is_some_and(|fingerprint| *fingerprint == chapter_analysis_workflow_fingerprint(ch))
}

pub async fn ensure_previous_chapter_analysis_complete(
    state: &AppState,
    ch: &chapter::Model,
    operation: &str,
) -> Result<(), String> {
    let previous = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(ch.project_id))
        .filter(chapter::Column::ChapterNumber.lt(ch.chapter_number))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("{operation}前序章节分析顺序校验失败：查询章节失败: {err}"))?;
    ensure_previous_chapter_analysis_complete_from_rows(operation, ch.chapter_number, &previous)
}

fn ensure_previous_chapter_analysis_complete_from_rows(
    operation: &str,
    target_chapter_number: i32,
    previous: &[chapter::Model],
) -> Result<(), String> {
    let incomplete = previous
        .iter()
        .filter(|chapter| chapter.status != "deleted")
        .filter_map(previous_chapter_analysis_gap)
        .collect::<Vec<_>>();
    if incomplete.is_empty() {
        return Ok(());
    }

    let details = incomplete
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join("；");
    let omitted = incomplete.len().saturating_sub(8);
    Err(format!(
        "{operation}不能跳过前序章节：第 {target_chapter_number} 章之前还有 {count} 章未完成分析顺序校验：{details}{suffix}",
        count = incomplete.len(),
        suffix = if omitted > 0 {
            format!("；另有 {omitted} 章未列出")
        } else {
            String::new()
        }
    ))
}

fn previous_chapter_analysis_gap(chapter: &chapter::Model) -> Option<String> {
    if chapter.status == chapter::STATUS_GENERATING {
        return Some(format!("第{}章正在生成正文", chapter.chapter_number));
    }
    if !chapter.has_available_content() {
        return Some(format!("第{}章正文为空", chapter.chapter_number));
    }
    let Some(analysis) = chapter.analysis_data.as_ref() else {
        return Some(format!("第{}章未生成结构化分析", chapter.chapter_number));
    };
    if let Err(err) = validate_saved_chapter_analysis(analysis) {
        return Some(format!(
            "第{}章结构化分析数据无效：{err}",
            chapter.chapter_number
        ));
    }
    None
}

pub async fn sync_project_state_after_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    ch: &chapter::Model,
    workflow_run_id: Option<i64>,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<(), String> {
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    mark_chapter_analysis_current_step(state, workflow_run_id, CHAPTER_ANALYSIS_STEP_DERIVED_SYNC)
        .await?;
    report_chapter_analysis_progress(
        progress,
        50,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在同步章节分析、伏笔和 RAG",
        "章节结构化分析已停止",
    )?;
    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(&checkpoint, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS)?;
        report_chapter_analysis_progress(
            progress,
            58,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "章节分析输出同步已从断点恢复",
            "章节结构化分析已停止",
        )?;
    } else {
        mark_chapter_analysis_current_step(
            state,
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
        )
        .await?;
        sync_chapter_analysis_outputs(state, ch).await?;
        save_completed_chapter_analysis_phase(
            state,
            workflow_run_id,
            ch,
            CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
            10,
            json!({ "completed": true }),
        )
        .await?;
    }

    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
        )?;
        report_chapter_analysis_progress(
            progress,
            68,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "物体档案发现已从断点恢复",
            "章节结构化分析已停止",
        )?;
    } else {
        report_chapter_analysis_progress(
            progress,
            58,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "正在识别本章关键物体并同步物体档案",
            "章节结构化分析已停止",
        )?;
        mark_chapter_analysis_current_step(
            state,
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
        )
        .await?;
        let object_reporter = |current: u32, total: u32, message: &str| {
            report_chapter_subprogress(progress, current, total, 58, 68, message);
        };
        let object_progress = progress.map(|progress| {
            crate::actions::object_discovery::ObjectDiscoveryProgress::new(
                Some(&object_reporter),
                progress.cancel_token,
            )
        });
        let object_result =
            crate::actions::object_discovery::discover_object_profiles_for_chapters(
                state,
                ch.project_id,
                Some(ch.chapter_number),
                object_progress.as_ref(),
            )
            .await
            .map_err(|err| format!("分析后物体档案发现失败: {err}"))?;
        save_completed_chapter_analysis_phase(
            state,
            workflow_run_id,
            ch,
            CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
            20,
            json!({ "completed": true, "result": object_result }),
        )
        .await?;
    }

    let discovery_result = if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
        )?;
        report_chapter_analysis_progress(
            progress,
            78,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "角色发现已从断点恢复",
            "章节结构化分析已停止",
        )?;
        required_checkpoint_field(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            "result",
        )?
    } else {
        report_chapter_analysis_progress(
            progress,
            68,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "正在发现本章新出场角色",
            "章节结构化分析已停止",
        )?;
        mark_chapter_analysis_current_step(
            state,
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
        )
        .await?;
        let character_discovery_reporter = |current: u32, total: u32, message: &str| {
            report_chapter_subprogress(progress, current, total, 68, 78, message);
        };
        let character_discovery_progress = progress.map(|progress| {
            crate::actions::character_ai::CharacterAiProgress::new(
                Some(&character_discovery_reporter),
                progress.cancel_token,
            )
        });
        let discovery_result =
            crate::actions::character_ai::discover_character_profiles_for_chapters(
                state,
                user_id,
                ch.project_id,
                Some(ch.chapter_number),
                character_discovery_progress.as_ref(),
            )
            .await
            .map_err(|err| format!("分析后角色发现失败: {err}"))?;
        save_completed_chapter_analysis_phase(
            state,
            workflow_run_id,
            ch,
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            30,
            json!({ "completed": true, "result": discovery_result.clone() }),
        )
        .await?;
        discovery_result
    };

    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(&checkpoint, CHAPTER_ANALYSIS_STEP_SCENE_SYNC)?;
        report_chapter_analysis_progress(
            progress,
            88,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "场景档案同步已从断点恢复",
            "章节结构化分析已停止",
        )?;
    } else {
        report_chapter_analysis_progress(
            progress,
            78,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "正在同步本章场景档案和场景内状态",
            "章节结构化分析已停止",
        )?;
        mark_chapter_analysis_current_step(
            state,
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
        )
        .await?;
        let scene_reporter = |current: u32, total: u32, message: &str| {
            report_chapter_subprogress(progress, current, total, 78, 88, message);
        };
        let scene_progress = progress.map(|progress| {
            crate::actions::scene_profile_sync::SceneProfileSyncProgress::new(
                Some(&scene_reporter),
                progress.cancel_token,
            )
        });
        crate::actions::scene_profile_sync::sync_scene_profiles_for_chapters(
            state,
            user_id,
            ch.project_id,
            Some(ch.chapter_number),
            scene_progress.as_ref(),
        )
        .await
        .map_err(|err| format!("分析后场景档案同步失败: {err}"))?;
        save_completed_chapter_analysis_phase(
            state,
            workflow_run_id,
            ch,
            CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
            40,
            json!({ "completed": true }),
        )
        .await?;
    }

    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
        )?;
        report_chapter_analysis_progress(
            progress,
            97,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "角色档案同步已从断点恢复",
            "章节结构化分析已停止",
        )?;
    } else {
        report_chapter_analysis_progress(
            progress,
            88,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "正在同步角色档案、事件和状态索引",
            "章节结构化分析已停止",
        )?;
        mark_chapter_analysis_current_step(
            state,
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
        )
        .await?;
        let character_sync_reporter = |current: u32, total: u32, message: &str| {
            report_chapter_subprogress(progress, current, total, 88, 97, message);
        };
        let character_progress = progress.map(|progress| {
            crate::actions::character_ai::CharacterAiProgress::new(
                Some(&character_sync_reporter),
                progress.cancel_token,
            )
        });
        let sync_result =
            crate::actions::character_ai::sync_analysis_character_profiles_to_chapter(
                state,
                user_id,
                ch.project_id,
                ch.chapter_number,
                ch.analysis_data.as_ref(),
                Some(&discovery_result),
                character_progress.as_ref(),
            )
            .await
            .map_err(|err| format!("分析后角色档案同步失败: {err}"))?;
        report_analysis_character_sync_failures(&sync_result)?;
        save_completed_chapter_analysis_phase(
            state,
            workflow_run_id,
            ch,
            CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
            50,
            json!({ "completed": true, "result": sync_result }),
        )
        .await?;
    }

    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
        )?;
        report_chapter_analysis_progress(
            progress,
            99,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "项目进度状态刷新已从断点恢复",
            "章节结构化分析已停止",
        )?;
    } else {
        report_chapter_analysis_progress(
            progress,
            97,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "正在刷新项目进度状态",
            "章节结构化分析已停止",
        )?;
        mark_chapter_analysis_current_step(
            state,
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
        )
        .await?;
        refresh_project_current_chapter(state, ch.project_id)
            .await
            .map_err(|err| err.to_string())?;
        sync_project_status_from_artifacts(state, ch.project_id)
            .await
            .map_err(|err| err.to_string())?;
        save_completed_chapter_analysis_phase(
            state,
            workflow_run_id,
            ch,
            CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
            60,
            json!({ "completed": true }),
        )
        .await?;
    }
    report_chapter_analysis_progress(
        progress,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "章节分析和项目状态同步已完成",
        "章节结构化分析已停止",
    )?;
    Ok(())
}

async fn mark_chapter_analysis_current_step(
    state: &AppState,
    workflow_run_id: Option<i64>,
    step_key: &str,
) -> Result<(), String> {
    if let Some(workflow_run_id) = workflow_run_id {
        state
            .workflow_run
            .mark_current_step(workflow_run_id, step_key)
            .await
            .map_err(|err| format!("更新章节分析工作流当前步骤失败: {err}"))?;
    }
    Ok(())
}

async fn completed_chapter_analysis_phase(
    state: &AppState,
    workflow_run_id: Option<i64>,
    ch: &chapter::Model,
    step_key: &str,
) -> Result<Option<Value>, String> {
    let fingerprint = chapter_analysis_phase_fingerprint(ch, step_key);
    load_completed_workflow_checkpoint(state, workflow_run_id, step_key, &fingerprint)
        .await?
        .map(|checkpoint| required_checkpoint_json(&checkpoint, step_key))
        .transpose()
}

async fn save_completed_chapter_analysis_phase(
    state: &AppState,
    workflow_run_id: Option<i64>,
    ch: &chapter::Model,
    step_key: &str,
    step_order: i32,
    checkpoint_json: Value,
) -> Result<(), String> {
    save_completed_workflow_checkpoint(
        state,
        workflow_run_id,
        step_key,
        step_order,
        chapter_analysis_phase_fingerprint(ch, step_key),
        checkpoint_json,
    )
    .await
}

fn require_completed_chapter_analysis_phase(
    checkpoint_json: &Value,
    step_key: &str,
) -> Result<(), String> {
    if checkpoint_json
        .get("completed")
        .and_then(Value::as_bool)
        .is_some_and(|completed| completed)
    {
        Ok(())
    } else {
        Err(format!("{step_key} 已完成 checkpoint 缺少 completed=true"))
    }
}

fn chapter_analysis_phase_fingerprint(ch: &chapter::Model, step_key: &str) -> Value {
    json!({
        "schema_version": 1,
        "workflow": "chapter_analysis.derived_sync",
        "step_key": step_key,
        "chapter": chapter_analysis_workflow_fingerprint(ch),
        "analysis_data": ch.analysis_data.as_ref(),
    })
}

async fn preflight_chapter_analysis(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    operation: &str,
    empty_content_message: &str,
    require_saved_analysis: bool,
) -> Result<chapter::Model, String> {
    let ch = chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| format!("{operation}失败：查询章节失败: {err}"))?
        .ok_or_else(|| format!("{operation}失败：章节不存在: chapter_id={chapter_id}"))?;
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    ensure_chapter_not_generating(&ch, operation)?;
    if !ch.has_available_content() {
        return Err(empty_content_message.to_string());
    }
    if require_saved_analysis {
        let analysis = ch
            .analysis_data
            .as_ref()
            .ok_or_else(|| "当前章节还没有已保存的结构化分析，不能继续同步".to_string())?;
        validate_saved_chapter_analysis(analysis)
            .map_err(|err| format!("当前章节已保存的结构化分析数据无效，不能继续同步：{err}"))?;
        ensure_chapter_analysis_sync_resume_needed(state, &ch).await?;
    }
    ensure_previous_chapter_analysis_complete(state, &ch, operation).await?;
    Ok(ch)
}

async fn ensure_chapter_analysis_sync_resume_needed(
    state: &AppState,
    ch: &chapter::Model,
) -> Result<(), String> {
    let run = state
        .workflow_run
        .latest_run_by_business_key(&chapter_analysis_workflow_business_key(
            ch.project_id,
            ch.id,
        ))
        .await
        .map_err(|err| format!("检查章节结构化分析同步状态失败: {err}"))?;
    let Some(run) = run else {
        return Ok(());
    };
    if let Some(reason) = chapter_analysis_resume_block_reason(&run, ch) {
        return Err(reason);
    }
    Ok(())
}

pub(crate) fn chapter_analysis_resume_block_reason(
    run: &workflow_run::Model,
    ch: &chapter::Model,
) -> Option<String> {
    if !chapter_analysis_run_input_matches(&run.input_fingerprint_json, ch) {
        return None;
    }
    if run.status == WORKFLOW_STATUS_COMPLETED && !run.can_resume {
        return Some("当前章节结构化分析派生数据已同步完成，无需继续同步".to_string());
    }
    if run.status == WORKFLOW_STATUS_RUNNING {
        return Some("当前章节结构化分析正在同步中，请等待完成后再操作".to_string());
    }
    if !run.can_resume {
        return Some(
            run.resume_block_reason
                .clone()
                .unwrap_or_else(|| "该章节结构化分析工作流当前不可继续同步".to_string()),
        );
    }
    None
}

fn ensure_chapter_not_generating(ch: &chapter::Model, operation: &str) -> Result<(), String> {
    if ch.is_generating() {
        return Err(format!(
            "第 {} 章正在生成中，请等待当前生成完成后再{operation}",
            ch.chapter_number
        ));
    }
    Ok(())
}

fn blocked_chapter_analysis_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<&str>,
    note: impl Into<String>,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        key,
        title,
        prompt_name.map(str::to_string),
        "blocked",
        Some(note.into()),
        None,
    )
}

fn character_discovery_chapter_analysis_steps(
    steps: Vec<crate::actions::character_ai::CharacterAiPreviewStep>,
) -> Vec<ChatStatusPreviewStep> {
    steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        })
        .collect()
}

fn object_discovery_chapter_analysis_steps(
    steps: Vec<crate::actions::object_discovery::ObjectDiscoveryPreviewStep>,
) -> Vec<ChatStatusPreviewStep> {
    steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                Some(step.request),
            )
        })
        .collect()
}

fn scene_profile_sync_chapter_analysis_steps(
    steps: Vec<crate::actions::scene_profile_sync::SceneProfileSyncPreviewStep>,
) -> Vec<ChatStatusPreviewStep> {
    steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        })
        .collect()
}

fn blocked_scene_profile_sync_chapter_analysis_steps(
    chapter_number: i32,
    note: &str,
) -> Vec<ChatStatusPreviewStep> {
    vec![
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_scenes"),
            "分析后场景档案候选识别请求",
            Some("scene_profile_candidates"),
            note,
        ),
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_occurrences"),
            "分析后场景出场抽取请求",
            Some("scene_occurrence_extraction"),
            "该请求依赖场景档案候选识别返回的 scenes，发送前无法提前构造真实请求",
        ),
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_transitions"),
            "分析后场景转场抽取请求",
            Some("scene_transition_extraction"),
            "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求",
        ),
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_entity_states"),
            "分析后场景实体状态抽取请求",
            Some("scene_entity_state_extraction"),
            "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求",
        ),
    ]
}

fn report_chapter_analysis_progress(
    progress: Option<&ChapterAnalysisProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.report(current, total, message, cancelled_message)?;
    }
    Ok(())
}

fn report_chapter_subprogress(
    progress: Option<&ChapterAnalysisProgress<'_>>,
    current: u32,
    total: u32,
    span_start: u32,
    span_end: u32,
    message: &str,
) {
    let Some(progress) = progress else {
        return;
    };
    let Some(reporter) = progress.reporter else {
        return;
    };
    let mapped = map_subprogress(current, total, span_start, span_end);
    reporter(mapped, CHAPTER_ANALYSIS_PROGRESS_TOTAL, message);
}

fn map_subprogress(current: u32, total: u32, span_start: u32, span_end: u32) -> u32 {
    let total = total.max(1);
    let span_width = span_end.saturating_sub(span_start);
    span_start + span_width.saturating_mul(current.min(total)) / total
}

#[cfg(test)]
mod analysis_order_tests {
    use super::*;

    fn chapter_row(
        chapter_number: i32,
        content: &str,
        status: &str,
        analyzed: bool,
    ) -> chapter::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter::Model {
            id: i64::from(chapter_number),
            project_id: 1,
            outline_id: None,
            chapter_number,
            title: None,
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: status.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: analyzed.then(valid_analysis_data),
            created_at: now,
            updated_at: now,
        }
    }

    fn valid_analysis_data() -> serde_json::Value {
        serde_json::json!({
            "metadata": {
                "characters": [],
                "locations": [],
                "items": [],
                "tags": [],
                "tone": null,
                "timeline_marker": null
            },
            "summaries": {
                "compressed": "章节摘要",
                "one_line": "一句话摘要",
                "keywords": []
            },
            "character_states": {},
            "foreshadowing": {
                "planted": [],
                "resolved": [],
                "tensions": []
            },
            "key_events": []
        })
    }

    #[test]
    fn previous_analysis_preflight_rejects_unanalyzed_prior_chapters() {
        let previous = vec![
            chapter_row(1, "正文", chapter::STATUS_DRAFT, true),
            chapter_row(2, "正文", chapter::STATUS_DRAFT, false),
        ];

        let err =
            ensure_previous_chapter_analysis_complete_from_rows("生成结构化分析", 3, &previous)
                .expect_err("missing previous analysis should block later chapter");

        assert!(err.contains("第2章未生成结构化分析"));
        assert!(err.contains("不能跳过前序章节"));
    }

    #[test]
    fn previous_analysis_preflight_rejects_invalid_prior_analysis_data() {
        let mut invalid = chapter_row(1, "正文", chapter::STATUS_DRAFT, true);
        invalid.analysis_data = Some(serde_json::json!({"metadata": {}}));
        let previous = vec![invalid];

        let err =
            ensure_previous_chapter_analysis_complete_from_rows("生成结构化分析", 2, &previous)
                .expect_err("invalid previous analysis should block later chapter");

        assert!(err.contains("第1章结构化分析数据无效"));
        assert!(err.contains("chapter_analysis"));
    }

    #[test]
    fn previous_analysis_preflight_accepts_analyzed_prior_chapters() {
        let previous = vec![
            chapter_row(1, "正文", chapter::STATUS_DRAFT, true),
            chapter_row(2, "正文", chapter::STATUS_IMPORTED, true),
        ];

        ensure_previous_chapter_analysis_complete_from_rows("生成结构化分析", 3, &previous)
            .expect("all previous chapters are analyzed");
    }

    #[test]
    fn subprogress_mapping_scales_into_parent_span() {
        assert_eq!(map_subprogress(0, 100, 58, 68), 58);
        assert_eq!(map_subprogress(50, 100, 58, 68), 63);
        assert_eq!(map_subprogress(100, 100, 58, 68), 68);
        assert_eq!(map_subprogress(6, 6, 78, 88), 88);
    }

    #[test]
    fn chapter_analysis_workflow_input_uses_stable_business_key_and_content_fingerprint() {
        let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
        let input = chapter_analysis_workflow_run_input(&chapter, "chapter_analysis.derived_sync");

        assert_eq!(input.business_key, "chapter_analysis:1:3");
        assert_eq!(input.workflow_kind, "chapter_analysis");
        assert_eq!(input.chapter_id, Some(3));
        assert_eq!(
            input.current_step_key.as_deref(),
            Some("chapter_analysis.derived_sync")
        );
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert!(!input.can_resume);
        let fingerprint = input
            .input_fingerprint_json
            .expect("workflow input should include fingerprint");
        assert_eq!(fingerprint["workflow_kind"], "chapter_analysis");
        assert_eq!(fingerprint["project_id"], 1);
        assert_eq!(fingerprint["chapter_id"], 3);
        assert_eq!(fingerprint["content_sha256"], source_content_sha256("正文"));
    }

    #[test]
    fn chapter_analysis_resume_input_matches_only_same_chapter_content() {
        let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, false);
        let fingerprint = Some(chapter_analysis_workflow_fingerprint(&chapter));

        assert!(chapter_analysis_run_input_matches(&fingerprint, &chapter));

        let changed = chapter_row(3, "正文已修改", chapter::STATUS_DRAFT, false);
        assert!(!chapter_analysis_run_input_matches(&fingerprint, &changed));
        assert!(!chapter_analysis_run_input_matches(&None, &chapter));
    }

    #[test]
    fn completed_chapter_analysis_workflow_blocks_manual_resume_sync() {
        let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
        let run = workflow_run_row(&chapter, WORKFLOW_STATUS_COMPLETED, false);

        let reason = chapter_analysis_resume_block_reason(&run, &chapter)
            .expect("completed matching run should block resume sync");

        assert!(reason.contains("已同步完成"));
    }

    #[test]
    fn resumable_failed_chapter_analysis_workflow_allows_manual_resume_sync() {
        let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
        let run = workflow_run_row(&chapter, WORKFLOW_STATUS_FAILED, true);

        assert!(chapter_analysis_resume_block_reason(&run, &chapter).is_none());
    }

    #[test]
    fn stale_chapter_analysis_workflow_does_not_block_manual_resume_sync() {
        let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
        let mut run = workflow_run_row(&chapter, WORKFLOW_STATUS_COMPLETED, false);
        run.input_fingerprint_json = Some(chapter_analysis_workflow_fingerprint(&chapter_row(
            3,
            "旧正文",
            chapter::STATUS_DRAFT,
            true,
        )));

        assert!(chapter_analysis_resume_block_reason(&run, &chapter).is_none());
    }

    #[test]
    fn chapter_analysis_phase_fingerprint_tracks_saved_analysis_data() {
        let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
        let mut edited = chapter.clone();
        edited.analysis_data = Some(serde_json::json!({
            "metadata": {
                "characters": ["新角色"],
                "locations": [],
                "items": [],
                "tags": [],
                "tone": null,
                "timeline_marker": null
            },
            "summaries": {
                "compressed": "编辑后的章节摘要",
                "one_line": "一句话摘要",
                "keywords": []
            },
            "character_states": {},
            "foreshadowing": {
                "planted": [],
                "resolved": [],
                "tensions": []
            },
            "key_events": []
        }));

        assert_ne!(
            chapter_analysis_phase_fingerprint(&chapter, CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY),
            chapter_analysis_phase_fingerprint(&edited, CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY)
        );
    }

    fn workflow_run_row(
        ch: &chapter::Model,
        status: &str,
        can_resume: bool,
    ) -> workflow_run::Model {
        let now = chrono::Utc::now().naive_utc();
        workflow_run::Model {
            id: 1,
            business_key: chapter_analysis_workflow_business_key(ch.project_id, ch.id),
            workflow_kind: "chapter_analysis".to_string(),
            project_id: Some(ch.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(ch.id),
            chapter_id: Some(ch.id),
            mode: None,
            input_fingerprint_json: Some(chapter_analysis_workflow_fingerprint(ch)),
            status: status.to_string(),
            current_step_key: Some(CHAPTER_ANALYSIS_STEP_DERIVED_SYNC.to_string()),
            progress_json: None,
            active_task_id: None,
            can_resume,
            resume_block_reason: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            completed_at: (status == WORKFLOW_STATUS_COMPLETED).then_some(now),
        }
    }

    #[test]
    fn blocked_scene_profile_sync_preview_steps_cover_scene_sync_chain() {
        let steps =
            blocked_scene_profile_sync_chapter_analysis_steps(3, "结构化分析保存后才能构造");

        assert_eq!(steps.len(), 4);
        assert!(steps.iter().all(|step| step.state == "blocked"));
        assert_eq!(
            steps
                .iter()
                .filter_map(|step| step.prompt_name.as_deref())
                .collect::<Vec<_>>(),
            vec![
                "scene_profile_candidates",
                "scene_occurrence_extraction",
                "scene_transition_extraction",
                "scene_entity_state_extraction",
            ]
        );
        assert_eq!(steps[0].key, "scene_profile_sync_ch3_scenes");
    }
}

async fn sync_chapter_analysis_outputs(
    state: &AppState,
    ch: &chapter::Model,
) -> Result<(), String> {
    let Some(analysis) = ch.analysis_data.as_ref() else {
        return Err(format!(
            "章节缺少已保存结构化分析，不能同步派生数据: chapter_id={}",
            ch.id
        ));
    };
    validate_saved_chapter_analysis(analysis).map_err(|err| {
        format!(
            "章节已保存的结构化分析数据无效，不能同步派生数据: chapter_id={}：{err}",
            ch.id
        )
    })?;
    let foreshadowing = sync_foreshadowing_from_analysis(state, ch, analysis).await?;
    for item in &foreshadowing.current {
        ingest_foreshadowing_rag(state, ch.project_id, item).await?;
    }
    delete_foreshadowing_rag_by_ids(state, ch.project_id, &foreshadowing.stale_ids).await?;
    sync_chapter_rag_records(state, ch).await?;
    Ok(())
}

fn report_analysis_character_sync_failures(sync_result: &serde_json::Value) -> Result<(), String> {
    let failed = sync_result
        .get("failed")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    if failed == 0 {
        return Ok(());
    }

    let details = sync_result
        .get("failed")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .map(|item| {
            let name = item
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("未知角色");
            let error = item
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("未知错误");
            format!("{name}: {error}")
        })
        .take(5)
        .collect::<Vec<_>>()
        .join("；");
    Err(format!(
        "分析后角色档案同步未完全成功：失败 {failed} 个{}；已成功写入的角色会保留，重新同步会从未完成角色的断点继续",
        if details.is_empty() {
            String::new()
        } else {
            format!("：{details}")
        }
    ))
}

#[derive(Debug, Default)]
struct ForeshadowingAnalysisSync {
    current: Vec<foreshadowing_index::Model>,
    stale_ids: Vec<i64>,
}

struct ForeshadowingDraft {
    description: String,
    status: String,
    related_characters: Option<serde_json::Value>,
    importance: i32,
}

async fn sync_foreshadowing_from_analysis(
    state: &AppState,
    ch: &chapter::Model,
    analysis: &serde_json::Value,
) -> Result<ForeshadowingAnalysisSync, String> {
    let entries = foreshadowing_entries(analysis);
    let now = chrono::Utc::now().naive_utc();
    let mut seen_descriptions = HashSet::new();
    let mut drafts = Vec::with_capacity(entries.len());
    for entry in entries {
        let description = foreshadowing_description_from_entry(&entry);
        if !seen_descriptions.insert(description.clone()) {
            return Err(format!(
                "同步章节伏笔失败：同一章节存在重复伏笔：chapter_id={}, description={}",
                ch.id, description
            ));
        }
        let importance = match priority_to_importance(entry.priority.as_deref()) {
            Ok(importance) => importance,
            Err(err) => {
                return Err(format!(
                    "同步章节伏笔失败：优先级无效：chapter_id={}, description={}：{err}",
                    ch.id, description
                ));
            }
        };
        drafts.push(ForeshadowingDraft {
            description,
            status: entry.status,
            related_characters: entry.related_entities,
            importance,
        });
    }

    let existing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(ch.project_id))
        .filter(foreshadowing_index::Column::SetupChapter.eq(ch.chapter_number))
        .order_by_asc(foreshadowing_index::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询章节伏笔失败：chapter_id={}：{err}", ch.id))?;
    let mut existing_by_description = BTreeMap::<String, Vec<foreshadowing_index::Model>>::new();
    for item in existing {
        existing_by_description
            .entry(item.description.clone())
            .or_default()
            .push(item);
    }

    let mut current = Vec::with_capacity(drafts.len());
    for draft in drafts {
        if let Some(candidates) = existing_by_description.get_mut(&draft.description)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if foreshadowing_matches_draft(&existing, &draft) {
                current.push(existing);
                continue;
            }
            let status = foreshadowing_status_after_setup_resync(
                existing.status.clone(),
                existing.payoff_chapter,
                &draft.status,
            );
            let mut active: foreshadowing_index::ActiveModel = existing.into();
            active.status = Set(status);
            active.related_characters = Set(draft.related_characters);
            active.importance = Set(draft.importance);
            active.updated_at = Set(now);
            let updated = active
                .update(state.db.as_ref())
                .await
                .map_err(|err| format!("更新章节伏笔失败：chapter_id={}：{err}", ch.id))?;
            current.push(updated);
        } else {
            let inserted_item = (foreshadowing_index::ActiveModel {
                project_id: Set(ch.project_id),
                description: Set(draft.description),
                setup_chapter: Set(ch.chapter_number),
                payoff_chapter: Set(None),
                status: Set(draft.status),
                related_characters: Set(draft.related_characters),
                importance: Set(draft.importance),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .insert(state.db.as_ref())
            .await
            .map_err(|err| format!("同步章节伏笔失败：chapter_id={}：{err}", ch.id))?;
            current.push(inserted_item);
        }
    }

    let mut stale_ids = Vec::new();
    for stale in existing_by_description.into_values().flatten() {
        stale_ids.push(stale.id);
        foreshadowing_index::Entity::delete_by_id(stale.id)
            .exec(state.db.as_ref())
            .await
            .map_err(|err| format!("删除过期章节伏笔失败：chapter_id={}：{err}", ch.id))?;
    }

    Ok(ForeshadowingAnalysisSync { current, stale_ids })
}

fn foreshadowing_description_from_entry(entry: &ForeshadowingEntry) -> String {
    match (entry.category.as_deref(), entry.original_text.as_deref()) {
        (Some(category), Some(original_text)) if !original_text.trim().is_empty() => {
            format!(
                "{} [{}] 原文: {}",
                entry.description, category, original_text
            )
        }
        (Some(category), _) => format!("{} [{}]", entry.description, category),
        (_, Some(original_text)) if !original_text.trim().is_empty() => {
            format!("{} 原文: {}", entry.description, original_text)
        }
        _ => entry.description.clone(),
    }
}

fn foreshadowing_matches_draft(
    existing: &foreshadowing_index::Model,
    draft: &ForeshadowingDraft,
) -> bool {
    let expected_status = foreshadowing_status_after_setup_resync(
        existing.status.clone(),
        existing.payoff_chapter,
        &draft.status,
    );
    existing.status == expected_status
        && existing.related_characters == draft.related_characters
        && existing.importance == draft.importance
}

fn foreshadowing_status_after_setup_resync(
    existing_status: String,
    payoff_chapter: Option<i32>,
    draft_status: &str,
) -> String {
    if payoff_chapter.is_some() || matches!(existing_status.as_str(), "resolved" | "abandoned") {
        existing_status
    } else {
        draft_status.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreshadowing_setup_resync_preserves_resolved_status() {
        assert_eq!(
            foreshadowing_status_after_setup_resync("resolved".to_string(), Some(12), "pending"),
            "resolved"
        );
        assert_eq!(
            foreshadowing_status_after_setup_resync("pending".to_string(), None, "pending"),
            "pending"
        );
    }

    #[test]
    fn foreshadowing_description_identity_includes_category_and_original_text() {
        let description = foreshadowing_description_from_entry(&ForeshadowingEntry {
            description: "暗号指向后续叛徒身份".to_string(),
            original_text: Some("墙上的半枚印记".to_string()),
            category: Some("plot_twist".to_string()),
            priority: Some("high".to_string()),
            related_entities: None,
            status: "pending".to_string(),
        });

        assert_eq!(
            description,
            "暗号指向后续叛徒身份 [plot_twist] 原文: 墙上的半枚印记"
        );
    }

    #[test]
    fn analysis_character_sync_failure_report_keeps_failed_profiles_visible() {
        let result = serde_json::json!({
            "status": "completed_with_errors",
            "failed": [
                { "name": "林岚", "error": "模型响应缺少字段" }
            ]
        });

        let err = report_analysis_character_sync_failures(&result)
            .expect_err("failed profile sync should be visible");

        assert!(err.contains("失败 1 个"));
        assert!(err.contains("林岚: 模型响应缺少字段"));
    }
}
