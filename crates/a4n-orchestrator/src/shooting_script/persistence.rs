use a4n_core::{error::AppError, shooting_script::ChapterShootingScriptV1};
use a4n_db::shooting_script::{
    EditStoredChapterShootingScriptInput, InsertChapterShootingScriptInput,
    ParsedChapterShootingScript, UpdateChapterShootingScriptInput,
};

use crate::OrchestratorContext;

use super::{
    contracts::{
        ChapterShootingScriptSourceInput, EditChapterShootingScriptInput,
        EditedChapterShootingScript, ResolvedChapterShootingScript,
    },
    errors::{resolve_source_error, shooting_script_db_error},
};

impl OrchestratorContext {
    pub(super) async fn load_chapter_shooting_script_source(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<
        (
            a4n_db::project::ResolvedChapterSource,
            Option<ParsedChapterShootingScript>,
        ),
        AppError,
    > {
        let source = a4n_db::project::resolve_chapter_source(
            self.db.connection(),
            input.project_id,
            input.source_chapter_id,
        )
        .await
        .map_err(resolve_source_error)?;
        let existing = a4n_db::shooting_script::chapter_shooting_script_for_source(
            self.db.connection(),
            source.root_project_id,
            source.source_chapter_id,
            &source.source_revision_key,
        )
        .await
        .map_err(shooting_script_db_error)?;
        Ok((source, existing))
    }

    pub(super) async fn publish_chapter_shooting_script(
        &self,
        source: a4n_db::project::ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        input_fingerprint_sha256: String,
        script: ChapterShootingScriptV1,
        workflow_run_id: i64,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let shooting_script = match existing {
            Some(existing) => a4n_db::shooting_script::update_chapter_shooting_script(
                self.db.connection(),
                UpdateChapterShootingScriptInput {
                    record_id: existing.record.id,
                    expected_revision: existing.record.revision,
                    input_fingerprint_sha256,
                    script,
                    workflow_run_id: Some(workflow_run_id),
                },
            )
            .await
            .map_err(shooting_script_db_error)?,
            None => a4n_db::shooting_script::insert_chapter_shooting_script(
                self.db.connection(),
                InsertChapterShootingScriptInput {
                    source: source.clone(),
                    input_fingerprint_sha256,
                    script,
                    workflow_run_id: Some(workflow_run_id),
                },
            )
            .await
            .map_err(shooting_script_db_error)?,
        };
        Ok(ResolvedChapterShootingScript {
            source,
            shooting_script,
        })
    }

    pub async fn edit_chapter_shooting_script(
        &self,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScript, AppError> {
        input
            .script
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let outcome = a4n_db::shooting_script::edit_chapter_shooting_script_and_invalidate(
            self.db.connection(),
            EditStoredChapterShootingScriptInput {
                record_id: input.record_id,
                expected_revision: input.expected_revision,
                script: input.script,
            },
        )
        .await
        .map_err(shooting_script_db_error)?;
        Ok(EditedChapterShootingScript {
            shooting_script: outcome.shooting_script,
            invalidated_project_ids: outcome.invalidated_project_ids,
        })
    }
}
