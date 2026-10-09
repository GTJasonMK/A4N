use std::{
    collections::{HashMap, HashSet},
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Stdio,
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
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::{Mutex, mpsc},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    AppState,
    actions::{app_error_message, project_duplication, project_records},
};

const SCENARIO_VERSION: u32 = 1;
const PROJECT_TITLE_MAX_LENGTH: usize = 120;
const PROJECT_DESCRIPTION_MAX_LENGTH: usize = 2000;
const REPOSITORY_TEST_PREFIX: &str = "repository_test_";
const MATRIX_REPORT_SUFFIX: &str = "-matrix";
const MATRIX_CLEANUP_GRACE: Duration = Duration::from_secs(70);
const MATRIX_SUITE_GRACE_SECONDS: u64 = 60;
const MATRIX_RUNNER_GRACE_SECONDS: u64 = 60;
const DEFAULT_SCENARIO_TIMEOUT_SECONDS: u64 = 300;
const TEST_MATRIX_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../scripts/tests/test_matrix.json"
));

#[derive(Debug, serde::Deserialize)]
struct TestMatrixCatalog {
    suites: HashMap<String, TestMatrixSuite>,
    test_center: TestCenterCatalog,
}

#[derive(Debug, serde::Deserialize)]
struct TestMatrixSuite {
    timeout_seconds: u64,
}

