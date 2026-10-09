use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use a4n_api_contract::{
    admin::{
        DeleteTestRunInput, ListTestRunsInput, StartTestRunInput, TestImpact, TestRunDetail,
        TestRunStatus, TestRunSummary, TestScenarioDescriptor, TestScenarioEffect, TestTarget,
        TestTargetMode,
    },
    project::{CreateProjectInput, DuplicateProjectInput, UpdateProjectInput},
};
use chrono::Utc;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    AppState,
    actions::{app_error_message, project_duplication, project_records},
};

const SCENARIO_VERSION: u32 = 1;
#[derive(Default)]
struct ControlState {
    active_run_id: Option<String>,
    cancellations: HashMap<String, CancellationToken>,
}

#[derive(Clone)]
pub struct TestConsole {
    root: Arc<PathBuf>,
    control: Arc<Mutex<ControlState>>,
    runs: Arc<Mutex<HashMap<String, TestRunDetail>>>,
}

impl TestConsole {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Arc::new(root),
            control: Arc::new(Mutex::new(ControlState::default())),
            runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn scenarios() -> Vec<TestScenarioDescriptor> {
        vec![
            scenario(
                "admin_health_check",
                "system",
                "管理员健康检查",
                "验证数据库和运行时服务健康状态可以读取。",
                TestScenarioEffect::Read,
                vec![TestTargetMode::TestProject, TestTargetMode::CurrentProject],
                json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            scenario(
                "admin_overview",
                "system",
                "管理员运行概览",
                "验证管理员概览统计可以读取并返回结构化结果。",
                TestScenarioEffect::Read,
                vec![TestTargetMode::TestProject, TestTargetMode::CurrentProject],
                json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            scenario(
                "project_list",
                "project",
                "项目列表读取",
                "验证管理员可以读取项目列表。",
                TestScenarioEffect::Read,
                vec![TestTargetMode::TestProject, TestTargetMode::CurrentProject],
                json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            scenario(
                "project_create_update",
                "project",
                "项目创建与更新",
                "创建测试项目或更新当前项目描述，并重新读取结果。",
                TestScenarioEffect::Write,
                vec![TestTargetMode::TestProject, TestTargetMode::CurrentProject],
                json!({
                    "type":"object",
                    "properties":{
                        "title":{"type":"string","minLength":1,"maxLength":120},
                        "description":{"type":"string","maxLength":2000}
                    },
                    "additionalProperties":false
                }),
            ),
            scenario(
                "project_duplicate",
                "project",
                "项目复制",
                "复制当前项目或指定源项目，并验证复制后的项目可以读取。",
                TestScenarioEffect::Write,
                vec![TestTargetMode::TestProject, TestTargetMode::CurrentProject],
                json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
            scenario(
                "project_status_read",
                "project",
                "项目状态读取",
                "读取目标项目状态机，验证项目状态合同可用。",
                TestScenarioEffect::Read,
                vec![TestTargetMode::TestProject, TestTargetMode::CurrentProject],
                json!({"type":"object","properties":{},"additionalProperties":false}),
            ),
        ]
    }

    pub async fn start(
        &self,
        state: AppState,
        user_id: i64,
        mut input: StartTestRunInput,
    ) -> Result<TestRunSummary, String> {
        let scenario = Self::scenarios()
            .into_iter()
            .find(|item| item.id == input.scenario_id)
            .ok_or_else(|| format!("未知测试场景：{}", input.scenario_id))?;
        validate_target(&scenario, &input.target)?;
        if input.parameters.is_null() {
            input.parameters = json!({});
        }
        validate_parameters(&scenario, &input.parameters)?;
        if matches!(input.target, TestTarget::CurrentProject { .. })
            && !matches!(scenario.effect, TestScenarioEffect::Read)
            && !input.confirm_impact
        {
            return Err("当前项目测试需要确认数据影响".to_string());
        }

        let run_id = Uuid::new_v4().to_string();
        let created_at = now();
        let detail = TestRunDetail {
            summary: TestRunSummary {
                id: run_id.clone(),
                scenario_id: scenario.id,
                scenario_version: scenario.version,
                status: TestRunStatus::Queued,
                target: input.target.clone(),
                target_project_id: initial_target_project_id(&input.target),
                created_by: user_id,
                created_at,
                started_at: None,
                finished_at: None,
                duration_ms: None,
                message: None,
                impact: Vec::new(),
            },
            parameters: input.parameters.clone(),
            steps: Vec::new(),
            logs: Vec::new(),
        };

        let cancellation = CancellationToken::new();
        {
            let mut control = self.control.lock().await;
            if let Some(active) = control.active_run_id.as_ref() {
                return Err(format!("已有测试正在运行：{active}"));
            }
            control.active_run_id = Some(run_id.clone());
            control
                .cancellations
                .insert(run_id.clone(), cancellation.clone());
        }
        self.runs
            .lock()
            .await
            .insert(run_id.clone(), detail.clone());
        if let Err(error) = self.persist(&detail).await {
            self.runs.lock().await.remove(&run_id);
            let mut control = self.control.lock().await;
            control.cancellations.remove(&run_id);
            if control.active_run_id.as_deref() == Some(run_id.as_str()) {
                control.active_run_id = None;
            }
            return Err(error);
        }

        let manager = self.clone();
        tokio::spawn(async move {
            manager
                .run(state, user_id, run_id, input, cancellation)
                .await;
        });
        Ok(detail.summary)
    }

    pub async fn get(&self, run_id: &str) -> Result<TestRunDetail, String> {
        if let Some(detail) = self.runs.lock().await.get(run_id).cloned() {
            return Ok(detail);
        }
        let path = self.path_for(run_id)?;
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|error| format!("读取测试运行失败：{error}"))?;
        let mut detail: TestRunDetail =
            serde_json::from_slice(&bytes).map_err(|error| format!("测试运行报告损坏：{error}"))?;
        self.recover_interrupted(&mut detail).await?;
        Ok(detail)
    }

    pub async fn list(&self, input: ListTestRunsInput) -> Result<Vec<TestRunSummary>, String> {
        let limit = input.limit.clamp(1, 100) as usize;
        let mut details = Vec::new();
        let mut entries = match tokio::fs::read_dir(self.root.as_ref()).await {
            Ok(entries) => Some(entries),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("读取测试运行目录失败：{error}")),
        };
        if let Some(entries) = entries.as_mut() {
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|error| format!("读取测试运行目录失败：{error}"))?
            {
                if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let bytes = tokio::fs::read(entry.path())
                    .await
                    .map_err(|error| format!("读取测试运行报告失败：{error}"))?;
                let mut detail = serde_json::from_slice::<TestRunDetail>(&bytes)
                    .map_err(|error| format!("测试运行报告损坏：{error}"))?;
                self.recover_interrupted(&mut detail).await?;
                details.push(detail);
            }
        }
        for detail in self.runs.lock().await.values().cloned() {
            if !details
                .iter()
                .any(|item| item.summary.id == detail.summary.id)
            {
                details.push(detail);
            }
        }
        details.sort_by(|left, right| right.summary.created_at.cmp(&left.summary.created_at));
        Ok(details
            .into_iter()
            .take(limit)
            .map(|detail| detail.summary)
            .collect())
    }

    pub async fn cancel(&self, run_id: &str) -> Result<TestRunSummary, String> {
        let token = self.control.lock().await.cancellations.get(run_id).cloned();
        if let Some(token) = token {
            token.cancel();
        }
        Ok(self.get(run_id).await?.summary)
    }

    pub async fn delete(&self, state: &AppState, input: DeleteTestRunInput) -> Result<(), String> {
        if self.control.lock().await.active_run_id.as_deref() == Some(input.run_id.as_str()) {
            return Err("运行中的测试不能删除".to_string());
        }
        let detail = self.get(&input.run_id).await?;
        if input.delete_test_project {
            if !matches!(&detail.summary.target, TestTarget::TestProject { .. }) {
                return Err("当前项目测试不能由测试历史删除项目".to_string());
            }
            let mut project_ids = detail
                .summary
                .impact
                .iter()
                .filter(|impact| impact.action == "create" && impact.resource == "project")
                .filter_map(|impact| impact.project_id)
                .collect::<Vec<_>>();
            project_ids.sort_unstable();
            project_ids.dedup();
            if project_ids.is_empty() {
                return Err("该运行没有由测试中心创建的项目".to_string());
            }
            for project_id in project_ids.into_iter().rev() {
                project_records::delete_project(state, detail.summary.created_by, project_id)
                    .await?;
            }
        }
        self.runs.lock().await.remove(&input.run_id);
        let path = self.path_for(&input.run_id)?;
        match tokio::fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("删除测试历史失败：{error}")),
        }
    }

    async fn run(
        &self,
        state: AppState,
        user_id: i64,
        run_id: String,
        input: StartTestRunInput,
        cancellation: CancellationToken,
    ) {
        let started = Instant::now();
        let _ = self
            .mutate(&run_id, |detail| {
                detail.summary.started_at = Some(now());
                detail.summary.status = TestRunStatus::Running;
            })
            .await;
        let scenario_timeout = Self::scenarios()
            .into_iter()
            .find(|scenario| scenario.id == input.scenario_id)
            .map(|scenario| scenario.timeout_seconds)
            .unwrap_or(300);
        let result = tokio::select! {
            _ = cancellation.cancelled() => Err((TestRunStatus::Cancelled, "测试已取消".to_string())),
            result = tokio::time::timeout(
                Duration::from_secs(scenario_timeout),
                self.run_scenario(&state, user_id, &run_id, &input, &cancellation),
            ) => match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err((TestRunStatus::Failed, error)),
                Err(_) => Err((TestRunStatus::TimedOut, format!("测试超过 {scenario_timeout} 秒限制"))),
            }
        };
        let (status, message) = match result {
            Ok(()) => (TestRunStatus::Passed, Some("测试通过".to_string())),
            Err((status, error)) => (status, Some(error)),
        };
        let _ = self
            .mutate(&run_id, |detail| {
                detail.summary.status = status.clone();
                detail.summary.message = message;
                detail.summary.finished_at = Some(now());
                detail.summary.duration_ms = Some(started.elapsed().as_millis() as u64);
                for step in &mut detail.steps {
                    if step.status == TestRunStatus::Running {
                        step.status = status.clone();
                        step.finished_at = detail.summary.finished_at.clone();
                        step.message = detail.summary.message.clone();
                    }
                }
                detail.logs.push(format!(
                    "测试结束：{}：{}",
                    status_label(&status),
                    detail.summary.message.as_deref().unwrap_or_default()
                ));
            })
            .await;
        let mut control = self.control.lock().await;
        control.cancellations.remove(&run_id);
        if control.active_run_id.as_deref() == Some(run_id.as_str()) {
            control.active_run_id = None;
        }
    }

    async fn run_scenario(
        &self,
        state: &AppState,
        user_id: i64,
        run_id: &str,
        input: &StartTestRunInput,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        match input.scenario_id.as_str() {
            "admin_health_check" => {
                self.step(run_id, "读取服务健康状态", || async {
                    state
                        .admin_read_application()
                        .check_database_health(user_id)
                        .await
                        .map_err(app_error_message)?;
                    state
                        .service_health_application()
                        .summaries()
                        .await
                        .map_err(app_error_message)
                        .map(|_| ())
                })
                .await?;
            }
            "admin_overview" => {
                self.step(run_id, "读取管理员概览", || async {
                    state
                        .admin_read_application()
                        .overview(user_id)
                        .await
                        .map_err(app_error_message)
                        .map(|_| ())
                })
                .await?;
            }
            "project_list" => {
                self.step(run_id, "读取项目列表", || async {
                    state
                        .admin_read_application()
                        .all_project_summaries(user_id)
                        .await
                        .map_err(app_error_message)
                        .map(|_| ())
                })
                .await?;
            }
            "project_create_update" => {
                let target = ensure_project(state, user_id, input, run_id, self).await?;
                self.record_target(run_id, target).await?;
                let title = parameter_string(&input.parameters, "title")
                    .unwrap_or_else(|| format!("A4N 测试项目 {}", &run_id[..8]));
                let description = parameter_string(&input.parameters, "description")
                    .unwrap_or_else(|| "由管理员测试中心创建的功能测试项目".to_string());
                self.step(run_id, "写入项目描述", || async {
                    project_records::update_project(
                        state,
                        user_id,
                        target,
                        UpdateProjectInput {
                            title: Some(title),
                            description: Some(description),
                            fact_feedback_interval_chapters: None,
                        },
                    )
                    .await
                    .map(|_| ())
                })
                .await?;
                self.step(run_id, "重新读取项目", || async {
                    state
                        .project_read_application()
                        .summary(user_id, target)
                        .await
                        .map_err(app_error_message)
                        .map(|_| ())
                })
                .await?;
                self.impact(
                    run_id,
                    TestImpact {
                        action: "update".to_string(),
                        resource: "project".to_string(),
                        project_id: Some(target),
                        detail: "更新项目标题和描述".to_string(),
                    },
                )
                .await?;
            }
            "project_duplicate" => {
                let source = match &input.target {
                    TestTarget::CurrentProject { project_id } => {
                        state
                            .project_read_application()
                            .summary(user_id, *project_id)
                            .await
                            .map_err(app_error_message)?;
                        *project_id
                    }
                    TestTarget::TestProject {
                        source_project_id: Some(project_id),
                    } => {
                        state
                            .project_read_application()
                            .summary(user_id, *project_id)
                            .await
                            .map_err(app_error_message)?;
                        *project_id
                    }
                    TestTarget::TestProject {
                        source_project_id: None,
                    } => ensure_project(state, user_id, input, run_id, self).await?,
                };
                let title = format!("A4N 测试复制 {}", &run_id[..8]);
                let copied = self
                    .step(run_id, "复制项目", || async {
                        project_duplication::duplicate_project(
                            state,
                            user_id,
                            DuplicateProjectInput {
                                project_id: source,
                                new_title: title,
                            },
                        )
                        .await
                    })
                    .await?;
                self.record_target(run_id, copied.id).await?;
                self.step(run_id, "读取复制结果", || async {
                    state
                        .project_read_application()
                        .summary(user_id, copied.id)
                        .await
                        .map_err(app_error_message)
                        .map(|_| ())
                })
                .await?;
                self.impact(
                    run_id,
                    TestImpact {
                        action: "create".to_string(),
                        resource: "project".to_string(),
                        project_id: Some(copied.id),
                        detail: format!("复制源项目 {source}"),
                    },
                )
                .await?;
            }
            "project_status_read" => {
                let target = ensure_project(state, user_id, input, run_id, self).await?;
                self.record_target(run_id, target).await?;
                self.step(run_id, "读取项目状态机", || async {
                    state
                        .project_status_read_application()
                        .status_machine(user_id, target)
                        .await
                        .map_err(app_error_message)
                        .map(|_| ())
                })
                .await?;
            }
            _ => return Err(format!("测试场景未实现：{}", input.scenario_id)),
        }
        if cancellation.is_cancelled() {
            return Err("测试已取消".to_string());
        }
        Ok(())
    }

    async fn step<T, F, Fut>(&self, run_id: &str, name: &str, operation: F) -> Result<T, String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let index = self.begin_step(run_id, name).await?;
        match operation().await {
            Ok(value) => {
                self.finish_step(
                    run_id,
                    index,
                    TestRunStatus::Passed,
                    Some("通过".to_string()),
                )
                .await?;
                Ok(value)
            }
            Err(error) => {
                self.finish_step(run_id, index, TestRunStatus::Failed, Some(error.clone()))
                    .await?;
                Err(error)
            }
        }
    }

    async fn begin_step(&self, run_id: &str, name: &str) -> Result<u32, String> {
        let mut index = 0;
        self.mutate(run_id, |detail| {
            index = detail.steps.len() as u32;
            detail.steps.push(a4n_api_contract::admin::TestStepSummary {
                index,
                name: name.to_string(),
                status: TestRunStatus::Running,
                started_at: now(),
                finished_at: None,
                message: None,
            });
            detail.logs.push(format!("开始：{name}"));
        })
        .await?;
        Ok(index)
    }

    async fn finish_step(
        &self,
        run_id: &str,
        index: u32,
        status: TestRunStatus,
        message: Option<String>,
    ) -> Result<(), String> {
        self.mutate(run_id, |detail| {
            if let Some(step) = detail.steps.iter_mut().find(|step| step.index == index) {
                step.status = status;
                step.finished_at = Some(now());
                step.message = message.clone();
                detail.logs.push(format!(
                    "结束：{}：{}",
                    step.name,
                    message.as_deref().unwrap_or_default()
                ));
            }
        })
        .await
        .map(|_| ())
    }

    async fn record_target(&self, run_id: &str, project_id: i64) -> Result<(), String> {
        self.mutate(run_id, |detail| {
            detail.summary.target_project_id = Some(project_id);
        })
        .await
        .map(|_| ())
    }

    async fn recover_interrupted(&self, detail: &mut TestRunDetail) -> Result<(), String> {
        if !matches!(
            detail.summary.status,
            TestRunStatus::Queued | TestRunStatus::Running
        ) {
            return Ok(());
        }
        detail.summary.status = TestRunStatus::Failed;
        detail.summary.message = Some("Host 重启时测试尚未完成".to_string());
        detail.summary.finished_at = Some(now());
        for step in &mut detail.steps {
            if step.status == TestRunStatus::Running {
                step.status = TestRunStatus::Failed;
                step.finished_at = detail.summary.finished_at.clone();
                step.message = detail.summary.message.clone();
            }
        }
        detail
            .logs
            .push("测试失败：Host 重启时测试尚未完成".to_string());
        self.persist(detail).await
    }

    async fn impact(&self, run_id: &str, impact: TestImpact) -> Result<(), String> {
        self.mutate(run_id, |detail| detail.summary.impact.push(impact.clone()))
            .await
            .map(|_| ())
    }

    async fn mutate<F>(&self, run_id: &str, change: F) -> Result<TestRunDetail, String>
    where
        F: FnOnce(&mut TestRunDetail),
    {
        let mut detail = self.get(run_id).await?;
        change(&mut detail);
        self.runs
            .lock()
            .await
            .insert(run_id.to_string(), detail.clone());
        self.persist(&detail).await?;
        Ok(detail)
    }

    async fn persist(&self, detail: &TestRunDetail) -> Result<(), String> {
        tokio::fs::create_dir_all(self.root.as_ref())
            .await
            .map_err(|error| format!("创建测试运行目录失败：{error}"))?;
        let path = self.path_for(&detail.summary.id)?;
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(detail)
            .map_err(|error| format!("序列化测试运行失败：{error}"))?;
        tokio::fs::write(&temporary, bytes)
            .await
            .map_err(|error| format!("写入测试运行失败：{error}"))?;
        tokio::fs::rename(temporary, path)
            .await
            .map_err(|error| format!("提交测试运行失败：{error}"))
    }

    fn path_for(&self, run_id: &str) -> Result<PathBuf, String> {
        if run_id.is_empty()
            || !run_id
                .chars()
                .all(|value| value.is_ascii_hexdigit() || value == '-')
        {
            return Err("测试运行 ID 无效".to_string());
        }
        Ok(self.root.join(format!("{run_id}.json")))
    }
}

