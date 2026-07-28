#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SNAPSHOT="scripts/checks/a4n-core-public-types.tsv"
CATEGORIES="scripts/checks/a4n-core-public-type-categories.tsv"

[[ -f "$SNAPSHOT" && -f "$CATEGORIES" ]] || {
    printf 'a4n-core public type inventory inputs are missing\n' >&2
    exit 1
}

mapfile -t actual < <(
    rg -n '^[[:space:]]*pub (struct|enum|type|trait)\b' crates/a4n-core/src --glob '*.rs' |
        sed -E 's#^crates/a4n-core/src/(.*):[0-9]+:[[:space:]]*pub (struct|enum|type|trait) ([A-Za-z_][A-Za-z0-9_]*).*#\1\t\2\t\3#' |
        sort
)
mapfile -t expected < <(sed -E '/^[[:space:]]*(#|$)/d' "$SNAPSHOT" | sort)

if [[ "${actual[*]}" != "${expected[*]}" ]]; then
    printf 'a4n-core public type inventory drifted; update the snapshot and architecture classification together\n' >&2
    diff -u <(printf '%s\n' "${expected[@]}") <(printf '%s\n' "${actual[@]}") || true
    exit 1
fi

while IFS=$'\t' read -r path kind name; do
    [[ -n "$path" ]] || continue
    category="$({ awk -F '\t' -v path="$path" '
        $0 !~ /^[[:space:]]*#/ && $1 != "" && index(path, $1) == 1 { print $2; exit }
    ' "$CATEGORIES"; } || true)"
    if [[ -z "$category" ]]; then
        printf 'unclassified a4n-core public type: %s (%s %s)\n' "$path" "$kind" "$name" >&2
        exit 1
    fi
done < <(printf '%s\n' "${actual[@]}")

printf 'a4n-core public type inventory and category allowlist are valid (%s declarations)\n' "${#actual[@]}"
