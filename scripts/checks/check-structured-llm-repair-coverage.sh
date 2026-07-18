#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

expected_request_files="$({
  printf '%s\n' \
    crates/a4n-orchestrator/src/chapter_analysis/owner/requests/context.rs \
    crates/a4n-orchestrator/src/galgame/pipeline_run/llm/once/execution.rs \
    crates/a4n-orchestrator/src/galgame/pipeline_run/llm/stage/execution.rs \
    crates/a4n-orchestrator/src/import_reverse/request.rs \
    crates/a4n-orchestrator/src/manga/pipeline_prompt/llm/request.rs \
    crates/a4n-orchestrator/src/optimize/agent_loop/request/envelope.rs \
    crates/a4n-orchestrator/src/optimize/patch_request.rs \
    crates/a4n-orchestrator/src/outline/chapters/requests/request.rs \
    crates/a4n-orchestrator/src/outline/parts/requests/request.rs \
    crates/a4n-orchestrator/src/shooting_script/context.rs \
    crates/a4n-orchestrator/src/structured_llm/schema_repair/request.rs \
    crates/a4n-orchestrator/src/video/prompt_generation/llm_chat/request.rs \
    crates/a4n-prompt/src/blueprint_generation/refine/builder.rs \
    crates/a4n-prompt/src/blueprint_generation/stage_request/builder.rs \
    crates/a4n-runtime/src/actions/galgame_records/pipeline/preview/request.rs \
    crates/a4n-runtime/src/actions/manga_records/optimization/request.rs \
    crates/a4n-runtime/src/actions/video_prompts/optimization/request/builder.rs
} | sort)"

test_only_request_file="crates/a4n-orchestrator/src/blueprint/generation/checkpoint/fingerprint.rs"
test_module_line="$(rg -n '^#\[cfg\(test\)\]$' "$test_only_request_file" | tail -n 1 | cut -d: -f1)"
if [[ -z "$test_module_line" ]]; then
  printf 'structured LLM repair coverage violation: %s is no longer test-only.\n' \
    "$test_only_request_file" >&2
  exit 1
fi
while IFS=: read -r request_line _; do
  if (( request_line <= test_module_line )); then
    printf 'structured LLM repair coverage violation: %s contains a production JSON-mode request.\n' \
      "$test_only_request_file" >&2
    exit 1
  fi
done < <(rg -n 'with_json_response_format\(' "$test_only_request_file")

actual_request_files="$({
  rg -l 'with_json_response_format\(' \
    crates/a4n-orchestrator/src \
    crates/a4n-prompt/src \
    crates/a4n-runtime/src \
    -g '*.rs' |
    while IFS= read -r file; do
      if [[ "$file" != "$test_only_request_file" ]]; then
        printf '%s\n' "$file"
      fi
    done
} | sort)"
if [[ "$actual_request_files" != "$expected_request_files" ]]; then
  printf 'structured LLM repair coverage violation: JSON-mode request inventory changed.\n' >&2
  diff -u <(printf '%s\n' "$expected_request_files") <(printf '%s\n' "$actual_request_files") >&2 || true
  printf 'Route every new JSON-mode LLM call through structured_llm::schema_repair, then update this audited inventory.\n' >&2
  exit 1
fi

required_execution_files=(
  crates/a4n-orchestrator/src/blueprint/generation/stage.rs
  crates/a4n-orchestrator/src/blueprint/refine/execution.rs
  crates/a4n-orchestrator/src/chapter_analysis/stages/runner.rs
  crates/a4n-orchestrator/src/galgame/pipeline_run/llm/once/execution.rs
  crates/a4n-orchestrator/src/galgame/pipeline_run/llm/stage/execution.rs
  crates/a4n-orchestrator/src/import_reverse/execution.rs
  crates/a4n-orchestrator/src/manga/pipeline_prompt/llm/execution.rs
  crates/a4n-orchestrator/src/optimize/agent_loop_executor/decision/execution.rs
  crates/a4n-orchestrator/src/optimize/patch_generation/execution/call.rs
  crates/a4n-orchestrator/src/outline/chapters/llm/execution.rs
  crates/a4n-orchestrator/src/outline/parts/llm/execution.rs
  crates/a4n-orchestrator/src/shooting_script/stage.rs
  crates/a4n-orchestrator/src/video/prompt_generation/frame_prompts/render_restore.rs
  crates/a4n-runtime/src/actions/manga_records/optimization/execution/call.rs
  crates/a4n-runtime/src/actions/video_prompts/optimization/api/call.rs
)

for file in "${required_execution_files[@]}"; do
  if ! rg -q 'parse_validate_or_repair_structured_(output|value)' "$file"; then
    printf 'structured LLM repair coverage violation: %s bypasses the shared repair executor.\n' "$file" >&2
    exit 1
  fi
done

printf 'structured LLM repair coverage check passed: %s production JSON-mode request builders audited.\n' \
  "$(printf '%s\n' "$actual_request_files" | wc -l | tr -d ' ')"
