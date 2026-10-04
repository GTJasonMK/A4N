#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

for command in rg; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "Required command not found: $command" >&2
        exit 127
    }
done

for command in cargo; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "Required command not found: $command" >&2
        exit 127
    }
done

violations=0

check_source() {
    local label="$1"
    local directory="$2"
    local pattern="$3"
    local hits

    if hits="$(rg --pcre2 --line-number --with-filename --no-heading \
        --glob '*.rs' --glob '!**/tests/**' --glob '!**/*test*.rs' "$pattern" "$PROJECT_ROOT/$directory" 2>/dev/null)"; then
        echo "FAIL: $label:" >&2
        printf '  %s\n' "$hits" >&2
        violations=$((violations + 1))
    fi
}

check_normal_dependencies() {
    local label="$1"
    local package="$2"
    local pattern="$3"
    local tree

    if tree="$(cargo tree --offline --locked --edges normal --depth 1 -p "$package" 2>&1)"; then
        if rg --pcre2 --line-number "$pattern" <<<"$tree"; then
            echo "FAIL: $label has an ORM normal dependency." >&2
            violations=$((violations + 1))
        fi
    else
        echo "FAIL: could not inspect $label normal dependencies:" >&2
        printf '  %s\n' "$tree" >&2
        violations=$((violations + 1))
    fi
}

# Core code may depend on repository contracts, but it must not expose the
# concrete SeaORM transaction aliases used by the adapters.
check_source \
    "chapter-content core exposes a concrete ORM transaction" \
    "crates/chapter/a4n-chapter-content-core/src" \
    '(?<![[:alnum:]_])(?:\bDbTransaction\b|\bDatabaseTransaction\b|a4n_db_models::)'
check_source \
    "chapter-content core imports SeaORM directly" \
    "crates/chapter/a4n-chapter-content-core/src" \
    '(?:sea_orm::|use[[:space:]]+sea_orm|\b(?:DatabaseConnection|DatabaseTransaction|DbErr)\b)'
check_source \
    "project-read core imports ORM types" \
    "crates/project/a4n-project-read-core/src" \
    '(?:a4n_db_models::|sea_orm::|use[[:space:]]+sea_orm|\b(?:DatabaseConnection|DatabaseTransaction|DbErr)\b)'
check_normal_dependencies \
    "a4n-chapter-content-core" \
    "a4n-chapter-content-core" \
    '(?:a4n-db-models|sea-orm)'
check_normal_dependencies \
    "a4n-project-read-core" \
    "a4n-project-read-core" \
    '(?:a4n-db-models|sea-orm)'
check_source \
    "Orchestrator production code imports SeaORM directly" \
    "crates/host/a4n-orchestrator/src" \
    '(?:sea_orm::|use[[:space:]]+sea_orm)'

check_source \
    "Agent application production code imports ORM types" \
    "crates/agent/a4n-agent-application/src" \
    '(?:a4n_db_models::|sea_orm::|\b(?:DatabaseConnection|DatabaseTransaction|DbErr)\b)'
check_source \
    "Auth application production code imports ORM types" \
    "crates/auth/a4n-auth-application/src" \
    '(?:a4n_db_models::|sea_orm::|\b(?:DatabaseConnection|DatabaseTransaction|DbErr)\b)'

for package in \
    a4n-agent-application \
    a4n-auth-application \
    a4n-runtime; do
    check_normal_dependencies \
        "$package" \
        "$package" \
        '(?:a4n-db-models|sea-orm)'
done

# Host orchestration may depend on the DB executor contract, but it must not
# name the concrete connection, transaction, or driver error types.
if hits="$(rg --pcre2 --line-number --with-filename --no-heading \
    --glob '*.rs' --glob '!**/tests/**' --glob '!**/*test*.rs' \
    --glob '!**/shooting_script/contracts.rs' \
    '\b(?:DatabaseConnection|SharedDatabaseConnection|DatabaseTransaction|DbErr)\b' \
    "$PROJECT_ROOT/crates/host/a4n-orchestrator/src" 2>/dev/null)"; then
    echo "FAIL: Orchestrator production code names concrete database types:" >&2
    printf '  %s\n' "$hits" >&2
    violations=$((violations + 1))