fn scenario(
    id: &str,
    category: &str,
    title: &str,
    description: &str,
    effect: TestScenarioEffect,
    target_modes: Vec<TestTargetMode>,
    parameter_schema: Value,
) -> TestScenarioDescriptor {
    TestScenarioDescriptor {
        id: id.to_string(),
        version: SCENARIO_VERSION,
        category: category.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        effect,
        target_modes,
        timeout_seconds: 300,
        parameter_schema,
    }
}

fn validate_target(scenario: &TestScenarioDescriptor, target: &TestTarget) -> Result<(), String> {
    let mode = match target {
        TestTarget::TestProject { source_project_id } => {
            if source_project_id.is_some_and(|project_id| project_id <= 0) {
                return Err("源项目 ID 必须为正数".to_string());
            }
            TestTargetMode::TestProject
        }
        TestTarget::CurrentProject { project_id } => {
            if *project_id <= 0 {
                return Err("项目 ID 必须为正数".to_string());
            }
            TestTargetMode::CurrentProject
        }
    };
    if scenario.target_modes.contains(&mode) {
        Ok(())
    } else {
        Err(format!("测试场景不支持目标模式：{mode:?}"))
    }
}

fn initial_target_project_id(target: &TestTarget) -> Option<i64> {
    match target {
        TestTarget::CurrentProject { project_id } => Some(*project_id),
        TestTarget::TestProject { .. } => None,
    }
}

