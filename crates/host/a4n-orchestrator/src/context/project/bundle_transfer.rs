use std::collections::{HashMap, HashSet};

use a4n_api_contract::project_transfer::{
    ExportProjectBundleApplicationInput, ExportedProjectPackage,
    ImportProjectPackageApplicationInput, ProjectBundleImportReport,
    ProjectBundleTransferApplicationPort, ProjectPackageTransferError,
    ProjectPackageTransferHostPort,
};
use a4n_db::canvas::{VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter};
use a4n_db::config::{
    list_image_config_records, list_llm_config_records, list_video_config_records,
};
use a4n_db::project::bundle::{
    ProjectBundleEntityIdMaps, ProjectBundleHistoryImportSummary, ProjectBundleHistoryRows,
};
use a4n_db::project::family::import::{
    ProjectFamilyMediaFileImporter, ProjectFamilyMediaImportError,
};
use chrono::Utc;
use sha2::{Digest, Sha256};

use super::super::OrchestratorContext;
use crate::project::{
    ExportedProjectPackage as InternalExportedProjectPackage, ImportMediaPayload,
    PROJECT_BUNDLE_MANIFEST_PATH, ProjectBundlePackage, ProjectFamilyPackage,
    ProjectPackageTransferError as InternalTransferError, assemble_project_bundle,
    assemble_project_family_export_package, assemble_video_studio_project_canvas_package,
    imported_project_family_media_path, index_import_media_payload, media_file_key,
    project_family_export_media_files, project_package_export_input, required_package_extension,
    validate_family_media_manifest, validate_family_package_manifest_with_canvas,
    validate_package_manifest as validate_video_studio_package_manifest,
    video_studio_export_media_files,
};
use a4n_domain::collaboration::ProjectAccessRole;

