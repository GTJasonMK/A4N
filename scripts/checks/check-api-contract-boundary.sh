#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CONTRACT_DIR="crates/a4n-api-contract"
RUNTIME_ACTIONS="crates/a4n-runtime/src/actions"
FRONTEND_API="frontend/src/api"
DTO_INVENTORY="scripts/checks/host-api-dto-inventory.tsv"

[[ -f "$CONTRACT_DIR/Cargo.toml" && -f "$CONTRACT_DIR/src/lib.rs" ]] || {
    printf 'a4n-api-contract is missing\n' >&2
    exit 1
}

if rg -n 'sea-orm|axum|tauri|reqwest|a4n-(core|db|runtime|orchestrator|capabilities|service-manager)' \
    "$CONTRACT_DIR/Cargo.toml" "$CONTRACT_DIR/src"; then
    printf 'a4n-api-contract must stay independent from DB, Host, provider, and sidecar crates\n' >&2
    exit 1
fi

mapfile -t contract_modules < <(
    sed -nE 's/^pub mod ([a-z0-9_]+);/\1/p' "$CONTRACT_DIR/src/lib.rs"
)
for module in "${contract_modules[@]}"; do
    # Generation DTOs land before runtime's generation Host API in IG-CORE-01.
    # The command catalog is consumed by the server metadata route and CLI,
    # not by runtime action handlers.
    [[ "$module" == "generation_tasks" || "$module" == "command_manifest" ]] && continue
    if ! rg -q "a4n_api_contract::${module}[:{]" "$RUNTIME_ACTIONS"; then
        printf 'runtime does not consume canonical contract module: %s\n' "$module" >&2
        exit 1
    fi
done

mapfile -t contract_types < <(
    rg --no-filename --only-matching --replace '$1' \
        '^pub (?:struct|enum|type) ([A-Za-z][A-Za-z0-9_]*)' \
        "$CONTRACT_DIR/src" -g '*.rs' | sort -u
)
for type_name in "${contract_types[@]}"; do
    if rg -n --glob '*.rs' \
        "^pub (struct|enum|type) ${type_name}([[:space:]<{=(;]|$)" \
        "$RUNTIME_ACTIONS"; then
        printf 'runtime redefines canonical contract type: %s\n' "$type_name" >&2
        exit 1
    fi
    if rg -n --glob '*.ts' --glob '*.tsx' --glob '!frontend/src/api/generated/**' \
        "(^|[[:space:]])(export[[:space:]]+)?interface[[:space:]]+${type_name}([[:space:]<{]|$)" \
        "$FRONTEND_API"; then
        printf 'frontend redefines canonical contract interface: %s\n' "$type_name" >&2
        exit 1
    fi
done

node scripts/checks/generate-host-api-dto-inventory.mjs --check
if awk -F '\t' '
    NR > 2 && $4 == "public_host_contract" && ($7 == "yes" || $10 != "") {
        print
        found = 1
    }
    END { exit found ? 0 : 1 }
' "$DTO_INVENTORY"; then
    printf 'runtime still owns frontend-visible or duplicated Host DTOs\n' >&2
    exit 1
fi

cargo run -q -p a4n-api-contract --bin generate_types -- --check
bash scripts/checks/check-generation-contracts.sh

printf 'a4n-api-contract dependency, ownership, inventory, and generated-type boundaries are valid\n'
