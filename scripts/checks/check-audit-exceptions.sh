#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

config="$ROOT/.cargo/audit.toml"
[[ -r "$config" ]] || {
    echo "missing cargo-audit exception config: $config" >&2
    exit 1
}

ignore="$(sed -nE 's/^[[:space:]]*ignore[[:space:]]*=[[:space:]]*(.*)$/\1/p' "$config")"
[[ "$ignore" == '["RUSTSEC-2023-0071"]' ]] || {
    echo "cargo-audit exceptions must contain only RUSTSEC-2023-0071" >&2
    exit 1
}

rg -Uq 'name = "rsa"\nversion = "0\.9\.10"' Cargo.lock || {
    echo "RSA advisory exception is stale; remove it or review the new locked version" >&2
    exit 1
}

active_tree="$(cargo tree --workspace --locked --all-features --target all --prefix none)"
if rg -q '^rsa v' <<<"$active_tree"; then
    echo "RUSTSEC-2023-0071 exception is invalid: RSA is active in the all-features dependency graph" >&2
    exit 1
fi

echo "cargo-audit exception is scoped to inactive sqlx-mysql RSA"
