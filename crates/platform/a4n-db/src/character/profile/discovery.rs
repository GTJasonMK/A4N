use std::collections::{BTreeMap, BTreeSet};

use a4n_domain::{
    chapter::ChapterSourceSnapshot,
    character::{CharacterDiscoveryCandidateRecordInput, CharacterDiscoveryCandidateRecordOutcome},
    text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind},
};
use sea_orm::{ConnectionTrait, TransactionTrait};

use crate::character::{CharacterDbError, profile_record_by_id, profile_record_by_project_name};

pub async fn persist_character_discovery_candidates<C>(
    db: &C,
    project_id: i64,
    source_chapters: &[ChapterSourceSnapshot],
    candidates: Vec<CharacterDiscoveryCandidateRecordInput>,
) -> Result<Vec<CharacterDiscoveryCandidateRecordOutcome>, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if project_id <= 0
        || source_chapters.is_empty()
        || source_chapters
            .iter()
            .any(|source| source.project_id != project_id)
    {
        return Err(CharacterDbError::InvalidArgument(
            "角色发现缺少有效章节来源快照".to_string(),
        ));
    }

    let txn = db.begin().await?;
    let project = crate::project::project_write_identity_for_update(&txn, project_id)
        .await?
        .ok_or_else(|| CharacterDbError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(CharacterDbError::InvalidState {
            current: project.lifecycle_gate,
            operation: "保存角色发现结果".to_string(),
        });
    }
    if !crate::chapter_content::chapter_source_snapshots_match_for_update(&txn, source_chapters)
        .await?
    {
        return Err(CharacterDbError::InvalidState {
            current: "chapter_source_changed".to_string(),
            operation: "保存角色发现结果；章节正文已变化，请重新分析".to_string(),
        });
    }

    let mut outcomes: Vec<CharacterDiscoveryCandidateRecordOutcome> =
        Vec::with_capacity(candidates.len());
    let mut evidence_by_profile = BTreeMap::<i64, BTreeSet<i64>>::new();
    for (candidate_index, candidate) in candidates.into_iter().enumerate() {
        let (profile, created, evidence_anchor_id) = match candidate {
            CharacterDiscoveryCandidateRecordInput::ExistingProfile {
                profile_id,
                evidence_anchor_id,
            } => {
                let profile = profile_record_by_id(&txn, profile_id)
                    .await?
                    .ok_or_else(|| {
                        CharacterDbError::NotFound(format!(
                            "角色档案不存在: profile_id={profile_id}"
                        ))
                    })?;
                ensure_profile_project(&profile, project_id)?;
                (Some(profile), false, Some(evidence_anchor_id))
            }
            CharacterDiscoveryCandidateRecordInput::ExistingWithoutEvidence => (None, false, None),
            CharacterDiscoveryCandidateRecordInput::PreviouslyResolvedCandidate {
                candidate_index: source_index,
                evidence_anchor_id,
            } => {
                let profile = outcomes
                    .get(source_index)
                    .filter(|_| source_index < candidate_index)
                    .and_then(|outcome| outcome.profile.clone())
                    .ok_or_else(|| {
                        CharacterDbError::InvalidArgument(format!(
                            "角色发现引用了无效的先前候选: index={source_index}, candidate={candidate_index}"
                        ))
                    })?;
                (Some(profile), false, Some(evidence_anchor_id))
            }
            CharacterDiscoveryCandidateRecordInput::Create {
                name,
                base_attributes,
                evidence_anchor_id,
                generation_run,
            } => match super::create_profile_with_generation_run_in(
                &txn,
                project_id,
                &name,
                base_attributes,
                generation_run,
            )
            .await
            {
                Ok(profile) => (
                    Some(crate::character::character_profile_record(profile)),
                    true,
                    Some(evidence_anchor_id),
                ),
                Err(super::CharacterProfileCreateError::AlreadyExists { name }) => {
                    let profile = profile_record_by_project_name(&txn, project_id, &name)
                        .await?
                        .ok_or_else(|| {
                            CharacterDbError::NotFound(format!(
                                "角色档案已存在但无法重新读取: project_id={project_id}, name={name}"
                            ))
                        })?;
                    (Some(profile), false, Some(evidence_anchor_id))
                }
                Err(super::CharacterProfileCreateError::InvalidArgument(message)) => {
                    return Err(CharacterDbError::InvalidArgument(message));
                }
                Err(super::CharacterProfileCreateError::InvalidState { current, operation }) => {
                    return Err(CharacterDbError::InvalidState { current, operation });
                }
                Err(super::CharacterProfileCreateError::Database(error)) => {
                    return Err(CharacterDbError::Database(error));
                }
                Err(super::CharacterProfileCreateError::Generation(error)) => {
                    return Err(CharacterDbError::Generation(error));
                }
            },
        };

        if let (Some(profile), Some(anchor_id)) = (&profile, evidence_anchor_id) {
            if anchor_id <= 0 {
                return Err(CharacterDbError::InvalidArgument(
                    "角色发现 evidence_anchor_id 必须大于 0".to_string(),
                ));
            }
            evidence_by_profile
                .entry(profile.id)
                .or_default()
                .insert(anchor_id);
        }
        outcomes.push(CharacterDiscoveryCandidateRecordOutcome { profile, created });
    }

    if !evidence_by_profile.is_empty() {
        let evidence = evidence_by_profile
            .into_iter()
            .map(|(profile_id, anchor_ids)| {
                (
                    profile_id,
                    anchor_ids
                        .into_iter()
                        .map(|text_anchor_id| TextAnchorEvidenceInput {
                            text_anchor_id,
                            evidence_role: crate::text_anchor::EVIDENCE_ROLE_FIRST_SEEN.to_string(),
                        })
                        .collect(),
                )
            })
            .collect();
        crate::text_anchor::merge_profile_evidence_records_batch(
            &txn,
            project_id,
            None,
            TextAnchorProfileKind::Character,
            evidence,
        )
        .await
        .map_err(character_text_anchor_error)?;
    }

    txn.commit().await?;
    Ok(outcomes)
}

fn ensure_profile_project(
    profile: &a4n_domain::character::CharacterProfileRecord,
    project_id: i64,
) -> Result<(), CharacterDbError> {
    if profile.project_id == project_id {
        Ok(())
    } else {
        Err(CharacterDbError::InvalidArgument(format!(
            "角色发现 profile/project 不匹配: profile_id={}, expected_project={project_id}, actual_project={}",
            profile.id, profile.project_id
        )))
    }
}

fn character_text_anchor_error(error: crate::text_anchor::TextAnchorDbError) -> CharacterDbError {
    match error {
        crate::text_anchor::TextAnchorDbError::Database(error) => CharacterDbError::Database(error),
        crate::text_anchor::TextAnchorDbError::NotFound(message) => {
            CharacterDbError::NotFound(message)
        }
        crate::text_anchor::TextAnchorDbError::InvalidArgument(message) => {
            CharacterDbError::InvalidArgument(message)
        }
    }
}
