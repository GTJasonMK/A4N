use super::*;

impl AppState {
    /// 在 Host 退出前停止其拥有的 managed sidecar；可重复调用。
    pub async fn shutdown_managed_sidecars(&self) -> Result<(), String> {
        let _lifecycle = self.managed_sidecar_lifecycle.lock().await;
        self.shutdown_agent_worker().await;
        let processes = std::mem::take(&mut *self.managed_sidecars.lock().await);
        let results =
            futures::future::join_all(processes.into_iter().map(|mut process| async move {
                let result = process.stop_in_place().await;
                (process, result)
            }))
            .await;
        let mut failed_processes = Vec::new();
        let mut failures = Vec::new();
        for (process, result) in results {
            if let Err(error) = result {
                failures.push(error.to_string());
                failed_processes.push(process);
            }
        }
        if !failed_processes.is_empty() {
            self.managed_sidecars.lock().await.extend(failed_processes);
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "停止 managed sidecars 失败: {}",
                failures.join("; ")
            ))
        }
    }

    pub async fn recover_cleanup_outbox(&self) {
        crate::cleanup_outbox::recover_cleanup_outbox(self).await;
    }

    pub(crate) async fn stop_managed_sidecars_for_services(
        &self,
        service_names: &[String],
    ) -> Result<(), String> {
        let _lifecycle = self.managed_sidecar_lifecycle.lock().await;
        if service_names.is_empty() {
            return Ok(());
        }
        let processes = {
            let mut managed = self.managed_sidecars.lock().await;
            let mut selected = Vec::new();
            let mut retained = Vec::with_capacity(managed.len());
            for process in std::mem::take(&mut *managed) {
                if service_names
                    .iter()
                    .any(|service| service == process.service_name())
                {
                    selected.push(process);
                } else {
                    retained.push(process);
                }
            }
            *managed = retained;
            selected
        };
        let mut failed_processes = Vec::new();
        let mut failures = Vec::new();
        for mut process in processes {
            if let Err(error) = process.stop_in_place().await {
                failures.push(error.to_string());
                failed_processes.push(process);
            }
        }
        if !failed_processes.is_empty() {
            self.managed_sidecars.lock().await.extend(failed_processes);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "停止功能包 managed sidecars 失败: {}",
                failures.join("; ")
            ))
        }
    }

    pub(crate) async fn start_managed_sidecars_for_services(
        &self,
        service_names: &[String],
    ) -> Result<(), String> {
        let _lifecycle = self.managed_sidecar_lifecycle.lock().await;
        let mut started = Vec::new();
        for service_name in service_names {
            let config = self
                .service_manager
                .config(service_name)
                .map_err(|error| error.to_string())?;
            if config.mode != a4n_service_manager::ServiceMode::Sidecar {
                continue;
            }
            let status = self
                .service_manager
                .process_status(service_name)
                .await
                .map_err(|error| error.to_string())?;
            let owner_exists = self
                .managed_sidecars
                .lock()
                .await
                .iter()
                .any(|process| process.service_name() == service_name);
            match status.as_ref().map(|status| status.state) {
                None
                | Some(
                    a4n_service_manager::ManagedSidecarState::Stopped
                    | a4n_service_manager::ManagedSidecarState::CircuitOpen,
                ) => {}
                Some(
                    a4n_service_manager::ManagedSidecarState::Starting
                    | a4n_service_manager::ManagedSidecarState::Running
                    | a4n_service_manager::ManagedSidecarState::BackingOff,
                ) if owner_exists => continue,
                Some(a4n_service_manager::ManagedSidecarState::Stopping) => {
                    return Err(format!("service {service_name} 正在停止，不能并发启动"));
                }
                Some(_) => {
                    return Err(format!(
                        "service {service_name} 已处于 managed 状态但 Host owner 缺失"
                    ));
                }
            }

            // A crashed supervisor can leave its finished process handle in the
            // Host collection. Remove that stale handle before replacing the
            // service-manager status entry.
            let stale = {
                let mut managed = self.managed_sidecars.lock().await;
                let mut stale = Vec::new();
                let mut retained = Vec::with_capacity(managed.len());
                for process in std::mem::take(&mut *managed) {
                    if process.service_name() == service_name {
                        stale.push(process);
                    } else {
                        retained.push(process);
                    }
                }
                *managed = retained;
                stale
            };
            drop(stale);

            match self.service_manager.start_sidecar(service_name).await {
                Ok(process) => started.push(process),
                Err(error) => {
                    // Keep already-started handles owned by Host. A retry can
                    // observe them and only start the missing service.
                    self.managed_sidecars.lock().await.extend(started);
                    return Err(format!("启动功能包 managed sidecar 失败: {error}"));
                }
            }
        }
        if !started.is_empty() {
            self.managed_sidecars.lock().await.extend(started);
        }
        Ok(())
    }

    /// 数据目录（来自 AppConfig）。
    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    pub(crate) async fn execute_media(
        &self,
        input: a4n_media_contract::MediaExecuteInput,
    ) -> Result<a4n_media_contract::MediaExecuteOutput, String> {
        if self.runtime_profile.uses_sidecars() {
            let client = self.capabilities.media.as_ref().ok_or_else(|| {
                "media sidecar capability 未配置，无法执行本地媒体处理".to_string()
            })?;
            let request = pure_transform_typed_service_request(
                a4n_media_contract::MEDIA_FFMPEG_CAPABILITY_VERSION,
                input,
                None,
                None,
            )?;
            let response = client
                .execute(request)
                .await
                .map_err(|error| error.message)?;
            return typed_response_payload(response, "media");
        }

        let data_dir = self.config.data_dir.clone();
        let temp_dir = self.config.temp_dir.clone();
        tokio::task::spawn_blocking(move || {
            let roots = a4n_media_processing_core::MediaRoots::new(&data_dir, &temp_dir)?;
            a4n_media_processing_core::execute(&roots, input)
        })
        .await
        .map_err(|error| format!("本地媒体处理任务异常终止：{error}"))?
    }

    pub(crate) fn data_media_file_ref(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<a4n_media_contract::MediaFileRef, String> {
        self.media_file_ref(path, a4n_media_contract::MediaRoot::Data, label)
    }

    pub(crate) fn temp_media_file_ref(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<a4n_media_contract::MediaFileRef, String> {
        self.media_file_ref(path, a4n_media_contract::MediaRoot::Temp, label)
    }

    pub(crate) async fn probe_media_video_properties(
        &self,
        path: &Path,
        root: a4n_media_contract::MediaRoot,
        label: &str,
    ) -> Result<a4n_media_processing_core::VideoProbeProperties, String> {
        let input = self.media_file_ref(path, root, label)?;
        let output = self
            .execute_media(a4n_media_contract::MediaExecuteInput::ProbeVideoProperties { input })
            .await?;
        match output {
            a4n_media_contract::MediaExecuteOutput::VideoProbeProperties {
                width,
                height,
                duration_secs,
                fps,
                codec_name,
                profile,
                has_b_frames,
            } => Ok(a4n_media_processing_core::VideoProbeProperties {
                width,
                height,
                duration_secs,
                fps,
                codec_name,
                profile,
                has_b_frames,
            }),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    pub(crate) async fn probe_media_has_audio(
        &self,
        path: &Path,
        root: a4n_media_contract::MediaRoot,
        label: &str,
    ) -> Result<bool, String> {
        let input = self.media_file_ref(path, root, label)?;
        let output = self
            .execute_media(a4n_media_contract::MediaExecuteInput::ProbeVideoHasAudio { input })
            .await?;
        match output {
            a4n_media_contract::MediaExecuteOutput::HasAudio { has_audio } => Ok(has_audio),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    pub(crate) async fn render_media_scrub_proxy(
        &self,
        input_path: &Path,
        output_path: &Path,
        include_audio: bool,
    ) -> Result<(), String> {
        let input = self.data_media_file_ref(input_path, "剪辑源视频")?;
        let output = self.temp_media_file_ref(output_path, "剪辑输出媒体")?;
        let request = a4n_media_contract::MediaExecuteInput::GenerateVideoScrubProxy {
            input,
            output,
            include_audio,
        };
        match self.execute_media(request).await? {
            a4n_media_contract::MediaExecuteOutput::Unit => Ok(()),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    pub(crate) async fn render_media_scrub_frames(
        &self,
        input_path: &Path,
        output_path: &Path,
    ) -> Result<(), String> {
        let input = self.data_media_file_ref(input_path, "剪辑源视频")?;
        let output = self.temp_media_file_ref(output_path, "剪辑输出媒体")?;
        let request =
            a4n_media_contract::MediaExecuteInput::GenerateVideoScrubFrames { input, output };
        match self.execute_media(request).await? {
            a4n_media_contract::MediaExecuteOutput::Unit => Ok(()),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    fn media_file_ref(
        &self,
        path: &Path,
        root: a4n_media_contract::MediaRoot,
        label: &str,
    ) -> Result<a4n_media_contract::MediaFileRef, String> {
        let root_path = match root {
            a4n_media_contract::MediaRoot::Data => &self.config.data_dir,
            a4n_media_contract::MediaRoot::Temp => &self.config.temp_dir,
        };
        let root_path = root_path
            .canonicalize()
            .map_err(|error| format!("无法解析{label}根目录：{error}"))?;
        let resolved = if path.exists() {
            path.canonicalize()
                .map_err(|error| format!("无法解析{label}：{error}"))?
        } else {
            let parent = path
                .parent()
                .ok_or_else(|| format!("{label}路径缺少父目录"))?
                .canonicalize()
                .map_err(|error| format!("无法解析{label}目录：{error}"))?;
            parent.join(
                path.file_name()
                    .ok_or_else(|| format!("{label}路径缺少文件名"))?,
            )
        };
        let relative = resolved
            .strip_prefix(&root_path)
            .map_err(|_| format!("{label}不在允许的本地目录内"))?;
        a4n_media_contract::MediaFileRef::new(root, relative.to_string_lossy().replace('\\', "/"))
    }
}
