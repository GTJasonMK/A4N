#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/object_profile_records/queries.rs \
    crates/host/a4n-runtime/src/actions/scene_profile_records/queries.rs; then
    printf 'Runtime profile list actions must consume profile read ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ObjectProfileReadApplicationPort' crates/host/a4n-api-contract/src/profile.rs; then
    printf 'ObjectProfileReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait SceneProfileReadApplicationPort' crates/host/a4n-api-contract/src/profile.rs; then
    printf 'SceneProfileReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'profiles_with_timeline_state' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile list must consume CharacterProfileOverviewReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_overview_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile list must route through CharacterProfileOverviewReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|delete_character_profile_records_validated|CharacterRecordLifecycleError' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile deletion must not use ORM/profile helpers or the concrete lifecycle result\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_deletion_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile deletion must route through CharacterProfileDeletionApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileDeletionApplicationPort' \
    crates/host/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileDeletionApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileOverviewReadApplicationPort' \
    crates/host/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileOverviewReadApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|deletion_marks_by_profile_validated|deletion_mark_summary|CharacterAttributeDeletion(Execution|Request)Input' \
    crates/host/a4n-runtime/src/actions/character_records/deletion_marks.rs; then
    printf 'Runtime character deletion-mark actions must consume character read/deletion ports and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/deletion_marks.rs || \
   ! rg -n '\.deletion_marks\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/deletion_marks.rs; then
    printf 'Runtime character deletion-mark reads must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_attribute_deletion_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/deletion_marks.rs; then
    printf 'Runtime character deletion-mark mutations must route through CharacterProfileAttributeDeletionApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileAttributeDeletionApplicationPort' \
    crates/host/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileAttributeDeletionApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|CharacterTimelineRollbackLifecycleResult' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots/rollback.rs; then
    printf 'Runtime character timeline rollback must not use ORM/profile helpers or the concrete orchestrator lifecycle result\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_timeline_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots/rollback.rs; then
    printf 'Runtime character timeline rollback must route through CharacterProfileTimelineMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileTimelineMutationApplicationPort' \
    crates/host/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileTimelineMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'require_profile_for_user|state\.orchestrator\(\)|CharacterAttributeValueChangeInput' \
    crates/host/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute mutations must not use ORM/profile helpers, domain mutation inputs, or the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_attribute_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute mutations must route through CharacterProfileAttributeMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileAttributeMutationApplicationPort' \
    crates/host/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileAttributeMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'impl CharacterProfileOverviewReadApplicationPort for CharacterMutationWorkerClient' \
    crates/profiles/a4n-character-mutation-worker-client/src/lib.rs; then
    printf 'CharacterProfileOverviewReadApplicationPort worker implementation is missing\n' >&2
    exit 1
fi

if ! rg -n 'impl CharacterTimelineReadApplicationPort for CharacterMutationWorkerClient' \
    crates/profiles/a4n-character-mutation-worker-client/src/lib.rs; then
    printf 'CharacterTimelineReadApplicationPort worker implementation is missing\n' >&2
    exit 1
fi

if rg -n 'verify_project_editor_string' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile reads must leave Editor authorization in CharacterProfileOverviewReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n '\.list\(user_id, input\.project_id\)' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile list must pass identity through CharacterProfileOverviewReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'find_profile_by_name_for_user|profile_by_project_name' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile field reads must consume CharacterProfileOverviewReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_arc_by_name|voice_fingerprint_by_name' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile field reads must route through the character read port\n' >&2
    exit 1
fi

if rg -n 'create_profile_validated|update_initial_profile_validated' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile create/update actions must consume CharacterProfileMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'character_profile_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/profiles.rs; then
    printf 'Runtime character profile create/update actions must route through CharacterProfileMutationApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait CharacterProfileMutationApplicationPort' \
    crates/host/a4n-api-contract/src/character.rs; then
    printf 'CharacterProfileMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'impl CharacterProfileMutationApplicationPort for CharacterMutationWorkerClient' \
    crates/profiles/a4n-character-mutation-worker-client/src/lib.rs; then
    printf 'CharacterProfileMutationApplicationPort worker implementation is missing\n' >&2
    exit 1
fi

if rg -n 'CharacterAttributeEventHistoryFilter|attribute_event_summary' \
    crates/host/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute history must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute history must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n '\.attribute_history\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/attributes.rs; then
    printf 'Runtime character attribute history must pass identity and input through the read port\n' >&2
    exit 1
fi

if rg -n 'behavior_record_summary|behavior_records_by_profile' \
    crates/host/a4n-runtime/src/actions/character_records/behaviors.rs; then
    printf 'Runtime character behavior history must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/behaviors.rs || \
   ! rg -n '\.behavior_history\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/behaviors.rs; then
    printf 'Runtime character behavior history must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'snapshot_references_for_user_validated|snapshot_reference_summary' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot lists must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.snapshots\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot lists must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'profile_snapshot_for_user_at_chapter_validated|profile_state_summary_from_snapshot' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot state reads must consume the character read port and owner DTO projection\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.state_at_chapter\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot state reads must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'character_snapshot_for_user_by_hash_validated|character_snapshot_by_hash' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot hash reads must consume the character read port\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.snapshot_by_hash\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot hash reads must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'diff_profile_snapshots_for_user_validated|diff_profile_snapshots_for_user' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot diffs must consume the character read port\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.diff_chapters\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character snapshot diffs must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'character_sync_conflict_for_user_validated|CharacterSyncConflictReadModel' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character sync conflict reads must consume the character read port\n' >&2
    exit 1
fi

