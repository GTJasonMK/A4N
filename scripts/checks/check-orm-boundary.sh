#!/usr/bin/env bash
set -euo pipefail

failed=0

is_exempt_source() {
    case "$1" in
        crates/platform/a4n-db/*|crates/platform/a4n-db-models/*|crates/platform/a4n-db-session/*|crates/platform/a4n-migration/*)
            return 0
            ;;
        */tests/*|*/test/*|*/tests.rs|*/test.rs|*/test_support.rs|*_tests.rs)
            return 0
            ;;
    esac
    return 1
}

while IFS= read -r manifest; do
    case "$manifest" in
        crates/platform/a4n-db/Cargo.toml|crates/platform/a4n-db-models/Cargo.toml|crates/platform/a4n-db-session/Cargo.toml|crates/platform/a4n-migration/Cargo.toml)
            continue
            ;;
    esac

    if awk -v manifest="$manifest" '
        /^\[/ {
            production = ($0 == "[dependencies]" || $0 ~ /\.dependencies\]$/)
        }
        production && /^[[:space:]]*sea-orm(-migration)?[[:space:]]*=/ {
            printf "%s:%d: SeaORM production dependency is restricted to database crates\n", manifest, NR > "/dev/stderr"
            failed = 1
        }
        END { exit failed }
    ' "$manifest"; then
        :
    else
        failed=1
    fi
done < <(rg --files crates -g 'Cargo.toml')

while IFS= read -r -d '' source; do
    if is_exempt_source "$source"; then
        continue
    fi
    if rg -n -e '(^|[[:space:]])(pub[[:space:]]+)?use[[:space:]]+sea_orm' \
        -e '(^|[^[:alnum:]_])sea_orm::' "$source"; then
        printf '%s: direct SeaORM source usage is restricted to database crates and tests\n' \
            "$source" >&2
        failed=1
    fi
done < <(rg --files -0 crates -g '*.rs')

if (( failed != 0 )); then
    exit 1
fi

printf 'SeaORM production dependencies are restricted to database crates\n'