#[derive(Debug, serde::Deserialize)]
struct TestCenterCatalog {
    functional_suites: Vec<String>,
    groups: Vec<TestCenterGroup>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TestCenterGroup {
    id: String,
    category: String,
    title: String,
    description: String,
    suites: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TestMatrixRunReport {
    success: bool,
    suites: HashMap<String, TestMatrixSuiteResult>,
}

#[derive(Debug, serde::Deserialize)]
struct TestMatrixSuiteResult {
    status: String,
    exit_code: Option<i32>,
    duration_seconds: Option<f64>,
    error: Option<String>,
    log: Option<String>,
}

#[derive(Default)]
struct ControlState {
    active_run_id: Option<String>,
    cancellation: Option<CancellationToken>,
    deleting_run_ids: HashSet<String>,
}

#[derive(Clone)]
pub struct TestConsole {
    root: Arc<PathBuf>,
    workspace_root: Option<Arc<PathBuf>>,
    control: Arc<Mutex<ControlState>>,
    runs: Arc<Mutex<HashMap<String, TestRunDetail>>>,
    mutation_lock: Arc<Mutex<()>>,
}

impl TestConsole {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Arc::new(root),
            workspace_root: discover_workspace_root().map(Arc::new),
            control: Arc::new(Mutex::new(ControlState::default())),
            runs: Arc::new(Mutex::new(HashMap::new())),
            mutation_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn scenarios(&self) -> Result<Vec<TestScenarioDescriptor>, String> {
        let mut scenarios = vec![
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
                        "title":{"type":"string","minLength":1,"maxLength":PROJECT_TITLE_MAX_LENGTH},
                        "description":{"type":"string","maxLength":PROJECT_DESCRIPTION_MAX_LENGTH}
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
        ];
        let catalog = test_matrix_catalog()?;
        for group in catalog.test_center.groups {
            let timeout_seconds =
                group
                    .suites
                    .iter()
                    .fold(DEFAULT_SCENARIO_TIMEOUT_SECONDS, |total, suite| {
                        total
                            .saturating_add(catalog.suites[suite].timeout_seconds)
                            .saturating_add(MATRIX_SUITE_GRACE_SECONDS)
                    });
            let mut descriptor = scenario(
                &format!("{REPOSITORY_TEST_PREFIX}{}", group.id),
                &group.category,
                &group.title,
                &group.description,
                TestScenarioEffect::Test,
                vec![TestTargetMode::Repository],
                json!({"type":"object","properties":{},"additionalProperties":false}),
            );
            descriptor.timeout_seconds = timeout_seconds
                .saturating_add(MATRIX_CLEANUP_GRACE.as_secs())
                .saturating_add(MATRIX_RUNNER_GRACE_SECONDS);
            scenarios.push(descriptor);
        }
        Ok(scenarios)
    }

    pub async fn start(
        &self,
        state: AppState,
        user_id: i64,
        mut input: StartTestRunInput,
    ) -> Result<TestRunSummary, String> {
        let scenario = self
            .scenarios()?
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

        let scenario_timeout = scenario.timeout_seconds;
        let is_repository_test = input.scenario_id.starts_with(REPOSITORY_TEST_PREFIX);
        if is_repository_test != matches!(input.target, TestTarget::Repository) {
            return Err("项目测试集必须使用仓库测试目标".to_string());
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
        let _guard = self.mutation_lock.lock().await;
        {
            let mut control = self.control.lock().await;
            if let Some(active) = control.active_run_id.as_ref() {
                return Err(format!("已有测试正在运行：{active}"));
            }
            control.active_run_id = Some(run_id.clone());
            control.cancellation = Some(cancellation.clone());
        }
        self.runs
            .lock()
            .await
            .insert(run_id.clone(), detail.clone());
        if let Err(error) = self.persist(&detail).await {
            self.runs.lock().await.remove(&run_id);
            let mut control = self.control.lock().await;
            if control.active_run_id.as_deref() == Some(run_id.as_str()) {
                control.active_run_id = None;
                control.cancellation = None;
            }
            return Err(error);
        }
        drop(_guard);

        let manager = self.clone();
        tokio::spawn(async move {
            manager
                .run(
                    state,
                    user_id,
                    run_id,
                    input,
                    scenario_timeout,
                    cancellation,
                )
                .await;
        });
        Ok(detail.summary)
    }

    pub async fn get(&self, run_id: &str) -> Result<TestRunDetail, String> {
        let _guard = self.mutation_lock.lock().await;
        self.get_unlocked(run_id).await
    }

    async fn get_unlocked(&self, run_id: &str) -> Result<TestRunDetail, String> {
        if let Some(detail) = self.runs.lock().await.get(run_id).cloned() {
            return Ok(detail);
        }
        let path = self.path_for(run_id)?;
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|error| format!("读取测试运行失败：{error}"))?;
        let mut detail: TestRunDetail =
            serde_json::from_slice(&bytes).map_err(|error| format!("测试运行报告损坏：{error}"))?;
        let is_active =
            self.control.lock().await.active_run_id.as_deref() == Some(detail.summary.id.as_str());
        if !is_active {
            self.recover_interrupted(&mut detail).await?;
        }
        Ok(detail)
    }

    pub async fn list(&self, input: ListTestRunsInput) -> Result<Vec<TestRunSummary>, String> {
        let _guard = self.mutation_lock.lock().await;
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
                let cached = self.runs.lock().await.get(&detail.summary.id).cloned();
                let active = self.control.lock().await.active_run_id.clone();
                if let Some(cached) = cached {
                    detail = cached;
                } else if active.as_deref() != Some(detail.summary.id.as_str()) {
                    self.recover_interrupted(&mut detail).await?;
                }
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
        let token = {
            let control = self.control.lock().await;
            (control.active_run_id.as_deref() == Some(run_id))
                .then(|| control.cancellation.clone())
                .flatten()
        };
        if let Some(token) = token {
            token.cancel();
        }
        Ok(self.get(run_id).await?.summary)
    }

    pub async fn delete(&self, state: &AppState, input: DeleteTestRunInput) -> Result<(), String> {
        {
            let mut control = self.control.lock().await;
            if control.active_run_id.as_deref() == Some(input.run_id.as_str()) {
                return Err("运行中的测试不能删除".to_string());
            }
            if !control.deleting_run_ids.insert(input.run_id.clone()) {
                return Err("该测试运行正在删除".to_string());
            }
        }
        let result = async {
            let detail = self.get(&input.run_id).await?;
            if input.delete_test_project {
                let project_ids = created_project_ids(&detail);
                if project_ids.is_empty() {
                    return Err("该运行没有由测试中心创建的项目".to_string());
                }
                for project_id in project_ids.into_iter().rev() {
                    // Created projects belong to the run creator; the deleting admin may differ.
                    project_records::delete_project(state, detail.summary.created_by, project_id)
                        .await?;
                }
            }
            let _guard = self.mutation_lock.lock().await;
            if matches!(&detail.summary.target, TestTarget::Repository) {
                let report_dir = self.matrix_report_dir(&input.run_id)?;
                match tokio::fs::remove_dir_all(report_dir).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => return Err(format!("删除项目测试报告目录失败：{error}")),
                }
            }
            let path = self.path_for(&input.run_id)?;
            let remove_result = match tokio::fs::remove_file(path).await {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("删除测试历史失败：{error}")),
            };
            if remove_result.is_ok() {
                self.runs.lock().await.remove(&input.run_id);
            }
            remove_result
        }
        .await;
        self.control
            .lock()
            .await
            .deleting_run_ids
            .remove(&input.run_id);
        result
    }