impl OrchestratorContext {
    async fn export_project_bundle_validated<Host>(
        &self,
        input: ExportProjectBundleApplicationInput,
        host_port: &Host,
    ) -> Result<InternalExportedProjectPackage, InternalTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Owner)
            .await?;
        let project = self
            .project_read
            .by_id(input.project_id)
            .await?
            .ok_or_else(|| {
                InternalTransferError::Message(format!("项目不存在: {}", input.project_id))
            })?;

        let (family, family_project_ids, mut canvas_projects, mut media_files) =
            if project.project_kind == crate::project::PROJECT_KIND_TEXT {
                let selection = self
                    .project_family
                    .export_selection(project.id, input.include_derived_projects)
                    .await?;
                let derived_ids = selection
                    .derived_projects
                    .iter()
                    .map(|project| project.id)
                    .collect::<Vec<_>>();
                let snapshot_ids = selection
                    .derived_projects
                    .iter()
                    .filter_map(|project| project.source_snapshot_id)
                    .collect::<Vec<_>>();
                let rows = self
                    .project_family
                    .export_rows(project.id, &derived_ids, &snapshot_ids)
                    .await?;
                let media = project_family_export_media_files(&rows)?;
                let package = assemble_project_family_export_package(
                    selection.root,
                    selection.derived_projects,
                    "project_bundle_family",
                    rows,
                    media.clone(),
                )?;
                let project_ids = package_project_ids(&package);
                (Some(package), project_ids, Vec::new(), media)
            } else if project.root_project_id.is_some() {
                let selection = self
                    .project_family
                    .derived_export_selection(project.id)
                    .await?;
                let derived_ids = selection
                    .derived_projects
                    .iter()
                    .map(|project| project.id)
                    .collect::<Vec<_>>();
                let snapshot_ids = selection
                    .derived_projects
                    .iter()
                    .filter_map(|project| project.source_snapshot_id)
                    .collect::<Vec<_>>();
                let rows = self
                    .project_family
                    .export_rows(selection.root.id, &derived_ids, &snapshot_ids)
                    .await?;
                let media = project_family_export_media_files(&rows)?;
                let package = assemble_project_family_export_package(
                    selection.root,
                    selection.derived_projects,
                    "project_bundle_derived",
                    rows,
                    media.clone(),
                )?;
                let project_ids = package_project_ids(&package);
                (Some(package), project_ids, Vec::new(), media)
            } else {
                (None, Vec::new(), Vec::new(), Vec::new())
            };

        let canvas_owner_projects = if let Some(family) = &family {
            family
                .projects
                .iter()
                .filter(|project| project.project_kind == "video")
                .cloned()
                .collect()
        } else {
            vec![project.clone()]
        };
        for canvas_project in canvas_owner_projects {
            let mut rows = self
                .canvas
                .video_studio_project_export_rows(canvas_project.id)
                .await?;
            if family.is_some() {
                rows = a4n_db::canvas::VideoStudioProjectExportRows {
                    canvas_graphs: rows.canvas_graphs,
                    canvas_nodes: rows.canvas_nodes,
                    canvas_edges: rows.canvas_edges,
                    canvas_text_assets: rows.canvas_text_assets,
                    ..Default::default()
                };
            }
            let mut canvas_media = video_studio_export_media_files(&rows)?;
            let prefix = format!("canvas/{}/", canvas_project.id);
            for entry in &mut canvas_media {
                entry.package_path = format!("{prefix}{}", entry.package_path);
            }
            let mut canvas_package = assemble_video_studio_project_canvas_package(
                canvas_project,
                rows,
                canvas_media.clone(),
            )?;
            canvas_package.media_files = canvas_media.clone();
            media_files.extend(canvas_media);
            canvas_projects.push(canvas_package);
        }

        let project_ids = family_project_ids
            .into_iter()
            .chain(canvas_projects.iter().map(|package| package.project.id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let history = a4n_db::project::bundle::export_project_bundle_history(
            self.database.connection(),
            &project_ids,
        )
        .await
        .map_err(|error| InternalTransferError::Message(error.to_string()))?;
        let mut package = assemble_project_bundle(
            project.id,
            "project_bundle",
            family,
            canvas_projects,
            history,
            media_files,
        )?;
        self.attach_canvas_provider_slots(&mut package).await?;
        package.validate().map_err(InternalTransferError::Message)?;
        let host_input = project_package_export_input(
            &package,
            &package.media_files,
            PROJECT_BUNDLE_MANIFEST_PATH,
            "统一项目包清单",
            "统一项目导出包",
        )?;
        let package_bytes = host_port
            .render_export_package(host_input)
            .await
            .map_err(InternalTransferError::Message)?;
        Ok(InternalExportedProjectPackage {
            project_title: project.title,
            package_bytes,
        })
    }

    async fn attach_canvas_provider_slots(
        &self,
        package: &mut ProjectBundlePackage,
    ) -> Result<(), InternalTransferError> {
        let (llm, image, video) = tokio::try_join!(
            list_llm_config_records(self.database.connection()),
            list_image_config_records(self.database.connection()),
            list_video_config_records(self.database.connection()),
        )
        .map_err(|error| InternalTransferError::Message(error.to_string()))?;

        let mut descriptors = Vec::new();
        for canvas in &package.canvas_projects {
            for node in &canvas.canvas_nodes {
                let (capability, config_key) = match node.node_kind.as_str() {
                    "LlmTextNode" => ("llm", "llm_config_id"),
                    "GenerateImageNode" => ("image", "image_config_id"),
                    "GenerateVideoNode" => ("video", "video_config_id"),
                    _ => continue,
                };
                let Some(config_id) = node
                    .config_json
                    .get(config_key)
                    .and_then(serde_json::Value::as_i64)
                else {
                    continue;
                };
                match capability {
                    "llm" => {
                        let config = llm
                            .iter()
                            .find(|config| config.id == config_id)
                            .ok_or_else(|| {
                                InternalTransferError::Message(format!(
                                    "画布节点引用的 LLM 配置不存在: {config_id}"
                                ))
                            })?;
                        descriptors.push((
                            config_id,
                            capability,
                            Some(config.provider.clone()),
                            None,
                            Some(config.model.clone()),
                            Some(config.base_url.clone()),
                        ));
                    }
                    "image" => {
                        let config = image
                            .iter()
                            .find(|config| config.id == config_id)
                            .ok_or_else(|| {
                                InternalTransferError::Message(format!(
                                    "画布节点引用的图片配置不存在: {config_id}"
                                ))
                            })?;
                        descriptors.push((
                            config_id,
                            capability,
                            None,
                            Some(config.backend.clone()),
                            Some(config.model.clone()),
                            config.base_url.clone(),
                        ));
                    }
                    "video" => {
                        let config = video
                            .iter()
                            .find(|config| config.id == config_id)
                            .ok_or_else(|| {
                                InternalTransferError::Message(format!(
                                    "画布节点引用的视频配置不存在: {config_id}"
                                ))
                            })?;
                        descriptors.push((
                            config_id,
                            capability,
                            None,
                            Some(config.backend.clone()),
                            Some(config.model.clone()),
                            config.base_url.clone(),
                        ));
                    }
                    _ => unreachable!(),
                }
            }
        }
        for (config_id, capability, provider, backend, model, base_url) in descriptors {
            crate::project::add_canvas_provider_slot(
                package, config_id, capability, provider, backend, model, base_url,
            )
            .map_err(InternalTransferError::Message)?;
        }
        Ok(())
    }

    async fn read_project_bundle<Host>(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<(ProjectBundlePackage, HashMap<String, Vec<u8>>), InternalTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        if input.package_bytes.is_empty() {
            return Err(InternalTransferError::Message(
                "统一项目包不能为空".to_string(),
            ));
        }
        let payload = host_port
            .read_import_package(
                input.package_bytes,
                PROJECT_BUNDLE_MANIFEST_PATH,
                "统一项目包",
                "统一项目包清单",
            )
            .await
            .map_err(InternalTransferError::Message)?;
        let package: ProjectBundlePackage =
            a4n_import_core::parse_json_manifest_value(payload.manifest, "统一项目包清单")
                .map_err(|error| InternalTransferError::Message(error.to_string()))?;
        package.validate().map_err(InternalTransferError::Message)?;
        Ok((package, payload.media_bytes))
    }

    async fn inspect_project_bundle_validated<Host>(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<ProjectBundleImportReport, InternalTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        let (package, media_bytes) = self.read_project_bundle(input, host_port).await?;
        validate_bundle_media(&package, &media_bytes)?;
        Ok(bundle_report(
            history_summary(&package.history),
            self.provider_slot_reports(&package).await?,
        ))
    }

    async fn import_project_bundle_validated<Host>(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<ProjectBundleImportReport, InternalTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        let user_id = input.user_id;
        let mut imported_project_ids = HashMap::new();
        let result = async {
            let (package, media_bytes) = self.read_project_bundle(input, host_port).await?;
            let (family_payload, canvas_payloads) = validate_bundle_media(&package, &media_bytes)?;
            let provider_resolutions = self.provider_slot_resolutions(&package).await?;
            let missing_provider_slots =
                provider_slot_reports_from_resolutions(&package, &provider_resolutions);
            let mut root_project_ids = HashMap::new();
            let mut history_maps = ProjectBundleEntityIdMaps::default();

            if let Some(family) = package.family.clone() {
                let old_projects = family.projects.clone();
                let mut family_rows = family.into_import_package();
                for mut canvas in package
                    .canvas_projects
                    .iter()
                    .filter(|canvas| {
                        old_projects
                            .iter()
                            .any(|project| project.id == canvas.project.id)
                    })
                    .cloned()
                {
                    remap_canvas_provider_slots(
                        &mut canvas,
                        &package.provider_slots,
                        &provider_resolutions,
                    );
                    family_rows.canvas_projects.push(canvas.into_import_rows());
                }
                let mut writer = BundleFamilyMediaWriter {
                    host_port,
                    media_payload: &family_payload,
                };
                let (imported, mappings) = self
                    .project_family
                    .import_package_rows_with_mappings(user_id, &family_rows, &mut writer)
                    .await?;
                if imported.len() != old_projects.len() {
                    return Err(InternalTransferError::Message(
                        "项目族导入返回项目数量不一致".to_string(),
                    ));
                }
                for (old, new) in old_projects.iter().zip(imported) {
                    imported_project_ids.insert(old.id, new.id);
                }
                let old_root = old_projects
                    .iter()
                    .find(|project| project.id == package.root_project_id)
                    .ok_or_else(|| {
                        InternalTransferError::Message("统一包缺少根项目".to_string())
                    })?;
                let new_root = *imported_project_ids
                    .get(&old_root.id)
                    .ok_or_else(|| InternalTransferError::Message("根项目映射缺失".to_string()))?;
                for old in &old_projects {
                    let old_root_id = old.root_project_id.unwrap_or(old.id);
                    root_project_ids.insert(old_root_id, new_root);
                }
                history_maps.chapter_ids = mappings.chapter_ids;
                history_maps.source_snapshot_ids = mappings.source_snapshot_ids;
                history_maps.source_snapshot_chapter_ids = mappings.source_snapshot_chapter_ids;
            }

            for mut canvas in package.canvas_projects.clone() {
                if let Some(new_project_id) = imported_project_ids.get(&canvas.project.id) {
                    root_project_ids.insert(canvas.project.id, *new_project_id);
                    continue;
                }
                remap_canvas_provider_slots(
                    &mut canvas,
                    &package.provider_slots,
                    &provider_resolutions,
                );
                let payload = canvas_payloads.get(&canvas.project.id).ok_or_else(|| {
                    InternalTransferError::Message("画布媒体映射缺失".to_string())
                })?;
                let old_project_id = canvas.project.id;
                let imported = {
                    let mut writer = BundleCanvasMediaWriter {
                        host_port,
                        media_payload: payload,
                    };
                    self.canvas
                        .import_video_studio_package(
                            user_id,
                            canvas,
                            Utc::now().naive_utc(),
                            &mut writer,
                        )
                        .await?
                };
                imported_project_ids.insert(old_project_id, imported.id);
                root_project_ids.insert(old_project_id, imported.id);
            }

            history_maps.project_ids = imported_project_ids.clone();
            history_maps.root_project_ids = root_project_ids;
            let history_summary = a4n_db::project::bundle::import_project_bundle_history(
                self.database.connection(),
                &package.history,
                &history_maps,
                &package.bundle_id,
                user_id,
            )
            .await
            .map_err(|error| InternalTransferError::Message(error.to_string()))?;

            let mut summaries = Vec::new();
            for project_id in imported_project_ids.values().copied() {
                if let Some(summary) = self.project_summary_for_user(user_id, project_id).await? {
                    summaries.push(summary);
                }
            }
            summaries.sort_by_key(|summary| summary.id);
            let mut report = bundle_report(history_summary, missing_provider_slots);
            report.project_summaries = summaries;
            Ok(report)
        }
        .await;

        match result {
            Ok(report) => Ok(report),
            Err(error) => match self
                .rollback_imported_project_ids(
                    user_id,
                    imported_project_ids.values().copied(),
                    host_port,
                )
                .await
            {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(InternalTransferError::Message(format!(
                    "{error}；项目回滚失败：{cleanup_error}"
                ))),
            },
        }
    }

    async fn rollback_imported_project_ids<Host>(
        &self,
        user_id: i64,
        project_ids: impl IntoIterator<Item = i64>,
        host_port: &Host,
    ) -> Result<(), String>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        let mut project_ids = project_ids.into_iter().collect::<Vec<_>>();
        project_ids.sort_unstable_by(|left, right| right.cmp(left));
        project_ids.dedup();
        for project_id in project_ids {
            host_port
                .rollback_imported_project(user_id, project_id)
                .await?;
        }
        Ok(())
    }

    async fn provider_slot_resolutions(
        &self,
        package: &ProjectBundlePackage,
    ) -> Result<HashMap<String, Option<i64>>, InternalTransferError> {
        let (llm, image, video) = tokio::try_join!(
            list_llm_config_records(self.database.connection()),
            list_image_config_records(self.database.connection()),
            list_video_config_records(self.database.connection()),
        )
        .map_err(|error| InternalTransferError::Message(error.to_string()))?;
        Ok(package
            .provider_slots
            .iter()
            .map(|slot| {
                let model_matches = |model: &str| slot.model.as_deref() == Some(model);
                let matched = match slot.capability.as_str() {
                    "llm" => llm
                        .iter()
                        .find(|config| {
                            model_matches(&config.model)
                                && slot.provider.as_deref() == Some(config.provider.as_str())
                                && slot
                                    .base_url
                                    .as_deref()
                                    .is_none_or(|url| url == config.base_url)
                        })
                        .map(|config| config.id),
                    "image" => image
                        .iter()
                        .find(|config| {
                            model_matches(&config.model)
                                && slot
                                    .backend
                                    .as_deref()
                                    .is_none_or(|backend| backend == config.backend)
                                && slot
                                    .base_url
                                    .as_deref()
                                    .is_none_or(|url| config.base_url.as_deref() == Some(url))
                        })
                        .map(|config| config.id),
                    "video" => video
                        .iter()
                        .find(|config| {
                            model_matches(&config.model)
                                && slot
                                    .backend
                                    .as_deref()
                                    .is_none_or(|backend| backend == config.backend)
                                && slot
                                    .base_url
                                    .as_deref()
                                    .is_none_or(|url| config.base_url.as_deref() == Some(url))
                        })
                        .map(|config| config.id),
                    _ => None,
                };
                (slot.slot_key.clone(), matched)
            })
            .collect())
    }

    async fn provider_slot_reports(
        &self,
        package: &ProjectBundlePackage,
    ) -> Result<
        Vec<a4n_api_contract::project_transfer::ProjectBundleProviderSlotReport>,
        InternalTransferError,
    > {
        let resolutions = self.provider_slot_resolutions(package).await?;
        Ok(provider_slot_reports_from_resolutions(
            package,
            &resolutions,
        ))
    }
}

