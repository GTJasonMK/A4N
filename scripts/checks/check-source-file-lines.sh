#!/usr/bin/env bash
set -euo pipefail

max_lines=1200
failed=0

while IFS= read -r -d '' path; do
    case "$path" in
        frontend/src/api/generated/*|frontend/e2e/*|scripts/tests/*|*/tests/*|*/test/*|*/tests.rs|*/test.rs|*.test.ts|*.test.tsx|*.test.js|*.test.jsx|*.test.mjs)
            continue
            ;;
    esac

    [[ -f "$path" ]] || continue

    lines="$(wc -l < "$path")"
    if (( lines > max_lines )); then
        printf 'source file exceeds %d lines: %d %s\n' "$max_lines" "$lines" "$path" >&2
        failed=1
    fi
done < <(
    git ls-files -z -- \
        '*.rs' '*.ts' '*.tsx' '*.css' '*.py' '*.sh' '*.mjs' '*.jsx' '*.vue'
)

if (( failed != 0 )); then
    exit 1
fi

printf 'all tracked hand-written source files are at most %d lines\n' "$max_lines"