async fn ensure_project(
    state: &AppState,
    user_id: i64,
    input: &StartTestRunInput,
    run_id: &str,
    console: &TestConsole,
) -> Result<i64, String> {
    match input.target {
        TestTarget::CurrentProject { project_id } => {
            state
                .project_read_application()
                .summary(user_id, project_id)
                .await
                .map_err(app_error_message)?;
            Ok(project_id)
        }
        TestTarget::TestProject { source_project_id } => {
            if let Some(source_project_id) = source_project_id {
                let copied = project_duplication::duplicate_project(
                    state,
                    user_id,
                    DuplicateProjectInput {
                        project_id: source_project_id,
                        new_title: format!("A4N 测试副本 {}", &run_id[..8]),
                    },
                )
                .await?;
                console.record_target(run_id, copied.id).await?;
                console
                    .impact(
                        run_id,
                        TestImpact {
                            action: "create".to_string(),
                            resource: "project".to_string(),
                            project_id: Some(copied.id),
                            detail: format!("从项目 {source_project_id} 创建测试副本"),
                        },
                    )
                    .await?;
                Ok(copied.id)
            } else {
                let created = project_records::create_project(
                    state,
                    user_id,
                    CreateProjectInput {
                        title: format!("A4N 测试项目 {}", &run_id[..8]),
                        description: Some("由管理员测试中心创建的测试副本".to_string()),
                        project_type: "novel_long".to_string(),
                    },
                )
                .await?;
                console.record_target(run_id, created.id).await?;
                console
                    .impact(
                        run_id,
                        TestImpact {
                            action: "create".to_string(),
                            resource: "project".to_string(),
                            project_id: Some(created.id),
                            detail: "创建独立测试项目".to_string(),
                        },
                    )
                    .await?;
                Ok(created.id)
            }
        }
    }
}

