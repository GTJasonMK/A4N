use super::*;

impl VideoService {
    pub async fn preview_chapter_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::boundary_frames(),
        )
        .await
    }

    pub async fn preview_chapter_storyboard_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::storyboard(),
        )
        .await
    }

    async fn preview_local_video_prompt_requests(
        &self,
        request: LocalPromptPreviewInput<'_, '_>,
        spec: LocalVideoPromptPreviewSpec,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        let LocalPromptPreviewInput {
            project_id,
            chapter_id,
            source,
            prompt_engine,
            variables,
            options,
        } = request;
        let input = self
            .prepare_video_prompt_preview_inputs(
                project_id, chapter_id, &source, variables, options, false,
            )
            .await?;

        let mut steps = Vec::new();
        let segment_out = self
            .reusable_prompt_output(ReusablePromptOutputQuery {
                project_id,
                chapter_id,
                prompt_type: spec.segment_prompt_type,
                source_metadata: &input.source_metadata,
                style: &input.style,
                duration_secs: input.duration_secs,
                aspect_ratio: &input.aspect_ratio,
                reference_ids: &input.reference_ids,
                prompt_output_language: input.prompt_output_language,
            })
            .await?;
        let (segment_json, segment_plan, segment_is_placeholder) =
            if let Some(segment_out) = segment_out {
                steps.push(reused_step(
                    "segment_plan",
                    spec.segment_title,
                    "chapter_video_segment_plan",
                    spec.segment_reused_note,
                ));
                let (segment_json, segment_plan) =
                    parse_segment_plan_for_preview(&segment_out, &input)?;
                (segment_json, segment_plan, false)
            } else {
                let project_context = self
                    .chapter_video_project_context(project_id, source.analysis)
                    .await?;
                self.push_segment_plan_preview_steps(
                    &mut steps,
                    SegmentPlanPreviewStepsInput {
                        prompt_engine,
                        project_id,
                        chapter_id,
                        source: &source,
                        input: &input,
                        project_context: project_context.as_ref(),
                        current_workflow_mode: spec.workflow_mode(),
                        segment_title: spec.segment_title,
                    },
                )
                .await?;
                (placeholder_segment_plan(), placeholder_segments(), true)
            };
        let allowed_reference_ids =
            allowed_reference_ids_for_preview(&input.available_assets, &input.reference_image_ids)?;

        let (frame_json, frame_is_placeholder) = if segment_is_placeholder {
            let frame_user = match spec.mode {
                LocalVideoPromptPreviewMode::BoundaryFrames
                | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                    let next = Self::placeholder_boundary_frame_prompt_preview_state(
                        &input,
                        &segment_plan,
                    )?;
                    build_boundary_frame_prompt_preview_user(
                        prompt_engine,
                        project_id,
                        &source,
                        &input,
                        &segment_json,
                        &next.current_frame,
                        &next.available_assets,
                    )?
                }
                LocalVideoPromptPreviewMode::FirstFrameChain => {
                    build_first_frame_prompt_preview_user(
                        prompt_engine,
                        project_id,
                        &source,
                        &input,
                        &segment_json,
                    )?
                }
                LocalVideoPromptPreviewMode::Storyboard => {
                    let next =
                        Self::placeholder_storyboard_prompt_preview_state(&input, &segment_plan)?;
                    build_storyboard_prompt_preview_user(
                        prompt_engine,
                        project_id,
                        &source,
                        &input,
                        &segment_json,
                        &next.current_frame,
                        &next.available_assets,
                    )?
                }
            };
            let frame_request =
                build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                    prompt_engine,
                    project_id,
                    source: &source,
                    input: &input,
                    prompt_name: spec.frame_prompt_name,
                    user: frame_user,
                    segment_json: &segment_json,
                    asset_context: &input.available_assets,
                    segment_placeholder: segment_is_placeholder,
                })?;
            steps.push(placeholder_step_from_request(
                spec.frame_key,
                spec.frame_title,
                spec.frame_prompt_name,
                frame_request,
                "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 会替换为上一步片段规划模型输出。",
            )?);
            (
                match spec.mode {
                    LocalVideoPromptPreviewMode::BoundaryFrames
                    | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                        placeholder_boundary_frame_context_for_video_prompt(&segment_plan)
                    }
                    LocalVideoPromptPreviewMode::FirstFrameChain => {
                        placeholder_first_frame_prompts(&segment_plan)
                    }
                    LocalVideoPromptPreviewMode::Storyboard => {
                        placeholder_storyboard_prompts(&segment_plan)?
                    }
                },
                true,
            )
        } else {
            match spec.mode {
                LocalVideoPromptPreviewMode::BoundaryFrames
                | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                    match self
                        .boundary_frame_prompt_preview_state(BoundaryFramePromptPreviewStateInput {
                            project_id,
                            chapter_id,
                            input: &input,
                            segment_plan: &segment_plan,
                            allowed_reference_ids: &allowed_reference_ids,
                            workflow_mode: spec.prompt_workflow_mode(),
                            frame_prompt_type: spec.frame_prompt_type,
                            frame_source_prompt_type: spec.frame_source_prompt_type,
                        })
                        .await?
                    {
                        BoundaryFramePromptPreviewState::Complete(frame_prompts) => {
                            steps.push(reused_step(
                                spec.frame_key,
                                spec.frame_title,
                                spec.frame_prompt_name,
                                &format!(
                                    "已存在完整{}，视频提示词请求会直接使用该前置结果",
                                    spec.frame_title
                                ),
                            ));
                            (json!({ "frames": frame_prompts }), false)
                        }
                        BoundaryFramePromptPreviewState::Next(next) => {
                            let frame_user = build_boundary_frame_prompt_preview_user(
                                prompt_engine,
                                project_id,
                                &source,
                                &input,
                                &segment_json,
                                &next.current_frame,
                                &next.available_assets,
                            )?;
                            let frame_request = build_frame_prompt_preview_request(
                                FramePromptPreviewRequestInput {
                                    prompt_engine,
                                    project_id,
                                    source: &source,
                                    input: &input,
                                    prompt_name: spec.frame_prompt_name,
                                    user: frame_user,
                                    segment_json: &segment_json,
                                    asset_context: &next.available_assets,
                                    segment_placeholder: false,
                                },
                            )?;
                            steps.push(step_from_request(
                                spec.frame_key,
                                spec.frame_title,
                                spec.frame_prompt_name,
                                frame_request,
                            )?);
                            (
                                placeholder_boundary_frame_context_for_video_prompt(&segment_plan),
                                true,
                            )
                        }
                    }
                }
                LocalVideoPromptPreviewMode::FirstFrameChain => {
                    if let Some(frame_prompts) = self
                        .reusable_first_frame_prompts_for_preview(
                            project_id,
                            chapter_id,
                            &input,
                            &segment_plan,
                            &allowed_reference_ids,
                            true,
                        )
                        .await?
                    {
                        steps.push(reused_step(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            &format!(
                                "已存在完整{}，视频提示词请求会直接使用该前置结果",
                                spec.frame_title
                            ),
                        ));
                        (json!({ "frames": frame_prompts }), false)
                    } else {
                        let frame_user = build_first_frame_prompt_preview_user(
                            prompt_engine,
                            project_id,
                            &source,
                            &input,
                            &segment_json,
                        )?;
                        let frame_request =
                            build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                                prompt_engine,
                                project_id,
                                source: &source,
                                input: &input,
                                prompt_name: spec.frame_prompt_name,
                                user: frame_user,
                                segment_json: &segment_json,
                                asset_context: &input.available_assets,
                                segment_placeholder: false,
                            })?;
                        steps.push(step_from_request(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            frame_request,
                        )?);
                        (placeholder_first_frame_prompts(&segment_plan), true)
                    }
                }
                LocalVideoPromptPreviewMode::Storyboard => {
                    match self
                        .storyboard_prompt_preview_state(
                            project_id,
                            chapter_id,
                            &input,
                            &segment_plan,
                            &allowed_reference_ids,
                        )
                        .await?
                    {
                        StoryboardPromptPreviewState::Complete(frame_prompts) => {
                            steps.push(reused_step(
                                spec.frame_key,
                                spec.frame_title,
                                spec.frame_prompt_name,
                                &format!(
                                    "已存在完整{}，视频提示词请求会直接使用该前置结果",
                                    spec.frame_title
                                ),
                            ));
                            (json!({ "frames": frame_prompts }), false)
                        }
                        StoryboardPromptPreviewState::Next(next) => {
                            let frame_user = build_storyboard_prompt_preview_user(
                                prompt_engine,
                                project_id,
                                &source,
                                &input,
                                &segment_json,
                                &next.current_frame,
                                &next.available_assets,
                            )?;
                            let frame_request = build_frame_prompt_preview_request(
                                FramePromptPreviewRequestInput {
                                    prompt_engine,
                                    project_id,
                                    source: &source,
                                    input: &input,
                                    prompt_name: spec.frame_prompt_name,
                                    user: frame_user,
                                    segment_json: &segment_json,
                                    asset_context: &next.available_assets,
                                    segment_placeholder: false,
                                },
                            )?;
                            steps.push(step_from_request(
                                spec.frame_key,
                                spec.frame_title,
                                spec.frame_prompt_name,
                                frame_request,
                            )?);
                            (placeholder_storyboard_prompts(&segment_plan)?, true)
                        }
                    }
                }
            }
        };
        let frame_prompts_for_reference = frame_prompts_from_preview_context(&frame_json)?;
        let frame_by_number = frame_prompts_for_reference
            .iter()
            .map(|frame| (frame.frame_number, frame.clone()))
            .collect::<HashMap<_, _>>();
        let video_segment_plan = video_prompt_segments_with_reference_ids(
            spec.prompt_workflow_mode(),
            &segment_plan,
            &frame_by_number,
            &HashMap::new(),
            &input.reference_image_ids,
        );
        let video_segment_json = json!({ "segments": &video_segment_plan });
        let matching_video_prompts = if segment_is_placeholder {
            Vec::new()
        } else {
            self.matching_prompt_rows_for_workflow(
                PromptWorkflowRowsContext {
                    project_id,
                    chapter_id,
                    workflow_mode: spec.workflow_mode(),
                    prompt_type: "video_prompt",
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    duration_secs: input.duration_secs,
                    aspect_ratio: &input.aspect_ratio,
                    reference_ids: &input.reference_ids,
                    generation_asset_fingerprint: Some(&input.generation_asset_fingerprint),
                    source_prompt_type: Some(spec.segment_prompt_type),
                },
                input.prompt_output_language,
            )
            .await?
        };
        let reusable_video_prompts = if input.regenerate_existing {
            Default::default()
        } else {
            match spec.mode {
                LocalVideoPromptPreviewMode::BoundaryFrames => prompt_values_by_shot(
                    reusable_boundary_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
                LocalVideoPromptPreviewMode::FirstFrameChain => prompt_values_by_shot(
                    reusable_first_frame_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
                LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => prompt_values_by_shot(
                    reusable_chat_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
                LocalVideoPromptPreviewMode::Storyboard => prompt_values_by_shot(
                    reusable_boundary_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
            }
        };
        let (current_segment, all_segments_ready, previous_shot) = video_prompt_target(
            &video_segment_plan,
            &reusable_video_prompts,
            input.regenerate_existing,
            segment_is_placeholder,
            spec.reuse_sequence,
        );
        let Some(current_segment) = current_segment else {
            steps.push(reused_step(
                "video_prompt",
                spec.video_title,
                spec.video_prompt_name,
                &format!("片段规划为空，本次不会调用{}生成模型", spec.video_title),
            ));
            return Ok(preview_video_prompt_from_steps(steps));
        };
        if all_segments_ready {
            steps.push(reused_step(
                "video_prompt",
                spec.video_title,
                spec.video_prompt_name,
                &format!("所有片段均已有{}，本次生成不会调用模型", spec.video_title),
            ));
            return Ok(preview_video_prompt_from_steps(steps));
        }
        let current_segment_json = json!(current_segment);
        let previous_shot_json = previous_shot.cloned();
        let final_prompt_context = FinalVideoPromptContext {
            prompt_engine,
            project_id,
            chapter_summary: source.summary,
            chapter_analysis: source.analysis,
            segment_plan: &video_segment_json,
            frame_prompts: &frame_json,
            current_segment: &current_segment_json,
            previous_completed_shot: previous_shot_json.as_ref(),
            available_assets: &input.available_assets,
            style: &input.style,
            prompt_output_language: input.prompt_output_language,
            duration_secs: input.duration_secs,
            aspect_ratio: &input.aspect_ratio,
            reference_ids: &input.reference_ids,
        };
        let user = match spec.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => {
                build_boundary_video_prompt_user(final_prompt_context)
            }
            LocalVideoPromptPreviewMode::FirstFrameChain => {
                build_first_frame_video_prompt_user(final_prompt_context)
            }
            LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                build_chat_video_prompt_user(final_prompt_context)
            }
            LocalVideoPromptPreviewMode::Storyboard => {
                build_storyboard_video_prompt_user(final_prompt_context)
            }
        }?;
        let request = build_video_prompt_preview_request(VideoPromptPreviewRequestInput {
            prompt_engine,
            project_id,
            source: &source,
            input: &input,
            prompt_name: spec.video_prompt_name,
            frame_prompt_name: spec.frame_prompt_name,
            user,
        })?;
        push_video_prompt_request_step(
            &mut steps,
            spec.video_title,
            spec.video_prompt_name,
            request,
            (segment_is_placeholder || frame_is_placeholder).then_some(spec.video_placeholder_note),
            None,
        )?;
        let draft = placeholder_video_prompt_draft(spec.video_prompt_name);
        let review_user = match spec.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => {
                build_boundary_video_prompt_review_user(final_prompt_context, &draft)
            }
            LocalVideoPromptPreviewMode::FirstFrameChain => {
                build_first_frame_video_prompt_review_user(final_prompt_context, &draft)
            }
            LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                build_chat_video_prompt_review_user(final_prompt_context, &draft)
            }
            LocalVideoPromptPreviewMode::Storyboard => {
                build_storyboard_video_prompt_review_user(final_prompt_context, &draft)
            }
        }?;
        let video_review_name = format!("{}_review", spec.video_prompt_name);
        let review_context_manifest = build_final_video_context_manifest(
            prompt_engine,
            project_id,
            &input.vars,
            &source,
            &[
                "chapter_video_segment_plan",
                spec.frame_prompt_name,
                spec.video_prompt_name,
                &video_review_name,
            ],
            review_user.clone(),
        )?;
        let review_request = build_video_prompt_request(
            prompt_engine,
            &video_review_name,
            project_id,
            &input.vars,
            review_context_manifest,
            0.25,
        )?;
        steps.push(placeholder_step_from_request(
            "video_prompt_review",
            &format!("{}自检修订", spec.video_title),
            &video_review_name,
            review_request,
            spec.review_placeholder_note,
        )?);
        Ok(match spec.mode {
            LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                preview_chat_video_prompt_from_steps(steps)
            }
            LocalVideoPromptPreviewMode::Storyboard => {
                preview_storyboard_video_prompt_from_steps(steps)
            }
            _ => preview_video_prompt_from_steps(steps),
        })
    }

    pub async fn preview_chapter_chat_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::chat_video_boundary_frames(),
        )
        .await
    }

    pub async fn preview_chapter_first_frame_video_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_video_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::first_frame_chain(),
        )
        .await
    }
}
