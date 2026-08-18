#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

if rg -n 'a4n[-_](runtime|orchestrator|services|service_protocol|capabilities|service_manager|prompt_service|llm_gateway)' crates/a4n-core/Cargo.toml crates/a4n-core/src; then
    printf 'a4n-core must not depend on host, orchestrator, service, or sidecar crates\n' >&2
    exit 1
fi

printf 'a4n-core dependency boundary is valid\n'