if ! rg -n 'character_timeline_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs || \
   ! rg -n '\.sync_conflict\(user_id, input\)' \
    crates/host/a4n-runtime/src/actions/character_records/snapshots.rs; then
    printf 'Runtime character sync conflict reads must route through CharacterTimelineReadApplicationPort\n' >&2
    exit 1
fi

if rg -n 'a4n_orchestrator|OrchestratorContext|state\.orchestrator\(\)' \
    crates/host/a4n-runtime/src/actions/object_profile_records/mutations.rs \
    crates/host/a4n-runtime/src/actions/scene_profile_records/mutations.rs; then
    printf 'Runtime profile mutation actions must consume profile mutation ports instead of the concrete orchestrator facade\n' >&2
    exit 1
fi

if ! rg -n 'ObjectProfileWorkerClient::from_env\(\)' \
    crates/host/a4n-runtime/src/actions/object_profile_records/mutations.rs; then
    printf 'Runtime object profile mutation actions must route through ObjectProfileWorkerClient\n' >&2
    exit 1
fi

if ! rg -n 'SceneProfileWorkerClient::from_env\(\)' \
    crates/host/a4n-runtime/src/actions/scene_profile_records/mutations.rs; then
    printf 'Runtime scene profile mutation actions must route through SceneProfileWorkerClient\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ObjectProfileMutationApplicationPort' crates/host/a4n-api-contract/src/profile.rs; then
    printf 'ObjectProfileMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if ! rg -n 'pub trait SceneProfileMutationApplicationPort' crates/host/a4n-api-contract/src/profile.rs; then
    printf 'SceneProfileMutationApplicationPort contract is missing\n' >&2
    exit 1
fi

if rg -n 'state\.orchestrator\(\)|OrchestratorContext' \
    crates/host/a4n-runtime/src/actions/profile_materials/list.rs \
    crates/host/a4n-runtime/src/actions/profile_materials/primary.rs \
    crates/host/a4n-runtime/src/actions/profile_materials/upload/api.rs; then
    printf 'Runtime profile-material actions must consume split profile-material ports\n' >&2
    exit 1
fi

if ! rg -n 'profile_material_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/profile_materials/list.rs \
    crates/host/a4n-runtime/src/actions/profile_materials/primary.rs || \
   ! rg -n 'profile_material_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/profile_materials/primary.rs; then
    printf 'Runtime profile-material record actions must route through split API profile-material ports\n' >&2
    exit 1
fi

if ! rg -n 'profile_material_upload_application\(\)' \
    crates/host/a4n-runtime/src/actions/profile_materials/upload/api.rs; then
    printf 'Runtime profile-material upload must route through ProfileMaterialUploadApplicationPort\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ProfileMaterial(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/profile.rs || \
   ! rg -n 'pub trait ProfileMaterialUploadApplicationPort' \
    crates/image/a4n-image-host-contract/src/lib.rs; then
    printf 'Profile-material split contracts are missing\n' >&2
    exit 1
fi

if rg -n 'ForeshadowingApplicationPort|foreshadowing_application\(\)' \
    crates/host/a4n-api-contract/src/profile.rs \
    crates/host/a4n-orchestrator/src \
    crates/host/a4n-runtime/src; then
    printf 'The mixed ForeshadowingApplicationPort must remain removed\n' >&2
    exit 1
fi

if ! rg -n 'foreshadowing_read_application\(\)' \
    crates/host/a4n-runtime/src/actions/foreshadowing_records.rs || \
   ! rg -n 'foreshadowing_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/foreshadowing_records.rs; then
    printf 'Runtime foreshadowing actions must route through split read and mutation ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait Foreshadowing(Read|Mutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/profile.rs; then
    printf 'Foreshadowing split contracts are missing\n' >&2
    exit 1
fi

if rg -n 'shooting_script_application\(\)|ShootingScriptApplicationPort' \
    crates/host/a4n-runtime/src crates/host/a4n-orchestrator/src crates/host/a4n-orchestrator/src/lib.rs; then
    printf 'Shooting-script records and generation must use split application ports\n' >&2
    exit 1
fi

if ! rg -n 'shooting_script_unit_progress_application\(\)' \
    crates/host/a4n-runtime/src/actions/shooting_script_records/read.rs; then
    printf 'Runtime shooting-script progress reads must route through the progress port\n' >&2
    exit 1
fi

if ! rg -n 'shooting_script_record_mutation_application\(\)' \
    crates/host/a4n-runtime/src/actions/shooting_script_records/edit.rs; then
    printf 'Runtime shooting-script record edits must route through the mutation port\n' >&2
    exit 1
fi

if ! rg -n 'shooting_script_full_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/shooting_script_records/generation.rs || \
   ! rg -n 'shooting_script_next_unit_generation_application\(\)' \
    crates/host/a4n-runtime/src/actions/shooting_script_records/generation.rs; then
    printf 'Runtime shooting-script generation actions must route through worker-specific ports\n' >&2
    exit 1
fi

if ! rg -n 'pub trait ShootingScript(FullGeneration|NextUnitGeneration)ApplicationPort' \
    crates/project/a4n-novel-host-contract/src/lib.rs || \
   ! rg -n 'pub trait ShootingScript(UnitProgressRead|RecordMutation)ApplicationPort' \
    crates/host/a4n-api-contract/src/shooting_script.rs; then
    printf 'ShootingScriptRecord read/mutation contracts are missing\n' >&2
    exit 1
fi

if rg -n 'ShootingScriptRecordApplicationPort' \
    crates/host/a4n-api-contract/src/shooting_script.rs \
    crates/host/a4n-orchestrator/src crates/host/a4n-runtime/src; then
    printf 'The mixed ShootingScriptRecordApplicationPort must remain removed\n' >&2
    exit 1
fi
