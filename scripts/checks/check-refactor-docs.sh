#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

PLAN="docs/重构/模块化重构执行计划-v2.md"
PROGRESS="docs/重构/模块化重构进度记录.md"
WORKPACK_DIR="docs/重构/剩余重构"
WORKPACK_README="$WORKPACK_DIR/README.md"
BASE_REF="${A4N_REFACTOR_BASE:-${1:-}}"

require_committed() {
    local file="$1"
    git ls-files --error-unmatch "$file" >/dev/null 2>&1 || {
        printf 'refactor document is not tracked by git: %s\n' "$file" >&2
        exit 1
    }
    git cat-file -e "HEAD:$file" 2>/dev/null || {
        printf 'refactor document is staged but not present in commit history: %s\n' "$file" >&2
        exit 1
    }
}

for file in "$PLAN" "$PROGRESS" "$WORKPACK_README"; do
    [[ -f "$file" ]] || {
        printf 'missing refactor status document: %s\n' "$file" >&2
        exit 1
    }
done

mapfile -t workpack_docs < <(find "$WORKPACK_DIR" -maxdepth 1 -type f \( -name '*.md' -o -name '*.tsv' \) -print | sort)
if [[ "${#workpack_docs[@]}" -eq 0 ]]; then
    printf 'missing remaining refactor work-package documents under %s\n' "$WORKPACK_DIR" >&2
    exit 1
fi

# 当前状态源只允许使用这两个值；历史说明中的自然语言不参与匹配。
if rg -n '状态：`(?!完成|未完成)`' "$PLAN" "$PROGRESS" --pcre2; then
    printf 'refactor status document contains an invalid status value\n' >&2
    exit 1
fi

for section in 3.10 3.12 3.13 3.14 3.15 3.16 3.17 3.18 3.19 3.20 3.21 3.22 3.23 3.24 3.25 3.26 3.27 3.28 3.29; do
    rg -q "^### ${section//./\\.} " "$PLAN" || {
        printf 'missing required plan section: %s\n' "$section" >&2
        exit 1
    }
done

git check-ignore -q "$PLAN" && {
    printf 'refactor plan is ignored by git: %s\n' "$PLAN" >&2
    exit 1
}
git check-ignore -q "$PROGRESS" && {
    printf 'refactor progress is ignored by git: %s\n' "$PROGRESS" >&2
    exit 1
}

for file in "$PLAN" "$PROGRESS"; do
    require_committed "$file"
done

for file in "${workpack_docs[@]}"; do
    git check-ignore -q "$file" && {
        printf 'refactor work-package document is ignored by git: %s\n' "$file" >&2
        exit 1
    }
    require_committed "$file"
done

rg -q --fixed-strings "当前执行依据：\`$PLAN\`" "$PROGRESS" || {
    printf 'refactor progress must name the V2 plan as its current source\n' >&2
    exit 1
}

if rg -n '\]\([^)]*(模块化重构执行计划|模块化重构未完成事项清单)\.md' \
    "$PLAN" "$PROGRESS" "$WORKPACK_README" "${workpack_docs[@]}"; then
    printf 'active refactor documents link to a historical plan\n' >&2
    exit 1
fi

if [[ -n "$BASE_REF" ]]; then
    BASE_COMMIT="$(git rev-parse --verify --end-of-options "${BASE_REF}^{commit}" 2>/dev/null)" || {
        printf 'refactor base commit is unavailable: %s\n' "$BASE_REF" >&2
        exit 1
    }
    changed_output="$(git -c core.quotePath=false diff --no-renames --name-only "${BASE_COMMIT}...HEAD" --)" || {
        printf 'unable to diff refactor base against HEAD: %s\n' "$BASE_COMMIT" >&2
        exit 1
    }
    mapfile -t changed_files < <(printf '%s' "$changed_output")
    architecture_changed=false
    status_doc_changed=false
    for file in "${changed_files[@]}"; do
        case "$file" in
            "$PLAN"|"$PROGRESS") status_doc_changed=true ;;
        esac
        case "$file" in
            Cargo.toml|Cargo.lock|crates/*/Cargo.toml|.github/workflows/*|crates/a4n-api-contract/*|crates/a4n-core/*|crates/a4n-domain/*|crates/a4n-runtime/*|crates/a4n-orchestrator/*|crates/a4n-service-protocol/*|crates/a4n-capabilities/*|crates/a4n-service-manager/*|scripts/checks/*)
                architecture_changed=true
                ;;
        esac
    done
    if [[ "$architecture_changed" == true && "$status_doc_changed" != true ]]; then
        printf 'architecture boundary changed without updating the V2 plan or progress record\n' >&2
        exit 1
    fi
fi

printf 'refactor status documents are present, tracked, and use valid status values\n'