#[async_trait::async_trait]
impl ProjectBundleTransferApplicationPort for OrchestratorContext {
    async fn export_bundle(
        &self,
        input: ExportProjectBundleApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError> {
        self.export_project_bundle_validated(input, host_port).await
    }

    async fn inspect_bundle(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<ProjectBundleImportReport, ProjectPackageTransferError> {
        self.inspect_project_bundle_validated(input, host_port)
            .await
    }

    async fn import_bundle(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<ProjectBundleImportReport, ProjectPackageTransferError> {
        match self.import_project_bundle_validated(input, host_port).await {
            Ok(report) => Ok(report),
            Err(error) => match host_port.rollback_imported_media_files().await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(ProjectPackageTransferError::Message(format!(
                    "{error}；{cleanup_error}"
                ))),
            },
        }
    }
}

fn package_project_ids(package: &ProjectFamilyPackage) -> Vec<i64> {
    package.projects.iter().map(|project| project.id).collect()
}

fn validate_bundle_media(
    package: &ProjectBundlePackage,
    media_bytes: &HashMap<String, Vec<u8>>,
) -> Result<(ImportMediaPayload, HashMap<i64, ImportMediaPayload>), InternalTransferError> {
    let root_paths = package
        .media_files
        .iter()
        .map(|entry| entry.package_path.as_str())
        .collect::<HashSet<_>>();
    let byte_paths = media_bytes
        .keys()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if root_paths != byte_paths {
        return Err(InternalTransferError::Message(
            "统一项目包媒体字节与清单不一致".to_string(),
        ));
    }
    if root_paths.is_empty() != package.media_sha256.is_empty()
        || package
            .media_sha256
            .keys()
            .map(String::as_str)
            .collect::<HashSet<_>>()
            != root_paths
    {
        return Err(InternalTransferError::Message(
            "统一项目包缺少完整媒体 SHA-256 清单".to_string(),
        ));
    }
    for path in &root_paths {
        let expected = package
            .media_sha256
            .get(*path)
            .ok_or_else(|| InternalTransferError::Message(format!("媒体缺少 SHA-256: {path}")))?;
        let bytes = media_bytes
            .get(*path)
            .ok_or_else(|| InternalTransferError::Message(format!("媒体字节缺失: {path}")))?;
        let actual = Sha256::digest(bytes);
        let actual = actual
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if expected != &actual {
            return Err(InternalTransferError::Message(format!(
                "媒体 SHA-256 不匹配: {path}"
            )));
        }
    }
    let family_payload = if let Some(family) = &package.family {
        let payload = scoped_import_media_payload(&family.media_files, media_bytes)?;
        validate_family_package_manifest_with_canvas(family, &package.canvas_projects)
            .map_err(InternalTransferError::Message)?;
        validate_family_media_manifest(family, &payload).map_err(InternalTransferError::Message)?;
        payload
    } else {
        ImportMediaPayload {
            entries: HashMap::new(),
            bytes: HashMap::new(),
        }
    };
    let mut canvas_payloads = HashMap::new();
    let mut nested_paths = HashSet::new();
    if let Some(family) = &package.family {
        nested_paths.extend(
            family
                .media_files
                .iter()
                .map(|entry| entry.package_path.as_str()),
        );
    }
    for canvas in &package.canvas_projects {
        nested_paths.extend(
            canvas
                .media_files
                .iter()
                .map(|entry| entry.package_path.as_str()),
        );
        let payload = scoped_import_media_payload(&canvas.media_files, media_bytes)?;
        let family_owned = package.family.as_ref().is_some_and(|family| {
            family
                .projects
                .iter()
                .any(|project| project.id == canvas.project.id)
        });
        if !family_owned {
            validate_video_studio_package_manifest(canvas, &payload)
                .map_err(InternalTransferError::Message)?;
        }
        canvas_payloads.insert(canvas.project.id, payload);
    }
    if nested_paths != root_paths {
        return Err(InternalTransferError::Message(
            "统一项目包嵌套媒体清单与根清单不一致".to_string(),
        ));
    }
    Ok((family_payload, canvas_payloads))
}

fn scoped_import_media_payload(
    entries: &[crate::project::MediaFileEntry],
    media_bytes: &HashMap<String, Vec<u8>>,
) -> Result<ImportMediaPayload, InternalTransferError> {
    let mut scoped = HashMap::with_capacity(entries.len());
    for entry in entries {
        let bytes = media_bytes.get(&entry.package_path).ok_or_else(|| {
            InternalTransferError::Message(format!("项目包缺少媒体文件 {}", entry.package_path))
        })?;
        scoped.insert(entry.package_path.clone(), bytes.clone());
    }
    index_import_media_payload(entries, scoped).map_err(InternalTransferError::Message)
}

fn bundle_report(
    history: ProjectBundleHistoryImportSummary,
    provider_slots: Vec<a4n_api_contract::project_transfer::ProjectBundleProviderSlotReport>,
) -> ProjectBundleImportReport {
    let has_missing_provider_slots = !provider_slots.is_empty();
    ProjectBundleImportReport {
        project_summaries: Vec::new(),
        conversations_imported: history.conversations_imported,
        messages_imported: history.messages_imported,
        workflow_runs_imported: history.workflow_runs_imported,
        workflow_runs_marked_interrupted: history.workflow_runs_marked_interrupted,
        generation_runs_imported: history.generation_runs_imported,
        missing_provider_slots: provider_slots,
        warnings: if has_missing_provider_slots {
            vec!["项目包不包含可用的本机 Provider 凭证；恢复生成前必须显式绑定配置".to_string()]
        } else {
            Vec::new()
        },
    }
}

fn history_summary(rows: &ProjectBundleHistoryRows) -> ProjectBundleHistoryImportSummary {
    ProjectBundleHistoryImportSummary {
        conversations_imported: rows.conversations.len(),
        messages_imported: rows.messages.len(),
        workflow_runs_imported: rows.workflow_runs.len(),
        workflow_runs_marked_interrupted: rows
            .workflow_runs
            .iter()
            .filter(|run| matches!(run.status.as_str(), "running" | "interrupted"))
            .count(),
        generation_runs_imported: rows.generation_runs.len(),
    }
}

fn provider_slot_reports_from_resolutions(
    package: &ProjectBundlePackage,
    resolutions: &HashMap<String, Option<i64>>,
) -> Vec<a4n_api_contract::project_transfer::ProjectBundleProviderSlotReport> {
    package
        .provider_slots
        .iter()
        .filter(|slot| {
            resolutions
                .get(&slot.slot_key)
                .is_none_or(|resolution| resolution.is_none())
        })
        .map(
            |slot| a4n_api_contract::project_transfer::ProjectBundleProviderSlotReport {
                slot_key: slot.slot_key.clone(),
                provider: slot.provider.clone().or_else(|| slot.backend.clone()),
                model: slot.model.clone(),
                configuration_fingerprint_sha256: slot.configuration_fingerprint_sha256.clone(),
                status: "missing_configuration_or_credentials".to_string(),
            },
        )
        .collect()
}

fn remap_canvas_provider_slots(
    package: &mut crate::project::VideoStudioProjectPackage,
    provider_slots: &[crate::project::ProjectBundleProviderSlot],
    resolutions: &HashMap<String, Option<i64>>,
) {
    for node in &mut package.canvas_nodes {
        let (capability, config_key) = match node.node_kind.as_str() {
            "LlmTextNode" => ("llm", "llm_config_id"),
            "GenerateImageNode" => ("image", "image_config_id"),
            "GenerateVideoNode" => ("video", "video_config_id"),
            _ => continue,
        };
        let Some(config) = node.config_json.as_object_mut() else {
            continue;
        };
        let source_config_id = config.get(config_key).and_then(serde_json::Value::as_i64);
        let configured_model = config.get("model").and_then(serde_json::Value::as_str);
        if source_config_id.is_none() && configured_model.is_none() {
            continue;
        }
        let slot = source_config_id
            .and_then(|source_config_id| {
                provider_slots.iter().find(|slot| {
                    slot.capability == capability && slot.source_config_id == Some(source_config_id)
                })
            })
            .or_else(|| {
                provider_slots
                    .iter()
                    .filter(|slot| slot.capability == capability)
                    .find(|slot| slot.model.as_deref() == configured_model)
            });
        let resolved_id = slot.and_then(|slot| resolutions.get(&slot.slot_key).copied().flatten());
        if let Some(config_id) = resolved_id {
            config.insert(config_key.to_string(), serde_json::Value::from(config_id));
            config.remove("a4n_provider_slot_unresolved");
        } else {
            config.remove(config_key);
            config.insert(
                "a4n_provider_slot_unresolved".to_string(),
                serde_json::Value::Bool(true),
            );
        }
    }
}

struct BundleFamilyMediaWriter<'a, Host: ?Sized> {
    host_port: &'a mut Host,
    media_payload: &'a ImportMediaPayload,
}

impl<Host> ProjectFamilyMediaFileImporter for BundleFamilyMediaWriter<'_, Host>
where
    Host: ProjectPackageTransferHostPort + ?Sized,
{
    fn write_imported_media_field(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        old_owner_id: i64,
        new_owner_id: i64,
        field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        let Some(entry) =
            self.media_payload
                .entries
                .get(&media_file_key(owner_kind, old_owner_id, field))
        else {
            return Ok(None);
        };
        let bytes = self
            .media_payload
            .bytes
            .get(&entry.package_path)
            .ok_or_else(|| {
                ProjectFamilyMediaImportError::Write(format!(
                    "媒体文件字节缺失: {}",
                    entry.package_path
                ))
            })?;
        let relative_path = imported_project_family_media_path(
            project_id,
            owner_kind,
            new_owner_id,
            field,
            &entry.package_path,
        )
        .map_err(ProjectFamilyMediaImportError::Write)?;
        self.host_port
            .write_imported_media_file(&relative_path, bytes, "统一项目包媒体文件")
            .map_err(ProjectFamilyMediaImportError::Write)?;
        Ok(Some(relative_path))
    }
}

struct BundleCanvasMediaWriter<'a, Host: ?Sized> {
    host_port: &'a mut Host,
    media_payload: &'a ImportMediaPayload,
}

impl<Host> VideoStudioProjectImportMediaFileWriter for BundleCanvasMediaWriter<'_, Host>
where
    Host: ProjectPackageTransferHostPort + ?Sized,
{
    fn write_imported_media_file(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        owner_id: i64,
        field: &str,
        folder: &str,
        stem: &str,
    ) -> Result<Option<String>, VideoStudioProjectImportMediaError> {
        let Some(entry) = self
            .media_payload
            .entries
            .get(&media_file_key(owner_kind, owner_id, field))
        else {
            return Ok(None);
        };
        let bytes = self
            .media_payload
            .bytes
            .get(&entry.package_path)
            .ok_or_else(|| {
                VideoStudioProjectImportMediaError::Write(format!(
                    "媒体文件字节缺失: {}",
                    entry.package_path
                ))
            })?;
        let extension = required_package_extension(&entry.package_path)
            .map_err(VideoStudioProjectImportMediaError::Write)?;
        let relative_path =
            format!("projects/{project_id}/{folder}/{stem}_{owner_id}_{field}.{extension}");
        self.host_port
            .write_imported_media_file(&relative_path, bytes, "统一项目包媒体文件")
            .map_err(VideoStudioProjectImportMediaError::Write)?;
        Ok(Some(relative_path))
    }
}