fi

# These files contain inline cfg(test) ORM fixtures; a4n-db-models is a
# dev-dependency only in Orchestrator.
if hits="$(rg --pcre2 --line-number --with-filename --no-heading \
    --glob '*.rs' --glob '!**/tests/**' --glob '!**/*test*.rs' \
    --glob '!**/project/family_package/mod.rs' \
    --glob '!**/project/chapters/records.rs' \
    --glob '!**/shooting_script/contracts.rs' \
    'a4n_db_models::' "$PROJECT_ROOT/crates/host/a4n-orchestrator/src" 2>/dev/null)"; then
    echo "FAIL: Orchestrator production code imports database models directly:" >&2
    printf '  %s\n' "$hits" >&2
    violations=$((violations + 1))
fi

if model_tree="$(cargo tree --offline --locked --edges normal --depth 1 -p a4n-orchestrator 2>&1)"; then
    if rg --quiet 'a4n-db-models' <<<"$model_tree"; then
        echo "FAIL: a4n-orchestrator has a normal dependency on a4n-db-models." >&2
        violations=$((violations + 1))
    fi
else
    echo "FAIL: could not inspect Orchestrator normal dependencies:" >&2
    printf '  %s\n' "$model_tree" >&2
    violations=$((violations + 1))
fi

# These application slices use domain records at the Host boundary.
check_source \
    "Optimize application exposes optimization_session ORM Model" \
    "crates/host/a4n-orchestrator/src/optimize" \
    '(?:optimization_session::Model|a4n_db_models::models::ai::optimization_session(?:[^[:alnum:]_]|$))'
check_source \
    "Project Family exposes chapter_video_prompt ORM Model" \
    "crates/host/a4n-orchestrator/src/project/family_package" \
    '(?:chapter_video_prompt::Model|a4n_db_models::models::chapter::chapter_video_prompt)'
check_source \
    "Video application imports chapter_video_prompt ORM module" \
    "crates/host/a4n-orchestrator/src/video" \
    'a4n_db_models::models::chapter::chapter_video_prompt'
check_source \
    "Video application exposes chapter ORM Model" \
    "crates/host/a4n-orchestrator/src/video/access.rs" \
    '(?:chapter::Model|a4n_db_models::models::chapter::chapter)'
check_source \
    "Video source application exposes chapter ORM Model" \
    "crates/host/a4n-orchestrator/src/video/source.rs" \
    '(?:chapter::Model|a4n_db_models::models::chapter::chapter)'
check_source \
    "Video prompt application exposes chapter ORM Model" \
    "crates/host/a4n-orchestrator/src/video/prompts/application.rs" \
    '(?:chapter::Model|a4n_db_models::models::chapter::chapter)'

# Provider configuration, timeline, and image-publication application owners
# expose domain projections only. Their database adapters may still use the
# ORM internally, but Host code must not carry those rows across the owner.
check_source \
    "Provider configuration owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/context/settings" \
    '(?:a4n_db_models::|(?:llm_config|embedding_config|image_gen_config|video_gen_config)::Model)'
check_source \
    "Dynamic provider owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/context/dynamic_provider.rs" \
    '(?:a4n_db_models::|(?:llm_config|embedding_config)::Model)'
check_source \
    "Video timeline owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/context/video" \
    '(?:chapter_video_timeline(?:::[[:space:]]*)?::Model|a4n_db_models::models::chapter::chapter_video_timeline)'
check_source \
    "Video timeline workflow exposes ORM item models" \
    "crates/host/a4n-orchestrator/src/video/timeline" \
    '(?:chapter_video_timeline_item::Model|a4n_db_models::models::chapter::chapter_video_timeline_item)'
check_source \
    "Chapter video generation exposes ORM timeline items" \
    "crates/host/a4n-orchestrator/src/video/chapter_video_generation" \
    '(?:chapter_video_timeline_item::Model|a4n_db_models::models::chapter::chapter_video_timeline_item)'