fn parameter_string(parameters: &Value, key: &str) -> Option<String> {
    parameters
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn validate_parameters(
    scenario: &TestScenarioDescriptor,
    parameters: &Value,
) -> Result<(), String> {
    let object = parameters
        .as_object()
        .ok_or_else(|| "测试参数必须是对象".to_string())?;
    let allowed: &[&str] = match scenario.id.as_str() {
        "project_create_update" => &["title", "description"],
        "project_duplicate"
        | "admin_health_check"
        | "admin_overview"
        | "project_list"
        | "project_status_read" => &[],
        _ => return Err(format!("测试场景未实现：{}", scenario.id)),
    };
    if let Some(name) = object.keys().find(|name| !allowed.contains(&name.as_str())) {
        return Err(format!("不支持的测试参数：{name}"));
    }
    if scenario.id == "project_create_update" {
        if let Some(title) = object.get("title") {
            let title = title
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty() && value.chars().count() <= 120)
                .ok_or_else(|| "项目标题必须为 1 到 120 个字符".to_string())?;
            let _ = title;
        }
        if object.get("description").is_some_and(|description| {
            description
                .as_str()
                .is_none_or(|value| value.chars().count() > 2000)
        }) {
            return Err("项目描述不能超过 2000 个字符".to_string());
        }
    }
    Ok(())
}

