#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

bash scripts/contracts/generate-generation-contracts.sh --check
node scripts/contracts/check-generation-fixtures.mjs

printf 'generation schema, TypeScript output, and fixtures are current\n'
