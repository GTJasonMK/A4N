#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ALLOWLIST="scripts/checks/a4n-core-public-modules.txt"
LIB="crates/a4n-core/src/lib.rs"

[[ -f "$ALLOWLIST" && -f "$LIB" ]] || {
    printf 'a4n-core boundary inventory inputs are missing\n' >&2
    exit 1
}

mapfile -t actual < <(sed -nE 's/^pub mod ([a-zA-Z0-9_]+);/\1/p' "$LIB" | sort)
mapfile -t expected < <(sed -E '/^[[:space:]]*(#|$)/d' "$ALLOWLIST" | sort -u)

if [[ "${actual[*]}" != "${expected[*]}" ]]; then
    printf 'a4n-core public module inventory drifted; update the explicit allowlist and architecture plan together\n' >&2
    diff -u <(printf '%s\n' "${expected[@]}") <(printf '%s\n' "${actual[@]}") || true
    exit 1
fi

if rg -n 'a4n_(runtime|orchestrator|services|service_protocol|capabilities|service_manager|prompt_service|llm_gateway)' crates/a4n-core/Cargo.toml crates/a4n-core/src; then
    printf 'a4n-core must not depend on host, orchestrator, service, or sidecar crates\n' >&2
    exit 1
fi

"$ROOT/scripts/checks/check-core-public-types.sh"

printf 'a4n-core public module inventory and dependency boundary are valid (%s modules)\n' "${#actual[@]}"
