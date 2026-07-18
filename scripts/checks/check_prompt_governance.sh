#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

SOURCE_DIRS=(
  "crates/a4n-orchestrator/src"
  "crates/a4n-runtime/src"
  "crates/a4n-tauri/src"
)
FRONTEND_SOURCE_DIRS=(
  "frontend/src"
)

fail=0

check_no_source_match() {
  local label="$1"
  local pattern="$2"
  local matches

  matches="$(rg -n "$pattern" "${SOURCE_DIRS[@]}" --glob '!**/tests/**' --glob '!**/tests.rs' || true)"
  if [[ -n "$matches" ]]; then
    printf 'prompt governance violation: %s\n%s\n\n' "$label" "$matches" >&2
    fail=1
  fi
}

check_no_file_match_before_cfg_tests() {
  local label="$1"
  local file="$2"
  local pattern="$3"
  local matches

  matches="$(
    awk '
      /^[[:space:]]*#\[cfg\(test\)\]/ { stop=1 }
      !stop { print FILENAME ":" FNR ":" $0 }
    ' "$file" | rg "$pattern" || true
  )"
  if [[ -n "$matches" ]]; then
    printf 'prompt governance violation: %s\n%s\n\n' "$label" "$matches" >&2
    fail=1
  fi
}

check_no_sources_match_before_cfg_tests() {
  local label="$1"
  local pattern="$2"
  local matches

  matches="$(
    while IFS= read -r file; do
      awk '
        /^[[:space:]]*#\[cfg\(test\)\]/ { stop=1 }
        !stop { print FILENAME ":" FNR ":" $0 }
      ' "$file"
    done < <(rg --files "${SOURCE_DIRS[@]}" -g '*.rs' --glob '!**/tests/**' --glob '!**/tests.rs') | rg "$pattern" || true
  )"
  if [[ -n "$matches" ]]; then
    printf 'prompt governance violation: %s\n%s\n\n' "$label" "$matches" >&2
    fail=1
  fi
}

frontmatter_name() {
  local file="$1"
  awk '
    NR == 1 && $0 != "---" { exit }
    NR > 1 && $0 == "---" { exit }
    /^name:[[:space:]]*/ {
      value = $0
      sub(/^name:[[:space:]]*/, "", value)
      gsub(/^"|"$/, "", value)
      print value
      exit
    }
  ' "$file"
}

frontmatter_dependencies() {
  local file="$1"
  awk '
    NR == 1 && $0 != "---" { exit }
    NR > 1 && $0 == "---" { exit }
    /^dependencies:[[:space:]]*$/ { in_dependencies = 1; next }
    in_dependencies && /^[[:space:]]+-[[:space:]]*[A-Za-z0-9_]+[[:space:]]*$/ {
      value = $0
      sub(/^[[:space:]]+-[[:space:]]*/, "", value)
      gsub(/[[:space:]]+$/, "", value)
      print value
      next
    }
    in_dependencies && /^[^[:space:]]/ { in_dependencies = 0 }
  ' "$file"
}

registered_prompt_names() {
  awk '
    /^prompts:/ { in_prompts = 1; next }
    /^categories:/ { in_prompts = 0 }
    in_prompts && /^[[:space:]]{2}[A-Za-z0-9_]+:$/ {
      name = $1
      sub(/:$/, "", name)
      print name
    }
  ' prompts/_registry.yaml
}

check_rendered_prompt_names_registered() {
  local registered_names
  local matches
  local missing=0
  registered_names="$(registered_prompt_names)"

  matches="$(
    while IFS= read -r file; do
      awk '
        /^[[:space:]]*#\[cfg\(test\)\]/ { stop=1 }
        !stop { print FILENAME ":" FNR ":" $0 }
      ' "$file"
    done < <(rg --files "${SOURCE_DIRS[@]}" -g '*.rs' --glob '!**/tests/**' --glob '!**/tests.rs') \
      | rg '\.render\("[A-Za-z0-9_]+"' || true
  )"
  while IFS= read -r line; do
    [[ -n "$line" ]] || continue
    name="$(printf '%s\n' "$line" | sed -E 's/.*\.render\("([A-Za-z0-9_]+)".*/\1/')"
    if ! grep -Fxq "$name" <<< "$registered_names"; then
      if [[ "$missing" -eq 0 ]]; then
        printf 'prompt governance violation: Rust prompt_engine.render("...") names must be registered\n' >&2
      fi
      printf '%s\n' "$line" >&2
      missing=1
    fi
  done <<< "$matches"
  if [[ "$missing" -ne 0 ]]; then
    printf '\n' >&2
    fail=1
  fi
}

