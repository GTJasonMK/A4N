#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

"$SCRIPT_DIR/check-core-boundary-contracts.sh"
"$SCRIPT_DIR/check-core-boundary-media-workflows.sh"
"$SCRIPT_DIR/check-core-boundary-process.sh"
"$SCRIPT_DIR/check-core-boundary-records.sh"
"$SCRIPT_DIR/check-core-boundary-application.sh"
"$SCRIPT_DIR/check-core-boundary-host-contracts.sh"
"$SCRIPT_DIR/check-core-boundary-agent.sh"
"$SCRIPT_DIR/check-core-boundary-filesystem.sh"
