#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$project_root/target}"
# Keep script-driven builds responsive; callers can override this limit.
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

if [[ "${CARGO_ARTIFACTS_DISABLED:-0}" == "1" ]]; then
    exec cargo "$@"
fi
if [[ -n "${CARGO_ARTIFACTS_BIN:-}" ]]; then
    exec "$CARGO_ARTIFACTS_BIN" run -- "$@"
fi
if command -v cargo-artifacts >/dev/null 2>&1; then
    exec cargo-artifacts run -- "$@"
fi

printf '%s\n' 'A4N: cargo-artifacts 未安装，本次构建不会自动回收缓存；安装 rust-artifact-manager 后即可启用。' >&2
exec cargo "$@"