check_prompt_name_constants_registered() {
  local registered_names
  local matches
  local missing=0
  registered_names="$(registered_prompt_names)"

  matches="$(
    while IFS= read -r file; do
      awk '
        /^[[:space:]]*#\[cfg\(test\)\]/ { stop=1 }
        stop { next }
        /const[[:space:]]+[A-Z0-9_]*(PROMPT_NAME|PROMPT_NAMES|SYSTEM_PROMPT|TEMPLATE)[A-Z0-9_]*[[:space:]]*:/ {
          if ($0 ~ /RETIRED_BACKEND_PROMPT_NAMES/) {
            next
          }
          in_block = 1
        }
        in_block { print FILENAME ":" FNR ":" $0 }
        in_block && /;[[:space:]]*$/ { in_block = 0 }
      ' "$file"
    done < <(rg --files "${SOURCE_DIRS[@]}" -g '*.rs' --glob '!**/tests/**' --glob '!**/tests.rs') || true
  )"
  while IFS= read -r line; do
    [[ -n "$line" ]] || continue
    while IFS= read -r quoted_name; do
      [[ -n "$quoted_name" ]] || continue
      name="${quoted_name%\"}"
      name="${name#\"}"
      if ! grep -Fxq "$name" <<< "$registered_names"; then
        if [[ "$missing" -eq 0 ]]; then
          printf 'prompt governance violation: Rust prompt-name/template constants must reference registered prompts\n' >&2
        fi
        printf '%s\n' "$line" >&2
        missing=1
      fi
    done < <(printf '%s\n' "$line" | grep -oE '"[A-Za-z0-9_]+[_][A-Za-z0-9_]*"' || true)
  done <<< "$matches"
  if [[ "$missing" -ne 0 ]]; then
    printf '\n' >&2
    fail=1
  fi
}

check_no_source_match \
  "business system prompts must live under prompts/ and registry" \
  '你是一位专业的小说场景写手|你是故事一致性检查专家|你是严格的 JSON schema 修正器|你是漫画整页绘图提示词优化工程师|你是视频生成提示词优化工程师|当前项目上下文如下。回答时必须优先尊重|当前任务采用分阶段蓝图生成|阶段指令优先于基础提示词'

check_no_file_match_before_cfg_tests \
  "blueprint stage prompt wrappers must be rendered from prompts/ registry" \
  "crates/a4n-orchestrator/src/blueprint/generation_contract.rs" \
  '请根据以下 JSON 输入执行当前蓝图生成阶段|请生成故事核心定位|请基于故事核心生成完整 world_setting|请基于故事核心和世界观生成主要角色|请基于故事核心、世界观和角色档案生成角色关系网络|请根据以下 JSON 输入优化蓝图'

check_no_source_match \
  "new business system prompts should not be hardcoded directly in ChatMessage content" \
  'content:\s*"(你是|你现在是|当前项目上下文)'

check_no_sources_match_before_cfg_tests \
  "structured JSON user-message leads must use a4n_prompt::request helpers" \
  '请根据以下 JSON 输入|请分析以下章节 JSON|请继续执行内容优化 Agent Loop'

frontend_prompt_matches="$(rg -n '你是|请根据以下 JSON 输入|只返回 output_contract|严格 JSON 对象|不要输出 Markdown' "${FRONTEND_SOURCE_DIRS[@]}" || true)"
if [[ -n "$frontend_prompt_matches" ]]; then
  printf 'prompt governance violation: frontend must not hardcode business model prompts\n%s\n\n' "$frontend_prompt_matches" >&2
  fail=1
fi

check_rendered_prompt_names_registered
check_prompt_name_constants_registered

check_no_file_match_before_cfg_tests \
  "video provider prompts must be rendered from prompts/ registry" \
  "crates/a4n-runtime/src/actions/video_generation.rs" \
  '请根据以下导演提示生成一段连续视频|上传的尾帧图片定义视频结尾目标|首帧目标：本次起步输入图片'