    async fn run(
        &self,
        state: AppState,
        user_id: i64,
        run_id: String,
        input: StartTestRunInput,
        scenario_timeout: u64,
        cancellation: CancellationToken,
    ) {
        let started = Instant::now();
        if let Err(error) = self
            .mutate(&run_id, |detail| {
                detail.summary.started_at = Some(now());
                detail.summary.status = TestRunStatus::Running;
            })
            .await
        {
            tracing::error!(run_id = %run_id, %error, "测试运行状态无法切换为运行中");
            self.runs.lock().await.remove(&run_id);
            let mut control = self.control.lock().await;
            if control.active_run_id.as_deref() == Some(run_id.as_str()) {
                control.active_run_id = None;
                control.cancellation = None;
            }
            return;
        }
        let scenario_future = self.run_scenario(
            &state,
            user_id,
            &run_id,
            &input,
            scenario_timeout,
            &cancellation,
        );
        let outer_cancellation_enabled = !matches!(input.target, TestTarget::Repository);
        let result = tokio::select! {
            _ = cancellation.cancelled(), if outer_cancellation_enabled => {
                Err((TestRunStatus::Cancelled, "测试已取消".to_string()))
            }
            result = tokio::time::timeout(
                Duration::from_secs(scenario_timeout),
                scenario_future,
            ) => match result {
                Ok(Ok(())) if cancellation.is_cancelled() => {
                    Err((TestRunStatus::Cancelled, "测试已取消".to_string()))
                }
                Ok(Ok(())) => Ok(()),
                Ok(Err(_)) if cancellation.is_cancelled() => {
                    Err((TestRunStatus::Cancelled, "测试已取消".to_string()))
                }
                Ok(Err(error)) => Err((TestRunStatus::Failed, error)),
                Err(_) if cancellation.is_cancelled() => {
                    Err((TestRunStatus::Cancelled, "测试已取消".to_string()))
                }
                Err(_) => Err((
                    TestRunStatus::TimedOut,
                    format!("测试超过 {scenario_timeout} 秒限制"),
                )),
            }
        };
        let (status, message) = match result {
            Ok(()) => (TestRunStatus::Passed, Some("测试通过".to_string())),
            Err((status, error)) => (status, Some(error)),
        };
        if let Err(error) = self
            .mutate(&run_id, |detail| {
                detail.summary.status = status.clone();
                detail.summary.message = message;
                detail.summary.finished_at = Some(now());
                detail.summary.duration_ms = Some(started.elapsed().as_millis() as u64);
                for step in &mut detail.steps {
                    if matches!(step.status, TestRunStatus::Queued | TestRunStatus::Running) {
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
            .await
        {
            tracing::error!(run_id = %run_id, %error, "测试运行终态无法持久化");
        }
        self.runs.lock().await.remove(&run_id);
        let mut control = self.control.lock().await;
        if control.active_run_id.as_deref() == Some(run_id.as_str()) {
            control.active_run_id = None;
            control.cancellation = None;
        }
    }

    async fn run_scenario(
        &self,
        state: &AppState,
        user_id: i64,
        run_id: &str,
        input: &StartTestRunInput,
        scenario_timeout: u64,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        if let Some(group_id) = input.scenario_id.strip_prefix(REPOSITORY_TEST_PREFIX) {
            self.run_repository_test_group(
                run_id,
                group_id,
                cancellation,
                scenario_timeout
                    .saturating_sub(MATRIX_CLEANUP_GRACE.as_secs())
                    .saturating_sub(MATRIX_RUNNER_GRACE_SECONDS),
            )
            .await?;
        } else {
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
                    self.step(run_id, "重新读取项目", || async {
                        state
                            .project_read_application()
                            .summary(user_id, target)
                            .await
                            .map_err(app_error_message)
                            .map(|_| ())
                    })
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
                        TestTarget::Repository => {
                            return Err("仓库测试目标不能用于项目复制场景".to_string());
                        }
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
                    self.record_created_project(
                        state,
                        user_id,
                        run_id,
                        copied.id,
                        format!("复制源项目 {source}"),
                    )
                    .await?;
                    self.step(run_id, "读取复制结果", || async {
                        state
                            .project_read_application()
                            .summary(user_id, copied.id)
                            .await
                            .map_err(app_error_message)
                            .map(|_| ())
                    })
                    .await?;
                }
                "project_status_read" => {
                    let target = ensure_project(state, user_id, input, run_id, self).await?;
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
        }
        if cancellation.is_cancelled() {
            return Err("测试已取消".to_string());
        }
        Ok(())
    }

    async fn run_repository_test_group(
        &self,
        run_id: &str,
        group_id: &str,
        cancellation: &CancellationToken,
        timeout_seconds: u64,
    ) -> Result<(), String> {
        let workspace_root = self
            .workspace_root
            .as_deref()
            .ok_or_else(|| "当前应用没有可访问的项目测试源码".to_string())?;
        let catalog = test_matrix_catalog()?;
        let group = catalog
            .test_center
            .groups
            .into_iter()
            .find(|group| group.id == group_id)
            .ok_or_else(|| format!("未知项目测试分组：{group_id}"))?;
        let report_dir = self.matrix_report_dir(run_id)?;
        let python = if cfg!(windows) { "python" } else { "python3" };
        let mut command = Command::new(python);
        command
            .arg("scripts/tests/run_test_matrix.py")
            .arg("--report-dir")
            .arg(&report_dir);
        for suite in &group.suites {
            command.arg("--suite").arg(suite);
        }
        command
            .current_dir(workspace_root.as_path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().process_group(0);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("启动项目测试矩阵失败：{error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "项目测试矩阵 stdout 不可用".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "项目测试矩阵 stderr 不可用".to_string())?;
        let (sender, mut receiver) = mpsc::channel::<Result<String, String>>(64);
        let stdout_forwarder = tokio::spawn(forward_lines(stdout, sender.clone()));
        let stderr_forwarder = tokio::spawn(forward_lines(stderr, sender.clone()));
        drop(sender);

        let mut suite_steps = HashMap::new();
        let mut finished_suites = HashSet::new();
        let mut output_open = true;
        let mut exit_status = None;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_seconds);
        let mut timed_out = false;
        let runner_result: Result<(), String> = async {
            while exit_status.is_none() {
                tokio::select! {
                    status = child.wait() => {
                        exit_status = Some(status.map_err(|error| format!("读取测试矩阵退出状态失败：{error}"))?);
                    }
                    line = receiver.recv(), if output_open => match line {
                        Some(Ok(line)) => self.record_matrix_event(
                            run_id,
                            &group,
                            &line,
                            &mut suite_steps,
                            &mut finished_suites,
                        ).await?,
                        Some(Err(error)) => return Err(error),
                        None => output_open = false,
                    },
                    _ = cancellation.cancelled() => {
                        self.append_log(run_id, "正在停止当前测试并清理其子进程".to_string()).await?;
                        terminate_test_runner(&mut child).await?;
                        exit_status = Some(child.wait().await.map_err(|error| {
                            format!("等待已取消的测试矩阵退出失败：{error}")
                        })?);
                    }
                    _ = tokio::time::sleep_until(deadline) => {
                        timed_out = true;
                        self.append_log(run_id, "测试矩阵达到运行时限，正在停止当前 suite 并清理子进程".to_string()).await?;
                        terminate_test_runner(&mut child).await?;
                        exit_status = Some(child.wait().await.map_err(|error| {
                            format!("等待超时后的测试矩阵退出失败：{error}")
                        })?);
                    }
                }
            }
            Ok(())
        }
        .await;

        let cleanup_error = if runner_result.is_err() && exit_status.is_none() {
            terminate_test_runner(&mut child).await.err()
        } else {
            None
        };
        stdout_forwarder.abort();
        stderr_forwarder.abort();
        let _ = stdout_forwarder.await;
        let _ = stderr_forwarder.await;
        if let Err(error) = runner_result {
            return Err(match cleanup_error {
                Some(cleanup_error) => {
                    format!("{error}；停止失败的测试 runner 也失败：{cleanup_error}")
                }
                None => error,
            });
        }

        if cancellation.is_cancelled() {
            return Err("测试已取消".to_string());
        }
        if timed_out {
            return Err(format!(
                "项目测试分组超过 {timeout_seconds} 秒运行时限；报告目录：{}",
                report_dir.display()
            ));
        }
        let exit_status = exit_status.ok_or_else(|| "项目测试矩阵没有退出状态".to_string())?;
        let report_path = report_dir.join("matrix.json");
        let report_bytes = tokio::fs::read(&report_path).await.map_err(|error| {
            format!(
                "读取测试矩阵报告失败（退出码 {}）：{error}",
                exit_status.code().unwrap_or(-1)
            )
        })?;
        let report: TestMatrixRunReport = serde_json::from_slice(&report_bytes)
            .map_err(|error| format!("测试矩阵报告格式无效：{error}"))?;

        let mut failed_suites = Vec::new();
        for suite in &group.suites {
            let result = report
                .suites
                .get(suite)
                .ok_or_else(|| format!("测试矩阵报告缺少 suite：{suite}"))?;
            let index = match suite_steps.get(suite).copied() {
                Some(index) => index,
                None => {
                    let index = self.begin_step(run_id, suite).await?;
                    suite_steps.insert(suite.clone(), index);
                    index
                }
            };
            let status = matrix_suite_status(&result.status);
            let message = matrix_suite_message(result, &report_dir).await?;
            if finished_suites.contains(suite) {
                self.update_step_summary(run_id, index, status.clone(), message)
                    .await?;
            } else {
                self.finish_step(run_id, index, status.clone(), Some(message))
                    .await?;
            }
            if status != TestRunStatus::Passed {
                failed_suites.push(suite.clone());
            }
        }

        if report.success && failed_suites.is_empty() && exit_status.success() {
            self.append_log(
                run_id,
                format!(
                    "全部 {} 个 suite 通过；报告：{}",
                    group.suites.len(),
                    report_path.display()
                ),
            )
            .await?;
            Ok(())
        } else {
            let failure_summary = if failed_suites.is_empty() {
                format!("测试矩阵退出码 {}", exit_status.code().unwrap_or(-1))
            } else {
                format!("失败 suite：{}", failed_suites.join("、"))
            };
            Err(format!(
                "{failure_summary}；详细日志目录：{}",
                report_dir.display()
            ))
        }
    }

    async fn record_matrix_event(
        &self,
        run_id: &str,
        group: &TestCenterGroup,
        line: &str,
        suite_steps: &mut HashMap<String, u32>,
        finished_suites: &mut HashSet<String>,
    ) -> Result<(), String> {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            return Ok(());
        };
        let (Some(suite), Some(status)) = (
            event.get("suite").and_then(Value::as_str),
            event.get("status").and_then(Value::as_str),
        ) else {
            return Ok(());
        };
        if !group.suites.iter().any(|registered| registered == suite) {
            return Err(format!("测试矩阵报告了未登记的 suite：{suite}"));
        }
        if status == "running" {
            if !suite_steps.contains_key(suite) {
                let index = self.begin_step(run_id, suite).await?;
                suite_steps.insert(suite.to_string(), index);
            }
            return Ok(());
        }
        let status = matrix_suite_status(status);
        let index = match suite_steps.get(suite).copied() {
            Some(index) => index,
            None => {
                let index = self.begin_step(run_id, suite).await?;
                suite_steps.insert(suite.to_string(), index);
                index
            }
        };
        self.finish_step(
            run_id,
            index,
            status.clone(),
            Some(status_label(&status).to_string()),
        )
        .await?;
        finished_suites.insert(suite.to_string());
        Ok(())
    }

    async fn append_log(&self, run_id: &str, message: String) -> Result<(), String> {
        let message = message.chars().take(2000).collect::<String>();
        self.mutate(run_id, |detail| {
            if detail.logs.len() >= 500 {
                detail.logs.remove(0);
            }
            detail.logs.push(message);
        })
        .await
        .map(|_| ())
    }

    async fn update_step_summary(
        &self,
        run_id: &str,
        index: u32,
        status: TestRunStatus,
        message: String,
    ) -> Result<(), String> {
        self.mutate(run_id, |detail| {
            if let Some(step) = detail.steps.iter_mut().find(|step| step.index == index) {
                step.status = status;
                step.message = Some(message);
            }
        })
        .await
        .map(|_| ())
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

    async fn record_created_project(
        &self,
        state: &AppState,
        user_id: i64,
        run_id: &str,
        project_id: i64,
        detail: String,
    ) -> Result<(), String> {
        let result = self
            .mutate(run_id, |run| {
                run.summary.target_project_id = Some(project_id);
                run.summary.impact.push(TestImpact {
                    action: "create".to_string(),
                    resource: "project".to_string(),
                    project_id: Some(project_id),
                    detail,
                });
            })
            .await;
        if let Err(record_error) = result {
            return match project_records::delete_project(state, user_id, project_id).await {
                Ok(()) => Err(record_error),
                Err(cleanup_error) => Err(format!(
                    "{record_error}；清理未能登记的测试项目失败：{cleanup_error}"
                )),
            };
        }
        Ok(())
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
            if matches!(step.status, TestRunStatus::Queued | TestRunStatus::Running) {
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
        let _guard = self.mutation_lock.lock().await;
        let mut detail = self.get_unlocked(run_id).await?;
        change(&mut detail);
        self.persist(&detail).await?;
        self.runs
            .lock()
            .await
            .insert(run_id.to_string(), detail.clone());
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
        if let Err(error) = tokio::fs::write(&temporary, bytes).await {
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(format!("写入测试运行失败：{error}"));
        }
        if let Err(error) = tokio::fs::rename(&temporary, &path).await {
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(format!("提交测试运行失败：{error}"));
        }
        Ok(())
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

    fn matrix_report_dir(&self, run_id: &str) -> Result<PathBuf, String> {
        self.path_for(run_id)?;
        Ok(self.root.join(format!("{run_id}{MATRIX_REPORT_SUFFIX}")))
    }
}

fn discover_workspace_root() -> Option<PathBuf> {
    let candidates = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf)),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
    ];
    for candidate in candidates.into_iter().flatten() {
        for ancestor in candidate.ancestors() {
            if ancestor.join("scripts/tests/run_test_matrix.py").is_file()
                && ancestor.join("scripts/tests/test_matrix.json").is_file()
            {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn test_matrix_catalog() -> Result<TestMatrixCatalog, String> {
    let catalog: TestMatrixCatalog = serde_json::from_str(TEST_MATRIX_SOURCE)
        .map_err(|error| format!("项目测试矩阵格式无效：{error}"))?;
    let functional_suites = &catalog.test_center.functional_suites;
    if functional_suites.is_empty() {
        return Err("项目测试矩阵没有功能 suite".to_string());
    }
    let functional_set = functional_suites.iter().collect::<HashSet<_>>();
    if functional_set.len() != functional_suites.len() {
        return Err("项目测试矩阵的功能 suite 重复".to_string());
    }

    let mut group_ids = HashSet::new();
    let mut grouped_suites = HashSet::new();
    let mut all_functional_suites = None;
    for group in &catalog.test_center.groups {
        if group.id.is_empty()
            || !group
                .id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(format!("项目测试分组 ID 无效：{}", group.id));
        }
        if !group_ids.insert(group.id.as_str()) {
            return Err(format!("项目测试分组 ID 重复：{}", group.id));
        }
        if group.category.trim().is_empty()
            || group.title.trim().is_empty()
            || group.description.trim().is_empty()
            || group.suites.is_empty()
        {
            return Err(format!("项目测试分组定义不完整：{}", group.id));
        }
        let suite_set = group.suites.iter().collect::<HashSet<_>>();
        if suite_set.len() != group.suites.len() {
            return Err(format!("项目测试分组 suite 重复：{}", group.id));
        }
        for suite in &group.suites {
            if !catalog.suites.contains_key(suite) {
                return Err(format!("项目测试分组引用了未知 suite：{suite}"));
            }
            if catalog.suites[suite].timeout_seconds == 0 {
                return Err(format!("项目测试 suite 超时无效：{suite}"));
            }
        }
        if group.id == "all-functional" {
            all_functional_suites = Some(&group.suites);
        } else {
            grouped_suites.extend(group.suites.iter());
        }
    }
    if grouped_suites != functional_set {
        return Err("项目测试分组未覆盖全部登记的功能 suite".to_string());
    }
    if all_functional_suites != Some(functional_suites) {
        return Err("全部功能测试分组与功能 suite 清单不一致".to_string());
    }
    Ok(catalog)
}

fn matrix_suite_status(status: &str) -> TestRunStatus {
    match status {
        "passed" => TestRunStatus::Passed,
        "timeout" => TestRunStatus::TimedOut,
        "interrupted" | "cancelled" => TestRunStatus::Cancelled,
        _ => TestRunStatus::Failed,
    }
}

async fn matrix_suite_message(
    result: &TestMatrixSuiteResult,
    report_dir: &Path,
) -> Result<String, String> {
    let mut parts = Vec::new();
    if let Some(seconds) = result.duration_seconds {
        parts.push(format!("{seconds:.1} 秒"));
    }
    if let Some(exit_code) = result.exit_code {
        parts.push(format!("退出码 {exit_code}"));
    }
    if let Some(error) = result.error.as_deref().filter(|error| !error.is_empty()) {
        parts.push(error.chars().take(240).collect());
    }
    if let Some(log) = result.log.as_deref() {
        if result.status != "passed" {
            let log_path = Path::new(log);
            let report_root = tokio::fs::canonicalize(report_dir)
                .await
                .map_err(|error| format!("解析测试报告目录失败：{error}"))?;
            let log_path = if log_path.is_absolute() {
                log_path.to_path_buf()
            } else {
                report_dir.join(log_path)
            };
            let log_path = tokio::fs::canonicalize(log_path)
                .await
                .map_err(|error| format!("读取 suite 失败日志路径失败：{error}"))?;
            if !log_path.starts_with(&report_root) {
                return Err("测试 suite 日志路径越出报告目录".to_string());
            }
            let visible_path = log_path.strip_prefix(&report_root).unwrap_or(&log_path);
            parts.push(format!("失败日志：{}", visible_path.display()));
        }
    }
    if parts.is_empty() {
        Ok(status_label(&matrix_suite_status(&result.status)).to_string())
    } else {
        Ok(parts.join("；").chars().take(2000).collect())
    }
}

async fn forward_lines<R>(reader: R, sender: mpsc::Sender<Result<String, String>>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if sender.send(Ok(line)).await.is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                let _ = sender
                    .send(Err(format!("读取测试矩阵输出失败：{error}")))
                    .await;
                break;
            }
        }
    }
}

async fn terminate_test_runner(child: &mut tokio::process::Child) -> Result<(), String> {
    let mut group_cleanup_error: Option<String> = None;
    if let Some(pid) = child.id() {
        #[cfg(unix)]
        {
            // SAFETY: the runner is started in its own process group below.
            let result = unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGTERM) };
            if result != 0 && std::io::Error::last_os_error().kind() != ErrorKind::NotFound {
                let group_error = std::io::Error::last_os_error();
                if let Err(child_error) = child.kill().await {
                    return Err(format!(
                        "请求测试 runner 清理子进程失败：{group_error}；直接停止 runner 失败：{child_error}"
                    ));
                }
                group_cleanup_error = Some(group_error.to_string());
            }
        }
        #[cfg(windows)]
        {
            let pid = pid.to_string();
            let _ = Command::new("taskkill")
                .args(["/PID", pid.as_str(), "/T", "/F"])
                .status()
                .await;
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = pid;
            child
                .kill()
                .await
                .map_err(|error| format!("停止测试 runner 失败：{error}"))?;
        }
    }

    match tokio::time::timeout(MATRIX_CLEANUP_GRACE, child.wait()).await {
        Ok(Ok(_)) => match group_cleanup_error {
            Some(error) => Err(format!("请求测试 runner 清理子进程失败：{error}")),
            None => Ok(()),
        },
        Ok(Err(error)) => Err(format!("等待测试 runner 退出失败：{error}")),
        Err(_) => {
            child
                .kill()
                .await
                .map_err(|error| format!("强制停止测试 runner 失败：{error}"))?;
            child
                .wait()
                .await
                .map_err(|error| format!("等待测试 runner 清理失败：{error}"))?;
            Ok(())
        }
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
        timeout_seconds: DEFAULT_SCENARIO_TIMEOUT_SECONDS,
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
        TestTarget::Repository => TestTargetMode::Repository,
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
        TestTarget::TestProject { .. } | TestTarget::Repository => None,
    }
}

fn created_project_ids(detail: &TestRunDetail) -> Vec<i64> {
    let mut project_ids = detail
        .summary
        .impact
        .iter()
        .filter(|impact| impact.action == "create" && impact.resource == "project")
        .filter_map(|impact| impact.project_id)
        .collect::<Vec<_>>();
    project_ids.sort_unstable();
    project_ids.dedup();
    project_ids
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
                console
                    .record_created_project(
                        state,
                        user_id,
                        run_id,
                        copied.id,
                        format!("从项目 {source_project_id} 创建测试副本"),
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
                console
                    .record_created_project(
                        state,
                        user_id,
                        run_id,
                        created.id,
                        "创建独立测试项目".to_string(),
                    )
                    .await?;
                Ok(created.id)
            }
        }
        TestTarget::Repository => Err("仓库测试目标不能创建或更新项目".to_string()),
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
    if scenario.id.starts_with(REPOSITORY_TEST_PREFIX) {
        return if object.is_empty() {
            Ok(())
        } else {
            Err("仓库测试分组不接受自定义参数".to_string())
        };
    }
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
            let valid = title.as_str().map(str::trim).is_some_and(|value| {
                !value.is_empty() && value.chars().count() <= PROJECT_TITLE_MAX_LENGTH
            });
            if !valid {
                return Err(format!(
                    "项目标题必须为 1 到 {PROJECT_TITLE_MAX_LENGTH} 个字符"
                ));
            }
        }
        if let Some(description) = object.get("description") {
            let Some(value) = description.as_str() else {
                return Err("项目描述必须是字符串".to_string());
            };
            if value.chars().count() > PROJECT_DESCRIPTION_MAX_LENGTH {
                return Err(format!(
                    "项目描述不能超过 {PROJECT_DESCRIPTION_MAX_LENGTH} 个字符"
                ));
            }
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
        let scenarios = TestConsole::new(PathBuf::from("test-runs"))
            .scenarios()
            .expect("registered test catalog");
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
        let scenarios = TestConsole::new(PathBuf::from("test-runs"))
            .scenarios()
            .expect("registered test catalog");
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
        assert!(validate_parameters(mutation, &json!({"description": 42})).is_err());
        assert!(
            validate_parameters(mutation, &json!({"title": "Test", "description": "ok"})).is_ok()
        );
        assert!(validate_parameters(mutation, &Value::Null).is_err());
    }

