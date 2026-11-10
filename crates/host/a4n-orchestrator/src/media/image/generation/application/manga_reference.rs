use a4n_api_contract::image_generation::MangaReferenceImageOptions;
use a4n_api_contract::manga::SaveMangaPipelineArtifactInput;
use a4n_domain::manga::{
    MangaBoundReferenceAsset, MangaPageReferenceDiscoveryV1, MangaPageReferenceImageV1,
    MangaPageReferenceNeedV1, MangaPipelineArtifactStage, MangaReferenceAssetKind,
};

pub(in crate::media::image::generation) struct MangaReferenceImageMaterial {
    page: MangaPagePrompt,
    discovery: MangaPageReferenceDiscoveryV1,
    requirement: MangaPageReferenceNeedV1,
    character_references: Vec<MangaReferenceBindingSummary>,
}

impl OrchestratorContext {
    pub(in crate::media::image::generation) async fn manga_reference_image_material(
        &self,
        user_id: i64,
        project_id: i64,
        options: &MangaReferenceImageOptions,
    ) -> Result<MangaReferenceImageMaterial, AppError> {
        self.require_image_project_editor(user_id, project_id)
            .await?;
        require_positive(options.prompt_id, "prompt_id")?;
        let page = self
            .manga_page_prompt_by_id(options.prompt_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("漫画页面 #{} 不存在", options.prompt_id)))?;
        if page.project_id != project_id {
            return Err(AppError::InvalidArgument("漫画页面不属于当前项目".into()));
        }
        if page.updated_at.to_string() != options.expected_updated_at {
            return Err(AppError::InvalidArgument(
                "漫画页面已被其他操作修改，请刷新后重试".into(),
            ));
        }
        self.ensure_manga_pipeline_not_running(project_id, page.chapter_id, "生成漫画参考图")
            .await?;
        let source = self
            .chapter_manga_source_by_id(page.chapter_id, project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "生成漫画参考图")?;
        let layout = a4n_manga_core::manga_page_pipeline_layout(
            page.id,
            page.page_number,
            page.layout.as_ref(),
        )
        .map_err(AppError::InvalidArgument)?;
        if layout.source.content_sha256 != source_content_sha256(&source.content) {
            return Err(AppError::InvalidArgument(
                "漫画页面原文已变化，请先更新上游产物".into(),
            ));
        }
        self.validate_page_artifact_chain_to_storyboard(
            project_id,
            page.chapter_id,
            page.page_number as u32,
        )
        .await?;
        let artifact = self
            .manga_pipeline_artifact(
                project_id,
                page.chapter_id,
                MangaPipelineArtifactStage::ReferenceDiscovery,
                Some(page.page_number as u32),
            )
            .await?
            .ok_or_else(|| AppError::InvalidArgument("请先完成参考图发现".into()))?;
        if artifact.stale {
            return Err(AppError::InvalidArgument(
                "参考图发现已过期，请先更新上游产物".into(),
            ));
        }
        let discovery = layout
            .pipeline
            .reference_discovery
            .ok_or_else(|| AppError::InvalidArgument("请先完成参考图发现".into()))?;
        if artifact.payload != serde_json::to_value(&discovery)? {
            return Err(AppError::InvalidArgument(
                "参考图发现与页面版本不一致，请刷新后重试".into(),
            ));
        }
        let requirement = reference_requirement(&discovery, options)?.clone();
        let character_references = layout
            .reference_bindings
            .into_iter()
            .filter(|binding| {
                options.target_kind == a4n_domain::manga::MangaPageReferenceTargetKind::Character
                    && binding.character_name == options.target_name
            })
            .collect();
        Ok(MangaReferenceImageMaterial {
            page,
            discovery,
            requirement,
            character_references,
        })
    }

    pub(in crate::media::image::generation) async fn generate_manga_reference_image_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        project_id: i64,
        options: MangaReferenceImageOptions,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<GeneratedImageSummary, AppError> {
        let material = self
            .manga_reference_image_material(user_id, project_id, &options)
            .await?;
        let requirement = material.requirement;
        let prompt = format!(
            "漫画视觉参考图。只生成一张用于后续漫画绘制的参考图片，不绘制分格、对白气泡、文字或水印。\n目标：{:?} · {}\n视觉要求：{}\n连续性用途：{}\n严格保留要求中的人物身份、道具外观、空间关系与状态，不新增剧情。",
            requirement.target_kind,
            requirement.target_name,
            requirement.description,
            requirement.usage,
        );
        progress.progress(0, 3, "正在准备参考图生成请求");
        check_progress_cancelled(progress)?;
        let mut prepared = self.prepare_direct_image(host, DirectImagePlan {
            project_id, prompt, image_type: "chapter_reference".into(), config_id: None,
            width: options.width, height: options.height, chapter_id: Some(material.page.chapter_id),
            source_snapshot_id: None, panel_id: None, workflow_run_id: None,
            metadata_json: Some(json!({"source":"manga_reference_generation", "prompt_id":options.prompt_id,
                "page_number":material.page.page_number, "expected_page_updated_at":options.expected_updated_at,
                "requirement":requirement})),
            command: "image_manga_reference".into(), file_extra: None, file_context: "漫画参考图文件".into(),
        }, ImageChapterBindingAccess::Writable).await?;
        prepared.preview.request.reference_images =
            collect_bound_reference_images(host, &[], &material.character_references)?;
        if !prepared.preview.request.reference_images.is_empty() {
            ensure_batch_reference_image_backend(&prepared.preview.config.backend)
                .map_err(AppError::InvalidArgument)?;
        }
        progress.progress(1, 3, "正在生成参考图");
        let image = self
            .execute_prepared_direct_image(host, user_id, prepared, progress)
            .await?
            .summary;
        let binding = async {
            check_progress_cancelled(progress)?;
            let mut discovery = material.discovery;
            discovery.references.push(MangaPageReferenceImageV1 {
                target_kind: requirement.target_kind,
                target_name: requirement.target_name,
                description: requirement.description,
                usage: requirement.usage,
                asset: MangaBoundReferenceAsset {
                    asset_kind: MangaReferenceAssetKind::GeneratedImage,
                    asset_id: image.id,
                    image_path: image.file_path.clone(),
                },
            });
            discovery
                .validate_for_write()
                .map_err(AppError::InvalidArgument)?;
            self.save_manga_pipeline_artifact_validated(
                user_id,
                SaveMangaPipelineArtifactInput {
                    project_id,
                    chapter_id: material.page.chapter_id,
                    stage: MangaPipelineArtifactStage::ReferenceDiscovery,
                    page_number: Some(material.page.page_number as u32),
                    payload: serde_json::to_value(discovery)?,
                    expected_prompt_updated_at: Some(options.expected_updated_at),
                },
            )
            .await?;
            Ok::<(), AppError>(())
        }
        .await;
        if let Err(error) = binding {
            if matches!(error, AppError::Cancelled) {
                return Err(error);
            }
            return Err(AppError::InvalidState {
                current: format!(
                    "参考图 #{} 已保存至素材库，但自动绑定失败：{error}",
                    image.id
                ),
                operation: "请刷新页面后从素材库选择该图片".into(),
            });
        }
        progress.progress(3, 3, "参考图已生成并绑定到发现需求");
        Ok(image)
    }
}

