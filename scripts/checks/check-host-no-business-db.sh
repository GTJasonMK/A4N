#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

for command in cargo rg sed; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "Required command not found: $command" >&2
        exit 127
    fi
done

packages=(
    a4n-control
    a4n-host-assembly
    a4n-server
    a4n-edge
    a4n-tauri
    a4n-runtime
)
violations=0

tree_has_package() {
    local tree="$1"
    local package="$2"
    rg -q --pcre2 "(^|[[:space:]])${package} v[0-9]" <<<"$tree"
}

source_hits() {
    local source_dir="$1"
    local pattern="$2"
    shift 2
    local output status match file remainder line_number previous filtered=""
    local -a rg_args=(
        --pcre2
        --line-number
        --with-filename
        --no-heading
        --glob '*.rs'
        --glob '!**/tests/**'
        --glob '!**/*test*.rs'
    )
    rg_args+=("$@")

    if output="$(rg "${rg_args[@]}" "$pattern" "$source_dir" 2>&1)"; then
        status=0
    else
        status=$?
    fi
    if ((status > 1)); then
        echo "Source scan failed in $source_dir:" >&2
        echo "$output" >&2
        return 2
    fi
    ((status == 0)) || return 0

    while IFS= read -r match; do
        file="${match%%:*}"
        remainder="${match#*:}"
        line_number="${remainder%%:*}"
        if [[ "$line_number" =~ ^[0-9]+$ ]] && ((line_number > 1)); then
            previous="$(sed -n "$((line_number - 1))p" "$file")"
            if [[ "$previous" =~ ^[[:space:]]*#\[cfg\(test\)\][[:space:]]*$ ]]; then
                continue
            fi
        fi
        [[ "$match" == *'#[cfg(test)]'* ]] && continue
        filtered+="${match}"$'\n'
    done <<<"$output"

    printf '%s' "$filtered"
}

check_source_pattern() {
    local package="$1"
    local source_dir="$2"
    local label="$3"
    local pattern="$4"
    shift 4
    local hits

    if ! hits="$(source_hits "$source_dir" "$pattern" "$@")"; then
        violations=$((violations + 1))
        return
    fi
    if [[ -n "$hits" ]]; then
        echo "FAIL: $package source contains forbidden $label reference(s):" >&2
        while IFS= read -r hit; do
            [[ -n "$hit" ]] && echo "  $hit" >&2
        done <<<"$hits"
        violations=$((violations + 1))
    fi
}

for package in "${packages[@]}"; do
    package_dir="$PROJECT_ROOT/crates/host/$package"
    source_dir="$package_dir/src"
    if [[ ! -f "$package_dir/Cargo.toml" || ! -d "$source_dir" ]]; then
        echo "Required Host crate source is missing: $package_dir" >&2
        exit 2
    fi

    # Composition roots may include business-owner crates whose normal closure
    # uses the database. This gate rejects direct Host DB dependencies; source
    # scans below reject Host code reaching across that owner boundary.
    if ! tree="$(cargo tree --locked --offline --edges normal --depth 1 -p "$package" 2>&1)"; then
        echo "Unable to inspect direct normal dependencies for $package (offline):" >&2
        echo "$tree" >&2
        exit 2
    fi

    if tree_has_package "$tree" a4n-db; then
        echo "FAIL: $package direct normal dependencies contain business repository crate a4n-db" >&2
        violations=$((violations + 1))
    fi

    if tree_has_package "$tree" a4n-db-models; then
        models_allowed=false
        case "$package" in
            a4n-tauri)
                if tree_has_package "$tree" a4n-runtime \
                    && tree_has_package "$tree" a4n-migration; then
                    models_allowed=true
                fi
                ;;
        esac
        if [[ "$models_allowed" != true ]]; then
            echo "FAIL: $package direct normal dependencies contain ORM model crate a4n-db-models" >&2
            violations=$((violations + 1))
        fi
    fi

    if [[ "$package" == a4n-runtime ]]; then
        check_source_pattern "$package" "$source_dir" "a4n-db repository" \
            '(?<![[:alnum:]_])a4n_db::'
        check_source_pattern "$package" "$source_dir" "ORM model" \
            '(?<![[:alnum:]_])a4n_db_models::'
        check_source_pattern "$package" "$source_dir" "SeaORM/migration crate" \
            '(?<![[:alnum:]_])(?:sea_orm|sea_orm_migration|a4n_migration)::'
    else
        check_source_pattern "$package" "$source_dir" "business database/ORM" \
            '(?<![[:alnum:]_])(?:a4n_db|a4n_db_models|sea_orm|sea_orm_migration|a4n_migration)::'
    fi
done

if ((violations > 0)); then
    echo "Host database boundary failed with $violations violation group(s)." >&2
    exit 1
fi

echo "Host database boundary passed for all six Host crates (normal dependencies only)."
