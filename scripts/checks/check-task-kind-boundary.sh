#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

canonical="crates/a4n-core/src/event/channel/contracts.rs"
[[ -f "$canonical" ]] || {
    printf 'canonical TaskKind owner is missing: %s\n' "$canonical" >&2
    exit 1
}

owners=$(rg -l '^pub enum TaskKind' crates --glob '*.rs' || true)
[[ "$owners" == "$canonical" ]] || {
    printf 'TaskKind must have exactly one enum owner in a4n-core\n' >&2
    printf '%s\n' "$owners" >&2
    exit 1
}

if rg -n '\bTaskType\b' \
    crates/a4n-core crates/a4n-queue crates/a4n-runtime crates/a4n-server crates/a4n-tauri \
    --glob '*.rs'; then
    printf 'legacy TaskType references remain in production crates\n' >&2
    exit 1
fi

rg -q '^pub use a4n_core::event::channel::TaskKind;' \
    crates/a4n-queue/src/queue/priority/kinds.rs || {
    printf 'a4n-queue must re-export the core TaskKind\n' >&2
    exit 1
}

printf 'canonical TaskKind boundary is valid\n'