    #[test]
    fn targets_reject_invalid_ids_and_unsupported_modes() {
        let scenario = TestConsole::new(PathBuf::from("test-runs"))
            .scenarios()
            .expect("registered test catalog")
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
    fn cleanup_uses_projects_created_by_the_run_not_its_input_target() {
        let mut detail = sample_detail(
            "123e4567-e89b-42d3-a456-426614174000",
            TestRunStatus::Passed,
        );
        detail.summary.target = TestTarget::CurrentProject { project_id: 1 };
        detail.summary.target_project_id = Some(2);
        detail.summary.impact = vec![
            TestImpact {
                action: "create".to_string(),
                resource: "project".to_string(),
                project_id: Some(2),
                detail: "测试副本".to_string(),
            },
            TestImpact {
                action: "update".to_string(),
                resource: "project".to_string(),
                project_id: Some(1),
                detail: "源项目".to_string(),
            },
            TestImpact {
                action: "create".to_string(),
                resource: "project".to_string(),
                project_id: Some(2),
                detail: "重复记录".to_string(),
            },
        ];

        assert_eq!(created_project_ids(&detail), vec![2]);
    }

    #[test]
    fn repository_test_groups_use_repository_target_and_no_parameters() {
        let scenarios = TestConsole::new(PathBuf::from("test-runs"))
            .scenarios()
            .expect("registered test catalog");
        let all = scenarios
            .iter()
            .find(|item| item.id == "repository_test_all-functional")
            .expect("all functional group");

        assert_eq!(all.effect, TestScenarioEffect::Test);
        assert_eq!(all.target_modes, vec![TestTargetMode::Repository]);
        assert!(validate_target(all, &TestTarget::Repository).is_ok());
        assert!(validate_parameters(all, &json!({})).is_ok());
        assert!(validate_parameters(all, &json!({"suite": "rust-workspace"})).is_err());
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

    fn sample_detail(id: &str, status: TestRunStatus) -> TestRunDetail {
        TestRunDetail {
            summary: TestRunSummary {
                id: id.to_string(),
                scenario_id: "project_list".to_string(),
                scenario_version: SCENARIO_VERSION,
                status,
                target: TestTarget::TestProject {
                    source_project_id: None,
                },
                target_project_id: None,
                created_by: 1,
                created_at: now(),
                started_at: None,
                finished_at: None,
                duration_ms: None,
                message: None,
                impact: Vec::new(),
            },
            parameters: json!({}),
            steps: Vec::new(),
            logs: Vec::new(),
        }
    }

    #[tokio::test]
    async fn list_does_not_recover_the_active_persisted_run() {
        let root = tempfile::tempdir().expect("create test run directory");
        let console = TestConsole::new(root.path().to_path_buf());
        let detail = sample_detail(
            "123e4567-e89b-42d3-a456-426614174000",
            TestRunStatus::Running,
        );
        console.persist(&detail).await.expect("persist active run");
        {
            let mut control = console.control.lock().await;
            control.active_run_id = Some(detail.summary.id.clone());
            control.cancellation = Some(CancellationToken::new());
        }

        let summaries = console
            .list(ListTestRunsInput { limit: 1 })
            .await
            .expect("list active run");
        assert_eq!(summaries[0].status, TestRunStatus::Running);
    }

    #[tokio::test]
    async fn persist_replaces_an_existing_report() {
        let root = tempfile::tempdir().expect("create test run directory");
        let console = TestConsole::new(root.path().to_path_buf());
        let id = "123e4567-e89b-42d3-a456-426614174000";
        let mut detail = sample_detail(id, TestRunStatus::Queued);
        console
            .persist(&detail)
            .await
            .expect("persist first report");
        detail.summary.status = TestRunStatus::Passed;
        console.persist(&detail).await.expect("replace report");

        let loaded = console.get(id).await.expect("read replaced report");
        assert_eq!(loaded.summary.status, TestRunStatus::Passed);
    }

    #[tokio::test]
    async fn recover_interrupted_finishes_queued_steps() {
        let root = tempfile::tempdir().expect("create test run directory");
        let console = TestConsole::new(root.path().to_path_buf());
        let id = "123e4567-e89b-42d3-a456-426614174001";
        let mut detail = sample_detail(id, TestRunStatus::Queued);
        detail.steps.push(a4n_api_contract::admin::TestStepSummary {
            index: 0,
            name: "未启动步骤".to_string(),
            status: TestRunStatus::Queued,
            started_at: now(),
            finished_at: None,
            message: None,
        });
        console
            .persist(&detail)
            .await
            .expect("persist interrupted report");

        let recovered = console.get(id).await.expect("recover interrupted report");
        assert_eq!(recovered.summary.status, TestRunStatus::Failed);
        assert_eq!(recovered.steps[0].status, TestRunStatus::Failed);
        assert!(recovered.steps[0].finished_at.is_some());
    }
}