check_source \
    "Image publication owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/context/video" \
    '(?:image_publication::Model|a4n_db_models::models::media::image_publication)'
check_source \
    "Image publication workflow exposes ORM models" \
    "crates/host/a4n-orchestrator/src/media/image" \
    '(?:image_publication::Model|a4n_db_models::models::media::image_publication)'
check_source \
    "Scene profile sync exposes ORM input models" \
    "crates/host/a4n-orchestrator/src/scene_profile" \
    '(?:a4n_db_models::|(?:chapter|text_anchor|protagonist_profile|object_profile)::Model)'
check_source \
    "Source sync application exposes run/conflict ORM models" \
    "crates/host/a4n-orchestrator/src/context/source_sync.rs" \
    '(?:source_sync_(?:run|conflict)::Model|a4n_db_models::models::source_sync)'
check_source \
    "Source sync decision preparation exposes run/conflict ORM models" \
    "crates/host/a4n-orchestrator/src/source_sync" \
    '(?:source_sync_(?:run|conflict)::Model|a4n_db_models::models::source_sync)'

# Galgame generation, portrait, script mutation, and video-studio transfer
# owners expose domain projections. ORM models remain adapter/test details.
check_source \
    "Galgame context exposes ORM models" \
    "crates/host/a4n-orchestrator/src/context/galgame.rs" \
    '(?:a4n_db_models::|(?:generation_spec|prompt_artifact|character_portrait|generated_image)::Model)'
check_source \
    "Galgame script mutation exposes ORM models" \
    "crates/host/a4n-orchestrator/src/galgame/mutations" \
    '(?:galgame_(?:line|scene|choice)::Model|a4n_db_models::models::galgame)'
check_source \
    "Galgame script owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/galgame/context.rs" \
    '(?:galgame_(?:line|scene|choice)::Model|a4n_db_models::models::galgame)'
check_source \
    "Galgame script record owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/galgame/script_records.rs" \
    '(?:galgame_(?:line|scene|choice)::Model|a4n_db_models::models::galgame)'
check_source \
    "Portrait application exposes ORM models" \
    "crates/host/a4n-orchestrator/src/portrait" \
    '(?:character_portrait::Model|protagonist_profile::Model|a4n_db_models::models::character)'
check_source \
    "Object discovery owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/object_discovery" \
    '(?:a4n_db_models::|(?:chapter|text_anchor|object_profile)::Model|DatabaseTransaction|sea_orm::)'
for discovery_file in \
    contracts.rs context.rs execution.rs identity.rs persistence.rs request.rs output.rs rules.rs workflow.rs; do
    check_source \
        "Character discovery pure owner exposes ORM models: $discovery_file" \
        "crates/host/a4n-orchestrator/src/character/discovery/$discovery_file" \
        '(?:a4n_db_models::|(?:chapter|text_anchor|protagonist_profile)::Model|DatabaseTransaction|sea_orm::)'