fn reference_requirement<'a>(
    discovery: &'a MangaPageReferenceDiscoveryV1,
    options: &MangaReferenceImageOptions,
) -> Result<&'a MangaPageReferenceNeedV1, AppError> {
    discovery
        .validate_for_write()
        .map_err(AppError::InvalidArgument)?;
    let requirement = discovery
        .requirements
        .iter()
        .find(|need| {
            need.target_kind == options.target_kind && need.target_name == options.target_name
        })
        .ok_or_else(|| AppError::InvalidArgument("未找到该参考图需求，请刷新后重试".into()))?;
    discovery
        .ensure_reference_capacity(options.target_kind, &options.target_name)
        .map_err(AppError::InvalidArgument)?;
    Ok(requirement)
}

#[cfg(test)]
mod manga_reference_tests {
    use super::*;
    use a4n_domain::manga::{MANGA_PAGE_REFERENCE_DISCOVERY_V1, MangaPageReferenceTargetKind};

    #[test]
    fn discovered_generation_requires_a_real_requirement_and_available_capacity() {
        let options = MangaReferenceImageOptions {
            prompt_id: 1,
            target_kind: MangaPageReferenceTargetKind::Object,
            target_name: "断裂树根".into(),
            expected_updated_at: "2026-11-10 00:00:00".into(),
            width: 1024,
            height: 1024,
        };
        let need = MangaPageReferenceNeedV1 {
            target_kind: options.target_kind,
            target_name: options.target_name.clone(),
            description: "近景树根遮挡身体".into(),
            usage: "保持人物藏在树根后的空间关系".into(),
        };
        let mut discovery = MangaPageReferenceDiscoveryV1 {
            schema_version: MANGA_PAGE_REFERENCE_DISCOVERY_V1.into(),
            requirements: vec![need.clone()],
            references: vec![],
        };
        assert_eq!(reference_requirement(&discovery, &options).unwrap(), &need);
        discovery.requirements.push(need.clone());
        assert!(
            reference_requirement(&discovery, &options)
                .unwrap_err()
                .to_string()
                .contains("不唯一")
        );
        discovery.requirements.pop();
        let mut wrong = options.clone();
        wrong.target_kind = MangaPageReferenceTargetKind::Scene;
        assert!(reference_requirement(&discovery, &wrong).is_err());
        wrong = options.clone();
        wrong.target_name = "其他树根".into();
        assert!(reference_requirement(&discovery, &wrong).is_err());
        for id in 1..=2 {
            discovery.references.push(MangaPageReferenceImageV1 {
                asset: MangaBoundReferenceAsset {
                    asset_kind: MangaReferenceAssetKind::GeneratedImage,
                    asset_id: id,
                    image_path: format!("reference-{id}.png"),
                },
                target_kind: need.target_kind,
                target_name: need.target_name.clone(),
                description: need.description.clone(),
                usage: need.usage.clone(),
            });
            assert_eq!(reference_requirement(&discovery, &options).is_ok(), id < 2);
        }
        discovery.requirements.clear();
        assert!(
            reference_requirement(&discovery, &options)
                .unwrap_err()
                .to_string()
                .contains("未找到")
        );
        discovery.requirements.push(need);
        for id in 3..=12 {
            let mut reference = discovery.references[0].clone();
            reference.target_name = format!("道具{id}");
            reference.asset.asset_id = id;
            discovery.references.push(reference);
        }
        let mut other_need = discovery.requirements[0].clone();
        other_need.target_name = "新道具".into();
        discovery.requirements.push(other_need);
        let mut other = options;
        other.target_name = "新道具".into();
        assert!(
            reference_requirement(&discovery, &other)
                .unwrap_err()
                .to_string()
                .contains("12")
        );
    }
}