check_no_file_match_before_cfg_tests \
  "video prompt optimization prompts must be rendered from prompts/ registry" \
  "crates/a4n-runtime/src/actions/video_prompts.rs" \
  '你是视频生成提示词优化工程师|请根据用户要求优化一个现有视频文字资产'

check_no_file_match_before_cfg_tests \
  "video builder prompt rules must be rendered from prompts/ registry" \
  "crates/a4n-orchestrator/src/video/prompt_context/builders/planning.rs" \
  '按 duration_secs 评估每个片段的动作容量|如果原文包含自伤|请根据以下 JSON 输入生成视频提示词|只返回 output_contract 要求的严格 JSON 对象|frame_prompt 是会直接发给图片模型'

check_no_file_match_before_cfg_tests \
  "portrait generation user prompt must be rendered from prompts/ registry" \
  "crates/a4n-orchestrator/src/portrait/prompt_generation/request.rs" \
  '请根据以下 JSON 输入生成可直接用于图像生成模型的中文人物三视图|只返回 prompt 文本，不要输出解释'

prompt_code_fences="$(rg -n '```' prompts -g '*.md' || true)"
if [[ -n "$prompt_code_fences" ]]; then
  printf 'prompt governance violation: prompt templates must not contain Markdown code fences\n%s\n\n' "$prompt_code_fences" >&2
  fail=1
fi

prompt_json_comments="$(rg -n '//' prompts -g '*.md' || true)"
if [[ -n "$prompt_json_comments" ]]; then
  printf 'prompt governance violation: prompt templates must not contain JSON-style comments\n%s\n\n' "$prompt_json_comments" >&2
  fail=1
fi

while IFS= read -r prompt_file; do
  prompt_name="$(frontmatter_name "$prompt_file")"
  if [[ -z "$prompt_name" ]]; then
    printf 'prompt governance violation: missing frontmatter name in %s\n' "$prompt_file" >&2
    fail=1
    continue
  fi
  if ! rg -q "^[[:space:]]{2}${prompt_name}:" prompts/_registry.yaml; then
    printf 'prompt governance violation: missing registry entry for %s (%s)\n' "$prompt_name" "$prompt_file" >&2
    fail=1
  fi
  while IFS= read -r dependency_name; do
    [[ -n "$dependency_name" ]] || continue
    if ! rg -q "^[[:space:]]{2}${dependency_name}:" prompts/_registry.yaml; then
      printf 'prompt governance violation: %s dependency %s is not registered (%s)\n' "$prompt_name" "$dependency_name" "$prompt_file" >&2
      fail=1
    fi
  done < <(frontmatter_dependencies "$prompt_file")
done < <(rg --files prompts -g '*.md' --glob '!prompts/_partials/**' | sort)

while IFS=$'\t' read -r prompt_name prompt_path; do
  [[ -n "$prompt_name" && -n "$prompt_path" ]] || continue
  prompt_file="prompts/${prompt_path}"
  if [[ ! -f "$prompt_file" ]]; then
    printf 'prompt governance violation: registry entry %s points to missing file %s\n' "$prompt_name" "$prompt_file" >&2
    fail=1
    continue
  fi
  actual_name="$(frontmatter_name "$prompt_file")"
  if [[ "$actual_name" != "$prompt_name" ]]; then
    printf 'prompt governance violation: registry entry %s path %s has frontmatter name %s\n' "$prompt_name" "$prompt_file" "${actual_name:-<missing>}" >&2
    fail=1
  fi
done < <(
  awk '
    /^prompts:/ { in_prompts = 1; next }
    /^categories:/ { in_prompts = 0 }
    in_prompts && /^[[:space:]]{2}[A-Za-z0-9_]+:$/ {
      name = $1
      sub(/:$/, "", name)
    }
    in_prompts && /^[[:space:]]{4}path:[[:space:]]*/ {
      path = $0
      sub(/^[[:space:]]*path:[[:space:]]*/, "", path)
      gsub(/"/, "", path)
      print name "\t" path
    }
  ' prompts/_registry.yaml
)

if ! "$SCRIPT_DIR/check-structured-llm-repair-coverage.sh"; then
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  exit "$fail"
fi

contract_count="$(rg -n '请根据以下 JSON 输入|output_contract|decision_contract' "${SOURCE_DIRS[@]}" --glob '!**/tests/**' | wc -l | tr -d ' ')"
printf 'prompt governance check passed. Structured request contract occurrences kept in code: %s\n' "$contract_count"