done
check_source \
    "Character profile sync application exposes ORM models" \
    "crates/host/a4n-orchestrator/src/character/profile_sync" \
    '(?:a4n_db_models::|(?:chapter|text_anchor|protagonist_profile)::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Character sync rules expose ORM models" \
    "crates/host/a4n-orchestrator/src/character/sync_rules.rs" \
    '(?:a4n_db_models::|(?:chapter|protagonist_profile)::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Character conflict rules expose ORM models" \
    "crates/host/a4n-orchestrator/src/character/conflict" \
    '(?:a4n_db_models::|(?:chapter|attribute_event|character_state_index)::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Character mutation worker exposes ORM models" \
    "crates/profiles/a4n-character-mutation-worker/src" \
    '(?:a4n_db_models::|::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Import reverse candidate application exposes ORM transaction or rows" \
    "crates/host/a4n-orchestrator/src/import_reverse/candidate.rs" \
    '(?:a4n_db_models::|(?:workflow_run|workflow_checkpoint)::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Import reverse chapter context rules expose ORM models" \
    "crates/host/a4n-orchestrator/src/import_reverse/context/chapter_entries" \
    '(?:a4n_db_models::|chapter::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Import reverse source extraction rules expose ORM models" \
    "crates/host/a4n-orchestrator/src/import_reverse/context/extraction" \
    '(?:a4n_db_models::|chapter::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Import reverse blueprint chapter rules expose ORM models" \
    "crates/host/a4n-orchestrator/src/import_reverse/context/blueprint/chapters.rs" \
    '(?:a4n_db_models::|chapter::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Import reverse blueprint extraction rules expose ORM models" \
    "crates/host/a4n-orchestrator/src/import_reverse/context/blueprint/extracted.rs" \
    '(?:a4n_db_models::|chapter::Model|DatabaseTransaction|sea_orm::)'
check_source \
    "Video-studio package exposes transfer ORM models" \
    "crates/host/a4n-orchestrator/src/project/video_studio_package" \
    '(?:generation_spec::Model|prompt_artifact::Model|generated_image::Model|generated_video::Model|media_asset::Model|media_provenance::Model|media_binding::Model|video_segment::Model|assembly_timeline::Model|assembly_clip::Model)'
check_source \
    "Generation task application exposes ORM checkpoint/run/trace models" \
    "crates/host/a4n-orchestrator/src/context/generation" \
    '(?:a4n_db_models::|generation_task_checkpoint::Model|generation_run::Model|llm_call_trace::Model)'
check_source \
    "Workflow application exposes ORM progress event model" \
    "crates/host/a4n-orchestrator/src/context/workflow" \
    '(?:a4n_db_models::models::generation::workflow_progress_event|workflow_progress_event::Model)'
check_source \
    "Blueprint application exposes ORM models" \
    "crates/host/a4n-orchestrator/src/blueprint" \
    '(?:a4n_db_models::|novel_blueprint::Model|blueprint_character::Model|blueprint_relationship::Model)'
check_source \
    "Outline context exposes ORM models" \
    "crates/host/a4n-orchestrator/src/outline/context" \
    '(?:a4n_db_models::|novel_blueprint::Model|blueprint_character::Model|blueprint_relationship::Model|protagonist_profile::Model|object_profile::Model|scene_profile::Model|foreshadowing_index::Model)'
check_source \
    "Chapter generation references expose ORM models" \
    "crates/host/a4n-orchestrator/src/chapter_generation/preparation/reference_context" \
    '(?:a4n_db_models::|novel_blueprint::Model|blueprint_character::Model|blueprint_relationship::Model|protagonist_profile::Model)'
check_source \
    "Chapter generation state context exposes ORM models" \
    "crates/host/a4n-orchestrator/src/chapter_generation/context_before/character_state" \
    '(?:a4n_db_models::|character_state_index::Model|protagonist_profile::Model)'
check_source \
    "Project blueprint owner exposes ORM models" \
    "crates/host/a4n-orchestrator/src/context/project/blueprint.rs" \
    '(?:a4n_db_models::|novel_blueprint::Model|blueprint_character::Model|blueprint_relationship::Model)'
check_source \
    "Project chapter context exposes ORM models" \
    "crates/host/a4n-orchestrator/src/context/project/chapter_context.rs" \
    '(?:a4n_db_models::|protagonist_profile::Model|character_state_index::Model)'
check_source \
    "Video prompt project context exposes ORM blueprint" \
    "crates/host/a4n-orchestrator/src/video/prompts/context/project_context.rs" \
    '(?:a4n_db_models::|novel_blueprint::Model)'
check_source \
    "Optimize Host exposes ORM message/patch rows" \
    "crates/host/a4n-orchestrator/src/optimize" \
    '(?:a4n_db_models::models::ai::optimization_(?:session_message|patch_set|patch)|optimization_(?:session_message|patch_set|patch)::Model)'
check_source \
    "Optimize context owner exposes ORM message/patch rows" \
    "crates/host/a4n-orchestrator/src/context/optimize.rs" \
    '(?:a4n_db_models::models::ai::optimization_(?:session_message|patch_set|patch)|optimization_(?:session_message|patch_set|patch)::Model)'

# Worker clients are transport adapters. They must depend on worker contracts
# and clients only, never on the database or ORM implementation packages.
for client_directory in "$PROJECT_ROOT"/crates/*/*worker-client; do
    [[ -d "$client_directory" ]] || continue
    relative_directory="${client_directory#"$PROJECT_ROOT/"}"
    check_source \
        "Worker client imports ORM types: $relative_directory" \
        "$relative_directory/src" \
        '(?:a4n_db::|a4n_db_models::|sea_orm::|\b(?:DatabaseConnection|DatabaseTransaction|DbErr)\b)'
    package_name="$(sed -n 's/^name = "\([^"]*\)"/\1/p' "$client_directory/Cargo.toml" | head -1)"
    if [[ -n "$package_name" ]]; then
        check_normal_dependencies \
            "$package_name" \
            "$package_name" \
            '(?:a4n-db|a4n-db-models|sea-orm)'
    fi
done

# Read workers may own HTTP routing and authorization, but their persistence
# calls must cross a4n-db domain records rather than carrying ORM rows.
for read_worker in \
    "crates/config/a4n-config-read-worker" \
    "crates/workflow/a4n-workflow-read-worker" \
    "crates/workflow/a4n-workflow-core"; do
    check_source \
        "Read worker/core imports ORM types: $read_worker" \
        "$read_worker/src" \
        '(?:a4n_db_models::|sea_orm::|\b(?:DatabaseConnection|DatabaseTransaction|DbErr)\b)'
    package_name="$(sed -n 's/^name = "\([^"]*\)"/\1/p' "$PROJECT_ROOT/$read_worker/Cargo.toml" | head -1)"
    if [[ -n "$package_name" ]]; then
        check_normal_dependencies \
            "$package_name" \
            "$package_name" \
            '(?:a4n-db-models|sea-orm)'
    fi
done

# Repository-wide production scan. The remaining ORM names outside a4n-db are
# limited to the session adapter, error conversion, and inline test fixtures.
if hits="$(rg --pcre2 --line-number --with-filename --no-heading \
    --glob '*.rs' \
    --glob '!crates/platform/a4n-db/**' \
    --glob '!crates/platform/a4n-db-models/**' \
    --glob '!crates/platform/a4n-db-session/**' \
    --glob '!crates/platform/a4n-error/**' \
    --glob '!crates/platform/a4n-migration/**' \
    --glob '!crates/platform/a4n-test-support/**' \
    --glob '!**/tests/**' \
    --glob '!**/*test*.rs' \
    --glob '!crates/host/a4n-orchestrator/src/shooting_script/contracts.rs' \
    --glob '!crates/host/a4n-orchestrator/src/project/chapters/records.rs' \
    --glob '!crates/host/a4n-runtime/src/actions/outline_records.rs' \
    --glob '!crates/host/a4n-runtime/src/actions/outline_records/access.rs' \
    --glob '!crates/host/a4n-runtime/src/actions/video_generation/media.rs' \
    --glob '!crates/planning/a4n-outline-worker/src/lib.rs' \
    --glob '!crates/agent/a4n-agent-worker/src/health.rs' \
    '(?:a4n_db_models::|sea_orm::|use[[:space:]]+sea_orm)' crates 2>/dev/null)"; then
    echo "FAIL: production ORM references remain outside adapter/test exceptions:" >&2
    printf '  %s\n' "$hits" >&2
    violations=$((violations + 1))
fi

if ((violations > 0)); then
    echo "ORM boundary failed with $violations violation group(s)." >&2
    exit 1
fi

echo "ORM boundary passed for Agent/Auth applications, Runtime/Orchestrator, Worker Clients, read workers, and migrated Core/Optimize/Project/Provider/Timeline/Publication/Scene Profile/Source Sync/Character/Import Reverse/Galgame/Portrait/Video Studio/Generation/Workflow/Blueprint/Outline slices."
