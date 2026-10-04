use super::*;

use a4n_domain::project::{OptimizationCharacterProfileRecord, ProjectOptimizationContextRecord};

use crate::outline::record_mapping::part_outline_record_from_model;

fn project_optimization_context_record(
    project: novel_project::Model,
) -> ProjectOptimizationContextRecord {
    ProjectOptimizationContextRecord {
        id: project.id,
        title: project.title,
        description: project.description,
        project_type: project.project_type,
        status: project.status,
        target_chapters: project.target_chapters,
        current_chapter: project.current_chapter,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        raw_blueprint: project.raw_blueprint,
        export_config: project.export_config,
    }
}

fn optimization_character_profile_record(
    profile: protagonist_profile::Model,
) -> OptimizationCharacterProfileRecord {
    OptimizationCharacterProfileRecord {
        name: profile.name,
        gender: profile.gender,
        age: profile.age,
        base_attributes: profile.base_attributes,
        current_state: profile.current_state,
        character_arc: profile.character_arc,
        voice_fingerprint: profile.voice_fingerprint,
        appearance: profile.appearance,
        background: profile.background,
        personality: profile.personality,
        last_synced_chapter: profile.last_synced_chapter,
    }
}

pub async fn optimization_chapter_by_id<C>(
    db: &C,
    chapter_id: i64,
) -> Result<Option<a4n_domain::chapter::ChapterRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    chapter::Entity::find_by_id(chapter_id)
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .one(db)
        .await
        .map(|chapter| chapter.map(crate::project::chapter_record))
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_context_rows<C>(
    db: &C,
    input: OptimizationContextReadInput,
) -> Result<Option<OptimizationContextRows>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let Some(current_chapter) = optimization_chapter_by_id(db, input.chapter_id).await? else {
        return Ok(None);
    };
    if current_chapter.project_id != input.project_id {
        return Err(OptimizeDbError::InvalidArgument(
            "内容优化章节不属于当前项目".into(),
        ));
    }

    let current_chapter_outline = if !input.read_outlines {
        None
    } else {
        match current_chapter.outline_id {
            Some(outline_id) => chapter_outline::Entity::find_by_id(outline_id)
                .select_only()
                .column(chapter_outline::Column::ChapterNumber)
                .column(chapter_outline::Column::Title)
                .column(chapter_outline::Column::Summary)
                .column(chapter_outline::Column::Beats)
                .column(chapter_outline::Column::KeyScenes)
                .column(chapter_outline::Column::PacingScore)
                .column(chapter_outline::Column::ForeshadowingSetup)
                .column(chapter_outline::Column::ForeshadowingPayoff)
                .column(chapter_outline::Column::CharactersInvolved)
                .column(chapter_outline::Column::WordCountTarget)
                .into_model::<OptimizationChapterOutlineRecord>()
                .one(db)
                .await
                .map_err(OptimizeDbError::Database)?,
            None => chapter_outline::Entity::find()
                .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
                .filter(chapter_outline::Column::ChapterNumber.eq(current_chapter.chapter_number))
                .select_only()
                .column(chapter_outline::Column::ChapterNumber)
                .column(chapter_outline::Column::Title)
                .column(chapter_outline::Column::Summary)
                .column(chapter_outline::Column::Beats)
                .column(chapter_outline::Column::KeyScenes)
                .column(chapter_outline::Column::PacingScore)
                .column(chapter_outline::Column::ForeshadowingSetup)
                .column(chapter_outline::Column::ForeshadowingPayoff)
                .column(chapter_outline::Column::CharactersInvolved)
                .column(chapter_outline::Column::WordCountTarget)
                .into_model::<OptimizationChapterOutlineRecord>()
                .one(db)
                .await
                .map_err(OptimizeDbError::Database)?,
        }
    };

    let project_id = input.project_id;
    let read_project_detail = input.read_project_detail;
    let read_blueprint = input.read_blueprint;
    let read_outlines = input.read_outlines;
    let read_character_profiles = input.read_character_profiles;
    let read_foreshadowing = input.read_foreshadowing;
    let chapter_number = current_chapter.chapter_number;

    // These projections have no dependency on one another. Keep the current chapter and its
    // outline as the only serial reads, then let the pool overlap the remaining round trips.
    let (
        project_result,
        blueprint_result,
        part_outlines_result,
        chapter_outlines_result,
        character_profiles_result,
        foreshadowing_result,
        previous_chapters_result,
        next_chapters_result,
    ) = futures::join!(
        async {
            if read_project_detail {
                novel_project::Entity::find_by_id(project_id)
                    .one(db)
                    .await
                    .map(|project| project.map(project_optimization_context_record))
                    .map_err(OptimizeDbError::Database)
            } else {
                Ok::<Option<ProjectOptimizationContextRecord>, OptimizeDbError>(None)
            }
        },
        async {
            if !read_blueprint {
                return Ok::<Option<OptimizationBlueprintRows>, OptimizeDbError>(None);
            }
            let Some(blueprint) = novel_blueprint::Entity::find()
                .filter(novel_blueprint::Column::ProjectId.eq(project_id))
                .one(db)
                .await
                .map_err(OptimizeDbError::Database)?
            else {
                return Ok(None);
            };
            let (characters_result, relationships_result) = futures::join!(
                blueprint_character::Entity::find()
                    .filter(blueprint_character::Column::ProjectId.eq(project_id))
                    .all(db),
                blueprint_relationship::Entity::find()
                    .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                    .all(db),
            );
            let characters = characters_result.map_err(OptimizeDbError::Database)?;
            let relationships = relationships_result.map_err(OptimizeDbError::Database)?;
            Ok(Some(crate::blueprint::context::blueprint_llm_context(
                &blueprint,
                &characters,
                &relationships,
            )))
        },
        async {
            if read_outlines {
                part_outline::Entity::find()
                    .filter(part_outline::Column::ProjectId.eq(project_id))
                    .order_by_asc(part_outline::Column::PartNumber)
                    .all(db)
                    .await
                    .map_err(OptimizeDbError::Database)
            } else {
                Ok::<Vec<part_outline::Model>, OptimizeDbError>(Vec::new())
            }
        },
        async {
            if read_outlines {
                chapter_outline::Entity::find()
                    .select_only()
                    .column(chapter_outline::Column::ChapterNumber)
                    .column(chapter_outline::Column::Title)
                    .column(chapter_outline::Column::Summary)
                    .column(chapter_outline::Column::Beats)
                    .column(chapter_outline::Column::KeyScenes)
                    .column(chapter_outline::Column::PacingScore)
                    .column(chapter_outline::Column::ForeshadowingSetup)
                    .column(chapter_outline::Column::ForeshadowingPayoff)
                    .column(chapter_outline::Column::CharactersInvolved)
                    .column(chapter_outline::Column::WordCountTarget)
                    .filter(chapter_outline::Column::ProjectId.eq(project_id))
                    .order_by_asc(chapter_outline::Column::ChapterNumber)
                    .into_model::<OptimizationChapterOutlineRecord>()
                    .all(db)
                    .await
                    .map_err(OptimizeDbError::Database)
            } else {
                Ok::<Vec<OptimizationChapterOutlineRecord>, OptimizeDbError>(Vec::new())
            }
        },
        async {
            if read_character_profiles {
                protagonist_profile::Entity::find()
                    .filter(protagonist_profile::Column::ProjectId.eq(project_id))
                    .order_by_asc(protagonist_profile::Column::Id)
                    .all(db)
                    .await
                    .map(|profiles| {
                        profiles
                            .into_iter()
                            .map(optimization_character_profile_record)
                            .collect()
                    })
                    .map_err(OptimizeDbError::Database)
            } else {
                Ok::<Vec<OptimizationCharacterProfileRecord>, OptimizeDbError>(Vec::new())
            }
        },
        async {
            if read_foreshadowing {
                foreshadowing_index::Entity::find()
                    .select_only()
                    .column(foreshadowing_index::Column::Id)
                    .column(foreshadowing_index::Column::Description)
                    .column(foreshadowing_index::Column::SetupChapter)
                    .column(foreshadowing_index::Column::PayoffChapter)
                    .column(foreshadowing_index::Column::Status)
                    .column(foreshadowing_index::Column::RelatedCharacters)
                    .column(foreshadowing_index::Column::Importance)
                    .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
                    .order_by_asc(foreshadowing_index::Column::SetupChapter)
                    .into_model::<crate::rag::RagForeshadowingRow>()
                    .all(db)
                    .await
                    .map_err(OptimizeDbError::Database)
            } else {
                Ok::<Vec<crate::rag::RagForeshadowingRow>, OptimizeDbError>(Vec::new())
            }
        },
        async {
            let mut chapters = chapter::Entity::find()
                .filter(chapter::Column::ProjectId.eq(project_id))
                .filter(chapter::Column::Status.ne("deleted"))
                .filter(chapter::Column::ChapterNumber.lt(chapter_number))
                .select_only()
                .column(chapter::Column::ChapterNumber)
                .column(chapter::Column::Title)
                .column(chapter::Column::Summary)
                .column(chapter::Column::WordCount)
                .column(chapter::Column::AnalysisData)
                .order_by_desc(chapter::Column::ChapterNumber)
                .limit(3)
                .into_model::<OptimizationChapterSummaryRecord>()
                .all(db)
                .await
                .map_err(OptimizeDbError::Database)?;
            chapters.reverse();
            Ok::<Vec<OptimizationChapterSummaryRecord>, OptimizeDbError>(chapters)
        },
        async {
            chapter::Entity::find()
                .filter(chapter::Column::ProjectId.eq(project_id))
                .filter(chapter::Column::Status.ne("deleted"))
                .filter(chapter::Column::ChapterNumber.gt(chapter_number))
                .select_only()
                .column(chapter::Column::ChapterNumber)
                .column(chapter::Column::Title)
                .column(chapter::Column::Summary)
                .column(chapter::Column::WordCount)
                .column(chapter::Column::AnalysisData)
                .order_by_asc(chapter::Column::ChapterNumber)
                .limit(2)
                .into_model::<OptimizationChapterSummaryRecord>()
                .all(db)
                .await
                .map_err(OptimizeDbError::Database)
        },
    );
    let project = project_result?;
    let blueprint = blueprint_result?;
    let part_outlines = part_outlines_result?
        .into_iter()
        .map(part_outline_record_from_model)
        .collect();
    let chapter_outlines = chapter_outlines_result?;
    let character_profiles = character_profiles_result?;
    let foreshadowing_index = foreshadowing_result?;
    let previous_chapters = previous_chapters_result?;
    let next_chapters = next_chapters_result?;

    Ok(Some(OptimizationContextRows {
        current_chapter,
        current_chapter_outline,
        project,
        blueprint,
        part_outlines,
        chapter_outlines,
        character_profiles,
        foreshadowing_index,
        previous_chapters,
        next_chapters,
    }))
}

pub async fn write_optimization_chapter_patch_content(
    db: &impl crate::DbTransactionExecutor,
    input: OptimizationChapterPatchWriteInput,
) -> Result<a4n_domain::chapter::ChapterRecord, OptimizeDbError> {
    if !crate::chapter_content::chapter_source_snapshots_match_for_update(
        db,
        &[input.chapter.source_snapshot()],
    )
    .await?
    {
        return Err(OptimizeDbError::InvalidArgument(
            "应用补丁前章节正文已变化，请刷新后重新生成补丁".into(),
        ));
    }
    let mut active = chapter::ActiveModel {
        id: Set(input.chapter.id),
        ..Default::default()
    };
    crate::chapter_content::assign_chapter_content(
        &mut active,
        input.content,
        input.word_count,
        chapter::STATUS_DRAFT,
        None,
        chrono::Utc::now().naive_utc(),
    );
    active
        .update(db)
        .await
        .map(crate::project::chapter_record)
        .map_err(OptimizeDbError::Database)
}
