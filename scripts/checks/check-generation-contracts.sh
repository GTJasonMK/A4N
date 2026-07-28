#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

bash scripts/contracts/generate-generation-contracts.sh --check
node scripts/contracts/check-generation-fixtures.mjs

printf 'generation schema, TypeScript output, and fixtures are current\n'