fn status_label(status: &TestRunStatus) -> &'static str {
    match status {
        TestRunStatus::Queued => "排队中",
        TestRunStatus::Running => "运行中",
        TestRunStatus::Passed => "通过",
        TestRunStatus::Failed => "失败",
        TestRunStatus::Blocked => "受阻",
        TestRunStatus::Cancelled => "已取消",
        TestRunStatus::TimedOut => "超时",
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_scenarios_have_unique_ids_and_bounded_timeouts() {
        let scenarios = TestConsole::scenarios();
        let ids = scenarios
            .iter()
            .map(|scenario| scenario.id.as_str())
            .collect::<std::collections::HashSet<_>>();

        assert!(!scenarios.is_empty());
        assert_eq!(ids.len(), scenarios.len());
        assert!(
            scenarios
                .iter()
                .all(|scenario| scenario.timeout_seconds > 0)
        );
        assert!(
            scenarios
                .iter()
                .all(|scenario| !scenario.target_modes.is_empty())
        );
    }

    #[test]
    fn scenario_parameters_reject_unknown_fields_and_invalid_project_values() {
        let scenarios = TestConsole::scenarios();
        let read = scenarios
            .iter()
            .find(|item| item.id == "project_list")
            .unwrap();
        let mutation = scenarios
            .iter()
            .find(|item| item.id == "project_create_update")
            .unwrap();

        assert!(validate_parameters(read, &json!({"unexpected": true})).is_err());
        assert!(validate_parameters(mutation, &json!({"title": "  "})).is_err());
        assert!(validate_parameters(mutation, &json!({"title": "x".repeat(121)})).is_err());
        assert!(validate_parameters(mutation, &json!({"description": "x".repeat(2001)})).is_err());
        assert!(
            validate_parameters(mutation, &json!({"title": "Test", "description": "ok"})).is_ok()
        );
        assert!(validate_parameters(mutation, &Value::Null).is_err());
    }

    #[test]
    fn targets_reject_invalid_ids_and_unsupported_modes() {
        let scenario = TestConsole::scenarios()
            .into_iter()
            .find(|item| item.id == "project_list")
            .unwrap();
        assert!(validate_target(&scenario, &TestTarget::CurrentProject { project_id: 0 }).is_err());
        assert!(
            validate_target(
                &scenario,
                &TestTarget::TestProject {
                    source_project_id: Some(-3)
                }
            )
            .is_err()
        );
    }

    #[test]
    fn report_paths_accept_only_uuid_characters() {
        let console = TestConsole::new(PathBuf::from("test-runs"));
        assert!(
            console
                .path_for("123e4567-e89b-42d3-a456-426614174000")
                .is_ok()
        );
        assert!(console.path_for("../escape").is_err());
        assert!(console.path_for("").is_err());
    }
}
