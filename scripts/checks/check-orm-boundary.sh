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

# Core code may depend on repository contracts, but it must not expose the
# concrete SeaORM transaction aliases used by the adapters.
check_source \
    "chapter-content core exposes a concrete ORM transaction" \
    "crates/chapter/a4n-chapter-content-core/src" \
    '(?<![[:alnum:]_])(?:DbTransaction|DatabaseTransaction|a4n_db_models::)'

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
for discovery_file in \
    contracts.rs execution.rs request.rs output.rs rules.rs workflow.rs; do
    check_source \
        "Character discovery pure owner exposes ORM models: $discovery_file" \
        "crates/host/a4n-orchestrator/src/character/discovery/$discovery_file" \
        '(?:a4n_db_models::|(?:chapter|text_anchor|protagonist_profile)::Model|DatabaseTransaction|sea_orm::)'
done
check_source \
    "Video-studio package exposes transfer ORM models" \
    "crates/host/a4n-orchestrator/src/project/video_studio_package" \
    '(?:generation_spec::Model|prompt_artifact::Model|generated_image::Model|generated_video::Model|media_asset::Model|media_provenance::Model|media_binding::Model|video_segment::Model|assembly_timeline::Model|assembly_clip::Model)'

if ((violations > 0)); then
    echo "ORM boundary failed with $violations violation group(s)." >&2
    exit 1
fi

echo "ORM boundary passed for migrated Core, Optimize, Project Family, Provider, Timeline, Publication, Scene Profile, Source Sync, Galgame, Portrait, and Video Studio slices."
